//! HTTP Server module
//!
//! Core HTTP/1.1 and HTTP/2 server implementation using Hyper and Tokio.

mod handler;
mod router;
mod static_files;

pub use handler::RequestHandler;
pub use router::Router;
pub use static_files::StaticFileHandler;

use crate::cache::CacheManager;
use crate::config::Config;
use crate::php::PhpPool;

use anyhow::Result;
use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::server::conn::http2;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, error, info};

/// VeloServe HTTP Server
pub struct Server {
    config: Arc<Config>,
    cache: Arc<CacheManager>,
    php_pool: Arc<PhpPool>,
}

impl Server {
    /// Create a new server instance
    pub fn new(config: Config) -> Self {
        let config = Arc::new(config);
        let cache = Arc::new(CacheManager::new(&config.cache));
        let php_pool = Arc::new(PhpPool::new(&config.php));

        Self {
            config,
            cache,
            php_pool,
        }
    }

    /// Run the server
    pub async fn run(&self) -> Result<()> {
        let addr: SocketAddr = self.config.server.listen.parse()?;

        info!("Starting VeloServe on {}", addr);

        // Start PHP worker pool
        if self.config.php.enable {
            info!(
                "Starting PHP worker pool with {} workers",
                self.config.php.workers
            );
            self.php_pool.start().await?;
        }

        // Create TCP listener
        let listener = TcpListener::bind(addr).await?;
        info!("Server listening on http://{}", addr);

        // Accept connections
        loop {
            let (stream, remote_addr) = listener.accept().await?;
            debug!("Accepted connection from {}", remote_addr);

            let config = self.config.clone();
            let cache = self.cache.clone();
            let php_pool = self.php_pool.clone();

            tokio::spawn(async move {
                let io = TokioIo::new(stream);

                // Create service function
                let service = service_fn(move |req: Request<hyper::body::Incoming>| {
                    let config = config.clone();
                    let cache = cache.clone();
                    let php_pool = php_pool.clone();

                    async move {
                        handle_request(req, remote_addr, config, cache, php_pool).await
                    }
                });

                // Use HTTP/1.1 by default (HTTP/2 requires ALPN negotiation with TLS)
                let conn = http1::Builder::new()
                    .keep_alive(true)
                    .serve_connection(io, service);

                if let Err(e) = conn.await {
                    if !is_connection_closed_error(&e) {
                        error!("Connection error: {}", e);
                    }
                }
            });
        }
    }

    /// Run the server with HTTP/2 support (requires TLS)
    pub async fn run_h2(&self, listener: TcpListener) -> Result<()> {
        info!("Starting HTTP/2 server");

        loop {
            let (stream, remote_addr) = listener.accept().await?;
            debug!("Accepted HTTP/2 connection from {}", remote_addr);

            let config = self.config.clone();
            let cache = self.cache.clone();
            let php_pool = self.php_pool.clone();

            tokio::spawn(async move {
                let io = TokioIo::new(stream);

                let service = service_fn(move |req: Request<hyper::body::Incoming>| {
                    let config = config.clone();
                    let cache = cache.clone();
                    let php_pool = php_pool.clone();

                    async move {
                        handle_request(req, remote_addr, config, cache, php_pool).await
                    }
                });

                let conn = http2::Builder::new(TokioExecutor)
                    .serve_connection(io, service);

                if let Err(e) = conn.await {
                    error!("HTTP/2 connection error: {}", e);
                }
            });
        }
    }
}

/// Check if error is just a closed connection (not worth logging)
fn is_connection_closed_error(e: &hyper::Error) -> bool {
    if e.is_incomplete_message() {
        return true;
    }
    if let Some(source) = e.source() {
        if let Some(io_err) = source.downcast_ref::<std::io::Error>() {
            return matches!(
                io_err.kind(),
                std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe
            );
        }
    }
    false
}

/// Handle incoming HTTP request
async fn handle_request(
    req: Request<hyper::body::Incoming>,
    remote_addr: SocketAddr,
    config: Arc<Config>,
    cache: Arc<CacheManager>,
    php_pool: Arc<PhpPool>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = std::time::Instant::now();

    debug!("{} {} from {}", method, uri, remote_addr);

    // Create request handler
    let handler = RequestHandler::new(config, cache, php_pool);

    // Handle the request
    let response = match handler.handle(req).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Request handling error: {}", e);
            Response::builder()
                .status(500)
                .header("Content-Type", "text/plain")
                .header("Server", crate::SERVER_NAME)
                .body(Full::new(Bytes::from("Internal Server Error")))
                .unwrap()
        }
    };

    let duration = start.elapsed();
    let status = response.status();

    info!(
        "{} {} {} {} {:?}",
        remote_addr, method, uri, status.as_u16(), duration
    );

    Ok(response)
}

/// Tokio executor for HTTP/2
#[derive(Clone, Copy)]
struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::spawn(fut);
    }
}

use std::error::Error;

