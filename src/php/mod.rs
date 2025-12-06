//! PHP Integration Module
//!
//! Process pool-based PHP execution for VeloServe.
//! Implements CGI/FastCGI-style environment variables like Nginx + PHP-FPM.
//!
//! ## CGI Environment Variables
//!
//! This module sets all standard CGI environment variables:
//! - `SCRIPT_FILENAME`: Absolute path to the PHP script
//! - `SCRIPT_NAME`: URI path to the script
//! - `PATH_INFO`: Additional path after the script name
//! - `PATH_TRANSLATED`: Absolute path translation of PATH_INFO
//! - `DOCUMENT_ROOT`: Document root directory
//! - `REQUEST_URI`: Original request URI
//! - `QUERY_STRING`: Query parameters
//!
//! ## Clean URL Support
//!
//! Supports clean URLs like WordPress/Laravel:
//! - `/blog/post/123` → `index.php` with `PATH_INFO=/blog/post/123`
//! - `/api.php/users/1` → `api.php` with `PATH_INFO=/users/1`

use crate::config::PhpConfig;
use anyhow::{anyhow, Result};
use hyper::Request;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

/// PHP worker pool for executing PHP scripts
pub struct PhpPool {
    /// Pool configuration
    config: PhpConfig,

    /// Path to PHP binary
    php_binary: PathBuf,

    /// Number of active workers
    active_workers: AtomicUsize,

    /// Request semaphore (limits concurrent PHP executions)
    semaphore: Arc<Semaphore>,

    /// Is the pool running
    running: AtomicBool,

    /// Is PHP actually available (binary found and working)
    available: AtomicBool,

    /// PHP version string
    php_version: Mutex<Option<String>>,
}

impl PhpPool {
    /// Create a new PHP worker pool
    pub fn new(config: &PhpConfig) -> Self {
        let php_binary = config
            .binary_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| find_php_binary(&config.version));

        info!("PHP binary: {:?}", php_binary);

        Self {
            config: config.clone(),
            php_binary,
            active_workers: AtomicUsize::new(0),
            semaphore: Arc::new(Semaphore::new(config.workers)),
            running: AtomicBool::new(false),
            available: AtomicBool::new(false),
            php_version: Mutex::new(None),
        }
    }

    /// Check if PHP is available and working
    pub fn is_available(&self) -> bool {
        self.available.load(Ordering::SeqCst)
    }

    /// Start the PHP worker pool
    pub async fn start(&self) -> Result<()> {
        if !self.config.enable {
            info!("PHP support disabled in configuration");
            self.available.store(false, Ordering::SeqCst);
            return Ok(());
        }

        // Verify PHP binary exists
        if !self.php_binary.exists() && self.php_binary.to_str() != Some("php") {
            warn!(
                "PHP binary not found at {:?}, PHP support disabled",
                self.php_binary
            );
            self.available.store(false, Ordering::SeqCst);
            return Ok(());
        }

        // Test PHP installation
        match self.get_php_version().await {
            Ok(version) => {
                info!("PHP version: {}", version);
                *self.php_version.lock() = Some(version);
                self.available.store(true, Ordering::SeqCst);
            }
            Err(e) => {
                warn!("PHP not working: {}, PHP support disabled", e);
                self.available.store(false, Ordering::SeqCst);
                return Ok(());
            }
        }

        self.running.store(true, Ordering::SeqCst);

        info!(
            "PHP worker pool started with {} workers",
            self.config.workers
        );

        Ok(())
    }

    /// Execute a PHP script with full CGI environment (like Nginx + PHP-FPM)
    ///
    /// # Arguments
    /// * `script_path` - Absolute path to the PHP script
    /// * `req` - HTTP request
    /// * `doc_root` - Document root directory
    /// * `script_name` - URI path to the script (e.g., "/index.php")
    /// * `path_info` - Additional path info (e.g., "/blog/post/123")
    pub async fn execute_with_path_info(
        &self,
        script_path: &Path,
        req: &Request<hyper::body::Incoming>,
        doc_root: &Path,
        script_name: &str,
        path_info: &str,
    ) -> Result<String> {
        if !self.is_available() {
            return Err(anyhow!("PHP support is not available"));
        }

        // Acquire semaphore permit (limits concurrent PHP processes)
        let _permit = self.semaphore.acquire().await
            .map_err(|_| anyhow!("Failed to acquire PHP worker permit"))?;

        self.active_workers.fetch_add(1, Ordering::SeqCst);
        let result = self.do_execute(script_path, req, doc_root, script_name, path_info).await;
        self.active_workers.fetch_sub(1, Ordering::SeqCst);

        result
    }

    /// Execute a PHP script (simple mode - for backward compatibility)
    pub async fn execute(
        &self,
        script_path: &Path,
        req: &Request<hyper::body::Incoming>,
    ) -> Result<String> {
        let script_name = req.uri().path();
        let doc_root = script_path.parent().unwrap_or(Path::new("/"));
        self.execute_with_path_info(script_path, req, doc_root, script_name, "").await
    }

    /// Execute a PHP script with minimal parameters
    pub async fn execute_simple(&self, script_path: &Path) -> Result<String> {
        if !self.is_available() {
            return Err(anyhow!("PHP support is not available"));
        }

        let _permit = self.semaphore.acquire().await
            .map_err(|_| anyhow!("Failed to acquire PHP worker permit"))?;

        self.active_workers.fetch_add(1, Ordering::SeqCst);
        let result = self.do_execute_simple(script_path).await;
        self.active_workers.fetch_sub(1, Ordering::SeqCst);

        result
    }

    /// Internal: Execute PHP with full CGI environment
    async fn do_execute(
        &self,
        script_path: &Path,
        req: &Request<hyper::body::Incoming>,
        doc_root: &Path,
        script_name: &str,
        path_info: &str,
    ) -> Result<String> {
        debug!(
            "Executing PHP: {} (script_name={}, path_info={})",
            script_path.display(),
            script_name,
            path_info
        );

        // Build CGI environment variables (like Nginx + PHP-FPM)
        let env = build_cgi_env(req, script_path, doc_root, script_name, path_info);

        // Build command
        let mut cmd = Command::new(&self.php_binary);
        self.configure_php_command(&mut cmd);

        // Execute the PHP script directly
        cmd.arg(script_path);

        // Set working directory to script directory for relative includes
        if let Some(script_dir) = script_path.parent() {
            cmd.current_dir(script_dir);
        }

        // Set environment variables
        cmd.envs(&env);

        // Configure I/O
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Spawn and execute
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.max_execution_time),
            cmd.output(),
        )
        .await
        .map_err(|_| anyhow!("PHP script execution timed out after {}s", self.config.max_execution_time))?
        .map_err(|e| anyhow!("Failed to execute PHP script: {}", e))?;

        // Log any errors
        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.trim().is_empty() {
                warn!("PHP stderr: {}", stderr.trim());
            }
        }

        // Check exit status but still return output if we have it
        if !output.status.success() && output.stdout.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("PHP script failed: {}", stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Internal: Execute PHP with minimal environment
    async fn do_execute_simple(&self, script_path: &Path) -> Result<String> {
        let mut cmd = Command::new(&self.php_binary);
        self.configure_php_command(&mut cmd);
        cmd.arg(script_path);

        if let Some(parent) = script_path.parent() {
            cmd.current_dir(parent);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.max_execution_time),
            cmd.output(),
        )
        .await
        .map_err(|_| anyhow!("PHP script execution timed out"))?
        .map_err(|e| anyhow!("Failed to execute PHP: {}", e))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Configure PHP command with standard settings
    fn configure_php_command(&self, cmd: &mut Command) {
        // Memory limit
        cmd.arg("-d").arg(format!("memory_limit={}", self.config.memory_limit));

        // Execution time
        cmd.arg("-d").arg(format!("max_execution_time={}", self.config.max_execution_time));

        // Security settings
        cmd.arg("-d").arg("expose_php=Off");
        cmd.arg("-d").arg("display_errors=Off");
        cmd.arg("-d").arg("log_errors=On");

        // Add custom ini settings
        for setting in &self.config.ini_settings {
            cmd.arg("-d").arg(setting);
        }
    }

    /// Get PHP version string
    async fn get_php_version(&self) -> Result<String> {
        let output = Command::new(&self.php_binary)
            .arg("-v")
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute PHP: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!("PHP version check failed"));
        }

        let version = String::from_utf8_lossy(&output.stdout);
        let first_line = version.lines().next().unwrap_or("Unknown");
        Ok(first_line.to_string())
    }

    /// Get pool statistics
    pub fn stats(&self) -> serde_json::Value {
        serde_json::json!({
            "enabled": self.config.enable,
            "available": self.available.load(Ordering::SeqCst),
            "running": self.running.load(Ordering::SeqCst),
            "version": self.php_version.lock().clone(),
            "max_workers": self.config.workers,
            "active_workers": self.active_workers.load(Ordering::SeqCst),
            "memory_limit": self.config.memory_limit,
            "max_execution_time": self.config.max_execution_time,
        })
    }
}

/// Find PHP binary on the system
fn find_php_binary(preferred_version: &str) -> PathBuf {
    // Try version-specific paths first
    let version_paths = [
        format!("/usr/bin/php{}", preferred_version),
        format!("/usr/local/bin/php{}", preferred_version),
        format!("/usr/bin/php{}", preferred_version.replace('.', "")),
    ];

    for path in &version_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            return p;
        }
    }

    // Try common paths
    let common_paths = [
        "/usr/bin/php",
        "/usr/local/bin/php",
        "/opt/php/bin/php",
        "/opt/homebrew/bin/php",
    ];

    for path in &common_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            return p;
        }
    }

    // Default to "php" and hope it's in PATH
    PathBuf::from("php")
}

/// Build CGI environment variables (like Nginx + PHP-FPM)
///
/// This creates all standard CGI environment variables as specified in RFC 3875
/// and as implemented by Nginx with PHP-FPM.
fn build_cgi_env(
    req: &Request<hyper::body::Incoming>,
    script_path: &Path,
    doc_root: &Path,
    script_name: &str,
    path_info: &str,
) -> HashMap<String, String> {
    let mut env = HashMap::new();

    // === CGI/1.1 Standard Variables (RFC 3875) ===

    env.insert("GATEWAY_INTERFACE".to_string(), "CGI/1.1".to_string());
    env.insert("SERVER_PROTOCOL".to_string(), format!("{:?}", req.version()));
    env.insert(
        "SERVER_SOFTWARE".to_string(),
        format!("VeloServe/{}", crate::VERSION),
    );

    // Request method
    env.insert("REQUEST_METHOD".to_string(), req.method().to_string());

    // Request URI (original, includes query string)
    env.insert("REQUEST_URI".to_string(), req.uri().to_string());

    // Script name (URI path to the PHP script)
    env.insert("SCRIPT_NAME".to_string(), script_name.to_string());

    // Script filename (absolute filesystem path)
    env.insert(
        "SCRIPT_FILENAME".to_string(),
        script_path.to_string_lossy().to_string(),
    );

    // Document root
    env.insert(
        "DOCUMENT_ROOT".to_string(),
        doc_root.to_string_lossy().to_string(),
    );

    // Query string
    env.insert(
        "QUERY_STRING".to_string(),
        req.uri().query().unwrap_or("").to_string(),
    );

    // === PATH_INFO support (for clean URLs) ===
    // This is crucial for WordPress, Laravel, and other frameworks

    if !path_info.is_empty() {
        env.insert("PATH_INFO".to_string(), path_info.to_string());

        // PATH_TRANSLATED: Document root + PATH_INFO
        let path_translated = doc_root.join(path_info.trim_start_matches('/'));
        env.insert(
            "PATH_TRANSLATED".to_string(),
            path_translated.to_string_lossy().to_string(),
        );
    }

    // === Server identification ===

    // Extract host and port from Host header
    if let Some(host) = req.headers().get("host") {
        if let Ok(host_str) = host.to_str() {
            let parts: Vec<&str> = host_str.split(':').collect();
            env.insert("SERVER_NAME".to_string(), parts[0].to_string());
            env.insert("HTTP_HOST".to_string(), host_str.to_string());

            if parts.len() > 1 {
                env.insert("SERVER_PORT".to_string(), parts[1].to_string());
            } else {
                env.insert("SERVER_PORT".to_string(), "80".to_string());
            }
        }
    } else {
        env.insert("SERVER_NAME".to_string(), "localhost".to_string());
        env.insert("SERVER_PORT".to_string(), "80".to_string());
    }

    // === Content headers ===

    if let Some(ct) = req.headers().get("content-type") {
        if let Ok(v) = ct.to_str() {
            env.insert("CONTENT_TYPE".to_string(), v.to_string());
        }
    }

    if let Some(cl) = req.headers().get("content-length") {
        if let Ok(v) = cl.to_str() {
            env.insert("CONTENT_LENGTH".to_string(), v.to_string());
        }
    }

    // === HTTP headers (converted to HTTP_* format) ===

    for (name, value) in req.headers() {
        // Skip content-type and content-length (already handled)
        if name == "content-type" || name == "content-length" {
            continue;
        }

        let env_name = format!(
            "HTTP_{}",
            name.as_str().to_uppercase().replace('-', "_")
        );

        if let Ok(v) = value.to_str() {
            env.insert(env_name, v.to_string());
        }
    }

    // === PHP-specific variables ===

    // Required for PHP-CGI to process the request
    env.insert("REDIRECT_STATUS".to_string(), "200".to_string());

    // PHP_SELF - same as SCRIPT_NAME for direct requests
    env.insert("PHP_SELF".to_string(), script_name.to_string());

    // HTTPS indicator
    // TODO: Set this based on actual connection
    env.insert("HTTPS".to_string(), "off".to_string());

    // Remote address (would be filled in by the server)
    env.insert("REMOTE_ADDR".to_string(), "127.0.0.1".to_string());
    env.insert("REMOTE_PORT".to_string(), "0".to_string());

    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_php_binary() {
        let path = find_php_binary("8.2");
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn test_cgi_env_path_info() {
        // This would require mocking the request
        // For now, just verify the function signature works
    }
}
