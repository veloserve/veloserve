//! Communication Protocol
//!
//! Defines the protocol between VeloServe and veloserve-php workers.
//! Uses bincode for efficient binary serialization.

use std::collections::HashMap;
use std::path::PathBuf;

/// Types of PHP requests
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RequestType {
    /// Execute a PHP script
    Execute,
    /// Health check
    HealthCheck,
    /// Get status
    Status,
}

/// PHP request from VeloServe to veloserve-php
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhpRequest {
    /// Type of request
    pub request_type: RequestType,
    /// Path to PHP script
    pub script_path: PathBuf,
    /// Request method (GET, POST, etc.)
    pub method: String,
    /// Request URI
    pub uri: String,
    /// HTTP headers
    pub headers: HashMap<String, String>,
    /// POST data or request body
    pub body: Vec<u8>,
    /// Query string parameters
    pub query_params: HashMap<String, String>,
    /// Server/environment variables ($_SERVER)
    pub server_vars: HashMap<String, String>,
    /// Document root
    pub document_root: PathBuf,
    /// Remote IP address
    pub remote_addr: String,
    /// Request timeout
    pub timeout_secs: u32,
}

impl PhpRequest {
    /// Create a simple PHP execution request
    pub fn execute(script_path: PathBuf) -> Self {
        Self {
            request_type: RequestType::Execute,
            script_path,
            method: "GET".to_string(),
            uri: "/".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
            query_params: HashMap::new(),
            server_vars: HashMap::new(),
            document_root: PathBuf::from("/var/www"),
            remote_addr: "127.0.0.1".to_string(),
            timeout_secs: 30,
        }
    }

    /// Create a health check request
    pub fn health_check() -> Self {
        Self {
            request_type: RequestType::HealthCheck,
            script_path: PathBuf::new(),
            method: "GET".to_string(),
            uri: "/health".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
            query_params: HashMap::new(),
            server_vars: HashMap::new(),
            document_root: PathBuf::from("/var/www"),
            remote_addr: "127.0.0.1".to_string(),
            timeout_secs: 5,
        }
    }
}

/// PHP response from veloserve-php to VeloServe
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhpResponse {
    /// Whether the request was successful
    pub success: bool,
    /// HTTP status code (if applicable)
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body (stdout from PHP)
    pub body: String,
    /// Error message (if any)
    pub error: Option<String>,
    ///stderr output
    pub stderr: String,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Whether response was queued
    pub queued: bool,
}

impl PhpResponse {
    /// Create a successful response
    pub fn ok(body: &str, stderr: &str) -> Self {
        Self {
            success: true,
            status_code: 200,
            headers: HashMap::new(),
            body: body.to_string(),
            error: None,
            stderr: stderr.to_string(),
            execution_time_ms: 0,
            queued: false,
        }
    }

    /// Create an error response
    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            status_code: 500,
            headers: HashMap::new(),
            body: String::new(),
            error: Some(message.to_string()),
            stderr: message.to_string(),
            execution_time_ms: 0,
            queued: false,
        }
    }

    /// Create a queued response (will be processed later)
    pub fn queued() -> Self {
        Self {
            success: true,
            status_code: 202,
            headers: HashMap::new(),
            body: String::new(),
            error: None,
            stderr: String::new(),
            execution_time_ms: 0,
            queued: true,
        }
    }

    /// Add a header to the response
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Set HTTP status code
    pub fn with_status(mut self, code: u16) -> Self {
        self.status_code = code;
        self
    }
}
