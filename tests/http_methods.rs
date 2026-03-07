use std::net::SocketAddr;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::{Method, Request, StatusCode};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
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
        std::fs::write(
            docroot.path().join("index.html"),
            "<h1>Hello from VeloServe</h1>",
        )
        .context("write index.html")?;

        let addr = reserve_local_addr().context("reserve local port")?;

        let config_dir = tempfile::tempdir().context("create temp config dir")?;
        let config_path = config_dir.path().join("veloserve.toml");
        let config_toml = format!(
            "[server]\nlisten = \"{}\"\n\n[php]\nenable = false\n\n[[virtualhost]]\ndomain = \"*\"\nroot = \"{}\"\nindex = [\"index.html\"]\n",
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
async fn supports_common_http_methods() -> Result<()> {
    let server = TestServer::start().await?;

    let connector = HttpConnector::new();
    let client: Client<_, http_body_util::Empty<Bytes>> =
        Client::builder(TokioExecutor::new()).build(connector);

    let test_cases = [
        (Method::GET, StatusCode::OK),
        (Method::HEAD, StatusCode::OK),
        (Method::POST, StatusCode::METHOD_NOT_ALLOWED),
        (Method::PUT, StatusCode::METHOD_NOT_ALLOWED),
        (Method::DELETE, StatusCode::METHOD_NOT_ALLOWED),
        (Method::OPTIONS, StatusCode::METHOD_NOT_ALLOWED),
    ];

    for (method, expected_status) in test_cases {
        let request = Request::builder()
            .method(method.clone())
            .uri(format!("http://{}/", server.addr))
            .header("Host", "example.test")
            .body(http_body_util::Empty::<Bytes>::new())
            .context("build request")?;

        let response = client
            .request(request)
            .await
            .with_context(|| format!("request failed for method {}", method))?;

        assert_eq!(
            response.status(),
            expected_status,
            "unexpected status for method {}",
            method
        );

        if method == Method::HEAD {
            let body = response
                .into_body()
                .collect()
                .await
                .context("read HEAD response body")?
                .to_bytes();
            assert!(body.is_empty(), "HEAD response should not include a body");
        }
    }

    Ok(())
}

async fn wait_until_ready(addr: SocketAddr) -> Result<()> {
    let connector = HttpConnector::new();
    let client: Client<_, http_body_util::Empty<Bytes>> =
        Client::builder(TokioExecutor::new()).build(connector);

    let url = format!("http://{}/health", addr);

    for _ in 0..60 {
        let request = Request::builder()
            .method(Method::GET)
            .uri(&url)
            .body(http_body_util::Empty::<Bytes>::new())
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
