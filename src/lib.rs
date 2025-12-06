//! VeloServe - High-performance web server with integrated PHP support
//!
//! VeloServe is designed as a modern alternative to LiteSpeed, featuring:
//! - Integrated PHP processing (no PHP-FPM required)
//! - Intelligent multi-layer caching
//! - Optimized WordPress and Magento support
//! - HTTP/1.1 and HTTP/2 support
//!
//! # Example
//!
//! ```rust,no_run
//! use veloserve::server::Server;
//! use veloserve::config::Config;
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = Config::load("veloserve.toml").unwrap();
//!     let server = Server::new(config);
//!     server.run().await.unwrap();
//! }
//! ```

pub mod cache;
pub mod cli;
pub mod config;
pub mod php;
pub mod server;

pub use config::Config;
pub use server::Server;

/// VeloServe version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Server name for HTTP headers
pub const SERVER_NAME: &str = "VeloServe";

