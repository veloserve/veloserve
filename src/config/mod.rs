//! Configuration module for VeloServe
//!
//! Handles TOML-based configuration for the server.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read configuration file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse configuration: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("Invalid configuration: {0}")]
    ValidationError(String),
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server settings
    #[serde(default)]
    pub server: ServerConfig,

    /// PHP settings
    #[serde(default)]
    pub php: PhpConfig,

    /// Cache settings
    #[serde(default)]
    pub cache: CacheConfig,

    /// SSL/TLS settings
    #[serde(default)]
    pub ssl: Option<SslConfig>,

    /// Virtual hosts
    #[serde(default)]
    pub virtualhost: Vec<VirtualHostConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            php: PhpConfig::default(),
            cache: CacheConfig::default(),
            ssl: None,
            virtualhost: vec![],
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from a string
    pub fn from_str(contents: &str) -> Result<Self, ConfigError> {
        let config: Config = toml::from_str(contents)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate server settings
        if self.server.max_connections == 0 {
            return Err(ConfigError::ValidationError(
                "max_connections must be greater than 0".to_string(),
            ));
        }

        // Validate PHP settings
        if self.php.workers == 0 {
            return Err(ConfigError::ValidationError(
                "php.workers must be greater than 0".to_string(),
            ));
        }

        // Validate SSL settings if enabled
        if let Some(ref ssl) = self.ssl {
            if ssl.cert.is_empty() || ssl.key.is_empty() {
                return Err(ConfigError::ValidationError(
                    "SSL cert and key paths must be specified".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Get the number of worker threads
    pub fn worker_threads(&self) -> usize {
        match self.server.workers.as_str() {
            "auto" => num_cpus::get(),
            n => n.parse().unwrap_or_else(|_| num_cpus::get()),
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// HTTP listen address
    #[serde(default = "default_listen")]
    pub listen: String,

    /// HTTPS listen address
    #[serde(default)]
    pub listen_ssl: Option<String>,

    /// Number of worker threads ("auto" or a number)
    #[serde(default = "default_workers")]
    pub workers: String,

    /// Maximum concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Keep-alive timeout in seconds
    #[serde(default = "default_keepalive_timeout")]
    pub keepalive_timeout: u64,

    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,

    /// Maximum request body size
    #[serde(default = "default_max_body_size")]
    pub max_body_size: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            listen_ssl: None,
            workers: default_workers(),
            max_connections: default_max_connections(),
            keepalive_timeout: default_keepalive_timeout(),
            request_timeout: default_request_timeout(),
            max_body_size: default_max_body_size(),
        }
    }
}

fn default_listen() -> String {
    "0.0.0.0:8080".to_string()
}

fn default_workers() -> String {
    "auto".to_string()
}

fn default_max_connections() -> usize {
    10000
}

fn default_keepalive_timeout() -> u64 {
    75
}

fn default_request_timeout() -> u64 {
    60
}

fn default_max_body_size() -> String {
    "100M".to_string()
}

/// PHP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhpConfig {
    /// PHP execution mode: "cgi", "socket" (vephp), or "embed"
    #[serde(default = "default_php_mode")]
    pub mode: PhpMode,

    /// Stack limit override for embed SAPI (e.g. "16M")
    #[serde(default = "default_embed_stack_limit")]
    pub embed_stack_limit: String,

    /// PHP version
    #[serde(default = "default_php_version")]
    pub version: String,

    /// Number of PHP worker processes
    #[serde(default = "default_php_workers")]
    pub workers: usize,

    /// PHP memory limit
    #[serde(default = "default_memory_limit")]
    pub memory_limit: String,

    /// Maximum execution time in seconds
    #[serde(default = "default_max_execution_time")]
    pub max_execution_time: u64,

    /// Path to PHP binary (auto-discovers EA-PHP if not set)
    #[serde(default)]
    pub binary_path: Option<String>,

    /// Unix socket path for vephp worker (used when mode = "socket")
    #[serde(default = "default_socket_path")]
    pub socket_path: String,

    /// Path to PHP error log file
    #[serde(default)]
    pub error_log: Option<String>,

    /// Display PHP errors in output (not recommended for production)
    #[serde(default)]
    pub display_errors: bool,

    /// Additional PHP configuration
    #[serde(default)]
    pub ini_settings: Vec<String>,

    /// Enable PHP
    #[serde(default = "default_true")]
    pub enable: bool,
}

impl Default for PhpConfig {
    fn default() -> Self {
        Self {
            mode: default_php_mode(),
            embed_stack_limit: default_embed_stack_limit(),
            version: default_php_version(),
            workers: default_php_workers(),
            memory_limit: default_memory_limit(),
            max_execution_time: default_max_execution_time(),
            binary_path: None,
            socket_path: default_socket_path(),
            error_log: None,
            display_errors: false,
            ini_settings: vec![],
            enable: true,
        }
    }
}

/// PHP execution mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PhpMode {
    /// Spawn php-cgi per request (simple, portable)
    Cgi,
    /// Connect to vephp persistent worker via Unix socket (like lsphp)
    Socket,
    /// Embedded PHP via libphp FFI (maximum performance, requires --features php-embed)
    Embed,
}

fn default_socket_path() -> String {
    "/run/veloserve/php.sock".to_string()
}

fn default_php_mode() -> PhpMode {
    PhpMode::Cgi
}

fn default_embed_stack_limit() -> String {
    "16M".to_string()
}

fn default_php_version() -> String {
    "8.2".to_string()
}

fn default_php_workers() -> usize {
    num_cpus::get() * 2
}

fn default_memory_limit() -> String {
    "256M".to_string()
}

fn default_max_execution_time() -> u64 {
    30
}

fn default_true() -> bool {
    true
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    #[serde(default = "default_true")]
    pub enable: bool,

    /// Cache storage backend
    #[serde(default = "default_cache_storage")]
    pub storage: CacheStorage,

    /// Memory limit for cache
    #[serde(default = "default_cache_memory_limit")]
    pub memory_limit: String,

    /// Default TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub default_ttl: u64,

    /// Redis URL (if using Redis backend)
    #[serde(default)]
    pub redis_url: Option<String>,

    /// Disk cache path
    #[serde(default = "default_cache_path")]
    pub disk_path: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable: true,
            storage: CacheStorage::Memory,
            memory_limit: default_cache_memory_limit(),
            default_ttl: default_cache_ttl(),
            redis_url: None,
            disk_path: default_cache_path(),
        }
    }
}

fn default_cache_storage() -> CacheStorage {
    CacheStorage::Memory
}

fn default_cache_memory_limit() -> String {
    "512M".to_string()
}

fn default_cache_ttl() -> u64 {
    3600
}

fn default_cache_path() -> String {
    "/var/cache/veloserve".to_string()
}

/// Cache storage backend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CacheStorage {
    Memory,
    Disk,
    Redis,
}

/// SSL/TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    /// Path to certificate file
    pub cert: String,

    /// Path to private key file
    pub key: String,

    /// Enabled protocols
    #[serde(default = "default_protocols")]
    pub protocols: Vec<String>,

    /// Enable OCSP stapling
    #[serde(default)]
    pub ocsp_stapling: bool,
}

fn default_protocols() -> Vec<String> {
    vec!["TLSv1.2".to_string(), "TLSv1.3".to_string()]
}

/// Virtual host configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualHostConfig {
    /// Domain name
    pub domain: String,

    /// Document root
    pub root: String,

    /// Platform type (wordpress, magento2, custom)
    #[serde(default)]
    pub platform: Option<String>,

    /// Per-vhost SSL certificate path (enables SNI for this domain)
    #[serde(default)]
    pub ssl_certificate: Option<String>,

    /// Per-vhost SSL key path
    #[serde(default)]
    pub ssl_certificate_key: Option<String>,

    /// Virtual host specific cache settings
    #[serde(default)]
    pub cache: Option<VHostCacheConfig>,

    /// Index files
    #[serde(default = "default_index_files")]
    pub index: Vec<String>,

    /// Error pages
    #[serde(default)]
    pub error_pages: std::collections::HashMap<u16, String>,
}

fn default_index_files() -> Vec<String> {
    vec!["index.php".to_string(), "index.html".to_string()]
}

/// Virtual host cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VHostCacheConfig {
    /// Enable caching for this vhost
    #[serde(default = "default_true")]
    pub enable: bool,

    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub ttl: u64,

    /// Vary headers
    #[serde(default)]
    pub vary: Vec<String>,

    /// Excluded paths from caching
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.listen, "0.0.0.0:8080");
        assert!(config.cache.enable);
        assert!(config.php.enable);
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
            [server]
            listen = "127.0.0.1:9000"
            workers = "4"
            max_connections = 5000

            [php]
            version = "8.3"
            workers = 8

            [cache]
            enable = true
            storage = "memory"
            default_ttl = 7200
        "#;

        let config = Config::from_str(toml).unwrap();
        assert_eq!(config.server.listen, "127.0.0.1:9000");
        assert_eq!(config.server.workers, "4");
        assert_eq!(config.php.version, "8.3");
        assert_eq!(config.cache.default_ttl, 7200);
    }

    #[test]
    fn test_worker_threads() {
        let mut config = Config::default();
        config.server.workers = "4".to_string();
        assert_eq!(config.worker_threads(), 4);

        config.server.workers = "auto".to_string();
        assert!(config.worker_threads() > 0);
    }
}

