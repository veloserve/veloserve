use std::net::SocketAddr;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, StatusCode};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio::time::sleep;

struct TestServer {
    addr: SocketAddr,
    _docroot: TempDir,
    _config_dir: TempDir,
    child: Child,
}

impl TestServer {
    async fn start() -> Result<Self> {
        let docroot = tempfile::tempdir().context("create temp docroot")?;
        std::fs::create_dir_all(docroot.path().join("catalog")).context("create catalog dir")?;
        std::fs::write(docroot.path().join("catalog").join("a.html"), "<h1>A</h1>")
            .context("write a.html")?;
        std::fs::write(docroot.path().join("catalog").join("b.html"), "<h1>B</h1>")
            .context("write b.html")?;

        let addr = reserve_local_addr().context("reserve local port")?;
        let config_dir = tempfile::tempdir().context("create temp config dir")?;
        let config_path = config_dir.path().join("veloserve.toml");
        let config_toml = format!(
            "[server]\nlisten = \"{}\"\n\n[php]\nenable = false\n\n[cache]\nenable = true\nl1_enabled = true\nl2_enabled = false\ndefault_ttl = 3600\n\n[[virtualhost]]\ndomain = \"*\"\nroot = \"{}\"\nindex = [\"index.html\"]\n",
            addr,
            docroot.path().to_string_lossy()
        );
        std::fs::write(&config_path, config_toml).context("write config file")?;

        let child = Command::new(env!("CARGO_BIN_EXE_veloserve"))
            .arg("--config")
            .arg(&config_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("start veloserve child process")?;

        wait_until_ready(addr).await?;

        Ok(Self {
            addr,
            _docroot: docroot,
            _config_dir: config_dir,
            child,
        })
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[tokio::test]
async fn magento_style_invalidation_contract_works() -> Result<()> {
    let server = TestServer::start().await?;
    let connector = HttpConnector::new();
    let client: Client<_, Full<Bytes>> = Client::builder(TokioExecutor::new()).build(connector);

    warm_path(&client, server.addr, "/catalog/a.html").await?;
    warm_path(&client, server.addr, "/catalog/b.html").await?;

    let invalidate_url = json!({
        "scope": "url",
        "domain": "example.test",
        "paths": ["/catalog/a.html"]
    });
    let response = post_json(
        &client,
        server.addr,
        "/api/v1/cache/invalidate",
        &invalidate_url,
        &[],
    )
    .await?;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body["scope"], "url");
    assert_eq!(response.body["deduped"], false);
    assert!(response.body["affected_keys"].as_u64().unwrap_or(0) >= 1);

    let a_after_url = get_path(&client, server.addr, "/catalog/a.html").await?;
    assert_eq!(a_after_url.status, StatusCode::OK);
    assert_eq!(a_after_url.cache_header.as_deref(), Some("MISS"));

    let b_after_url = get_path(&client, server.addr, "/catalog/b.html").await?;
    assert_eq!(b_after_url.status, StatusCode::OK);
    assert_eq!(b_after_url.cache_header.as_deref(), Some("HIT"));

    let invalidate_tag = json!({
        "scope": "tag",
        "tags": ["path:example.test/catalog/b.html"]
    });
    let response = post_json(
        &client,
        server.addr,
        "/api/v1/cache/invalidate",
        &invalidate_tag,
        &[],
    )
    .await?;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body["scope"], "tag");
    assert!(response.body["affected_keys"].as_u64().unwrap_or(0) >= 1);

    let b_after_tag = get_path(&client, server.addr, "/catalog/b.html").await?;
    assert_eq!(b_after_tag.cache_header.as_deref(), Some("MISS"));

    warm_path(&client, server.addr, "/catalog/a.html").await?;
    warm_path(&client, server.addr, "/catalog/b.html").await?;

    let invalidate_group = json!({
        "scope": "tag_group",
        "groups": [{
            "name": "catalog",
            "tags": [
                "path:example.test/catalog/a.html",
                "path:example.test/catalog/b.html"
            ]
        }]
    });
    let response = post_json(
        &client,
        server.addr,
        "/api/v1/cache/invalidate",
        &invalidate_group,
        &[("x-idempotency-key", "magento-group-1")],
    )
    .await?;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body["scope"], "tag_group");

    let response_deduped = post_json(
        &client,
        server.addr,
        "/api/v1/cache/invalidate",
        &invalidate_group,
        &[("x-idempotency-key", "magento-group-1")],
    )
    .await?;
    assert_eq!(response_deduped.status, StatusCode::ACCEPTED);
    assert_eq!(response_deduped.body["deduped"], true);

    let a_after_group = get_path(&client, server.addr, "/catalog/a.html").await?;
    let b_after_group = get_path(&client, server.addr, "/catalog/b.html").await?;
    assert_eq!(a_after_group.cache_header.as_deref(), Some("MISS"));
    assert_eq!(b_after_group.cache_header.as_deref(), Some("MISS"));

    Ok(())
}

struct HttpResult {
    status: StatusCode,
    body: Value,
}

struct PageResult {
    status: StatusCode,
    cache_header: Option<String>,
}

async fn post_json(
    client: &Client<HttpConnector, Full<Bytes>>,
    addr: SocketAddr,
    path: &str,
    payload: &Value,
    headers: &[(&str, &str)],
) -> Result<HttpResult> {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(format!("http://{}{}", addr, path))
        .header("Host", "example.test")
        .header("Content-Type", "application/json")
        .header("x-veloserve-request-id", "test-request");

    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }

    let request = builder
        .body(Full::new(Bytes::from(payload.to_string())))
        .context("build json request")?;
    let response = client
        .request(request)
        .await
        .context("execute json request")?;
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .context("read json response body")?
        .to_bytes();
    let body = serde_json::from_slice(&body).context("parse json response")?;
    Ok(HttpResult { status, body })
}

async fn get_path(
    client: &Client<HttpConnector, Full<Bytes>>,
    addr: SocketAddr,
    path: &str,
) -> Result<PageResult> {
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("http://{}{}", addr, path))
        .header("Host", "example.test")
        .body(Full::new(Bytes::new()))
        .context("build page request")?;
    let response = client
        .request(request)
        .await
        .context("execute page request")?;
    let status = response.status();
    let cache_header = response
        .headers()
        .get("X-Cache")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string());
    let _ = response
        .into_body()
        .collect()
        .await
        .context("read page body")?;
    Ok(PageResult {
        status,
        cache_header,
    })
}

async fn warm_path(
    client: &Client<HttpConnector, Full<Bytes>>,
    addr: SocketAddr,
    path: &str,
) -> Result<()> {
    let first = get_path(client, addr, path).await?;
    assert_eq!(first.status, StatusCode::OK);
    let second = get_path(client, addr, path).await?;
    assert_eq!(second.status, StatusCode::OK);
    assert_eq!(second.cache_header.as_deref(), Some("HIT"));
    Ok(())
}

async fn wait_until_ready(addr: SocketAddr) -> Result<()> {
    let connector = HttpConnector::new();
    let client: Client<_, Full<Bytes>> = Client::builder(TokioExecutor::new()).build(connector);
    let url = format!("http://{}/health", addr);

    for _ in 0..60 {
        let request = Request::builder()
            .method(Method::GET)
            .uri(&url)
            .body(Full::new(Bytes::new()))
            .context("build readiness request")?;

        if let Ok(response) = client.request(request).await {
            if response.status() == StatusCode::OK {
                return Ok(());
            }
        }

        sleep(Duration::from_millis(50)).await;
    }

    Err(anyhow::anyhow!("server did not become ready on {}", addr))
}

fn reserve_local_addr() -> Result<SocketAddr> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").context("bind ephemeral socket")?;
    let addr = listener.local_addr().context("read local addr")?;
    drop(listener);
    Ok(addr)
}
