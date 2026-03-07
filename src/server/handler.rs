//! HTTP Request Handler
//!
//! Handles incoming HTTP requests similar to Nginx/Apache/LiteSpeed.
//! Supports static files, PHP processing, and URL rewriting.

use crate::cache::{build_page_cache_key, CacheManager};
use crate::config::Config;
use crate::php::sapi::PhpResponse;
use crate::php::PhpPool;
use crate::server::static_files::StaticFileHandler;

use anyhow::{anyhow, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, Response, StatusCode};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use hyper::header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, SET_COOKIE};
use hyper::http::HeaderValue;
use tracing::{debug, warn};

/// Request handler for VeloServe
///
/// Implements request handling similar to traditional web servers:
/// - Static file serving with proper MIME types
/// - PHP script execution with PATH_INFO support
/// - Directory index handling
/// - Try-files pattern for clean URLs
pub struct RequestHandler {
    config: Arc<Config>,
    cache: Arc<CacheManager>,
    php_pool: Arc<PhpPool>,
    static_handler: StaticFileHandler,
}

/// Result of resolving a PHP script path
#[derive(Debug)]
struct PhpPathInfo {
    /// The actual PHP script file path
    script_filename: PathBuf,
    /// The script name (URI path to the script)
    script_name: String,
    /// Additional path info after the script
    path_info: String,
}

#[derive(Debug, Clone)]
struct CacheContext {
    key: String,
    domain: String,
    path: String,
    ttl: Duration,
}

impl RequestHandler {
    /// Create a new request handler
    pub fn new(config: Arc<Config>, cache: Arc<CacheManager>, php_pool: Arc<PhpPool>) -> Self {
        let static_handler = StaticFileHandler::new();

        Self {
            config,
            cache,
            php_pool,
            static_handler,
        }
    }

    /// Handle an incoming request
    ///
    /// Request processing order (similar to Nginx/Apache):
    /// 1. Internal endpoints (health, API)
    /// 2. Check if exact file exists
    /// 3. If directory, try index files
    /// 4. If PHP file, execute with PATH_INFO
    /// 5. Try files pattern for clean URLs
    /// 6. Return 404
    pub async fn handle(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<Full<Bytes>>> {
        let method = req.method().clone();
        let path = req.uri().path().to_string();

        // Health check endpoint (internal)
        if path == "/health" || path == "/healthz" {
            return self.health_check();
        }

        // API endpoints (internal)
        if path.starts_with("/api/v1/") {
            return self.handle_api(req).await;
        }

        // Find the virtual host and document root
        let (doc_root, vhost) = self.find_vhost(&req);
        debug!("Document root: {:?}, path: {}", doc_root, path);

        let cache_context = self.cache_context(&req, &path, vhost);
        if let Some(context) = &cache_context {
            if let Some((data, content_type)) = self.cache.get_with_metadata(&context.key).await {
                return self.cached_response(&method, &data, &content_type);
            }
        }

        // Get index files from vhost config or use defaults
        let index_files = vhost.map(|v| v.index.clone()).unwrap_or_else(|| {
            vec![
                "index.php".to_string(),
                "index.html".to_string(),
                "index.htm".to_string(),
            ]
        });

        // Read the request body for POST/PUT requests
        // We need to consume the body before we can use the request further
        let (parts, incoming_body) = req.into_parts();

        let body = if method == Method::POST || method == Method::PUT {
            match incoming_body.collect().await {
                Ok(collected) => collected.to_bytes().to_vec(),
                Err(e) => {
                    warn!("Failed to read request body: {}", e);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        // Create a reference-like wrapper with the request parts for PHP execution
        let req_parts = &parts;

        // === NGINX/APACHE-STYLE REQUEST PROCESSING ===

        // Step 1: Try the exact URI as a file
        let file_path = self.resolve_path(&doc_root, &path);

        if file_path.is_file() {
            // Exact file exists
            if self.is_php_file(&file_path) {
                // PHP file - execute it
                let response = self
                    .execute_php(req_parts, &doc_root, &file_path, &path, "", body)
                    .await?;
                return self
                    .finalize_response(response, cache_context.as_ref(), &method)
                    .await;
            } else {
                // Static file - serve it
                let response = self.serve_static_parts(req_parts, &file_path).await?;
                return self
                    .finalize_response(response, cache_context.as_ref(), &method)
                    .await;
            }
        }

        // Step 2: If directory, try index files (like DirectoryIndex in Apache)
        if file_path.is_dir() {
            for index in &index_files {
                let index_path = file_path.join(index);
                if index_path.is_file() {
                    let index_uri = format!("{}/{}", path.trim_end_matches('/'), index);

                    if self.is_php_file(&index_path) {
                        let response = self
                            .execute_php(req_parts, &doc_root, &index_path, &index_uri, "", body)
                            .await?;
                        return self
                            .finalize_response(response, cache_context.as_ref(), &method)
                            .await;
                    } else {
                        let response = self.serve_static_parts(req_parts, &index_path).await?;
                        return self
                            .finalize_response(response, cache_context.as_ref(), &method)
                            .await;
                    }
                }
            }
            // No index file found - return 403 (no directory listing)
            let response = self.forbidden("Directory listing denied")?;
            return self
                .finalize_response(response, cache_context.as_ref(), &method)
                .await;
        }

        // Step 3: Check for PHP file with PATH_INFO
        // This handles URLs like /index.php/page/1 or /blog.php/post/hello
        if let Some(php_info) = self.resolve_php_path_info(&doc_root, &path) {
            let response = self
                .execute_php(
                    req_parts,
                    &doc_root,
                    &php_info.script_filename,
                    &php_info.script_name,
                    &php_info.path_info,
                    body,
                )
                .await?;
            return self
                .finalize_response(response, cache_context.as_ref(), &method)
                .await;
        }

        // Step 4: Try files pattern (like Nginx try_files $uri $uri/ /index.php$is_args$args)
        // This is essential for WordPress, Laravel, and other frameworks with clean URLs
        if self.php_pool.is_available() {
            // Try /index.php with the original URI as PATH_INFO
            let front_controller = doc_root.join("index.php");
            if front_controller.is_file() {
                debug!(
                    "Using front controller pattern: index.php with PATH_INFO={}",
                    path
                );
                let response = self
                    .execute_php(
                        req_parts,
                        &doc_root,
                        &front_controller,
                        "/index.php",
                        &path,
                        body,
                    )
                    .await?;
                return self
                    .finalize_response(response, cache_context.as_ref(), &method)
                    .await;
            }
        }

        // Step 5: Nothing found - return 404
        let response = self.not_found()?;
        self.finalize_response(response, cache_context.as_ref(), &method)
            .await
    }

    /// Check if a file is a PHP file
    fn is_php_file(&self, path: &Path) -> bool {
        path.extension()
            .map(|ext| ext.to_str().unwrap_or("").to_lowercase() == "php")
            .unwrap_or(false)
    }

    /// Resolve PHP path with PATH_INFO support
    ///
    /// For a URL like /blog/index.php/post/123:
    /// - script_filename: /var/www/blog/index.php
    /// - script_name: /blog/index.php
    /// - path_info: /post/123
    fn resolve_php_path_info(&self, doc_root: &Path, uri_path: &str) -> Option<PhpPathInfo> {
        // Split the path and look for a PHP file
        let parts: Vec<&str> = uri_path.split('/').collect();
        let mut accumulated_path = String::new();

        for (i, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                accumulated_path.push('/');
                accumulated_path.push_str(part);
            }

            // Check if this accumulated path is a PHP file
            if part.ends_with(".php") || part.contains(".php") {
                let script_path = self.resolve_path(doc_root, &accumulated_path);
                if script_path.is_file() && self.is_php_file(&script_path) {
                    // Found a PHP file - rest is PATH_INFO
                    let path_info = if i + 1 < parts.len() {
                        format!("/{}", parts[i + 1..].join("/"))
                    } else {
                        String::new()
                    };

                    return Some(PhpPathInfo {
                        script_filename: script_path,
                        script_name: accumulated_path,
                        path_info,
                    });
                }
            }
        }

        None
    }

    /// Execute a PHP script
    async fn execute_php(
        &self,
        req_parts: &hyper::http::request::Parts,
        doc_root: &Path,
        script_path: &Path,
        script_name: &str,
        path_info: &str,
        body: Vec<u8>,
    ) -> Result<Response<Full<Bytes>>> {
        // Check if PHP is available
        if !self.php_pool.is_available() {
            warn!("PHP requested but not available: {}", script_name);
            return self.internal_error("PHP is not available on this server");
        }

        debug!(
            "Executing PHP: script={}, script_name={}, path_info={}, body_len={}",
            script_path.display(),
            script_name,
            path_info,
            body.len()
        );

        // Choose execution mode: embed or CGI
        if self.php_pool.is_embed_mode() {
            match self
                .php_pool
                .execute_embed(
                    script_path,
                    req_parts,
                    doc_root,
                    script_name,
                    path_info,
                    &body,
                )
                .await
            {
                Ok(resp) => self.build_embed_response(resp),
                Err(e) => {
                    warn!("PHP embed execution error: {}", e);
                    self.internal_error(&format!("PHP Error: {}", e))
                }
            }
        } else {
            // Execute PHP script with full CGI environment and POST body
            match self
                .php_pool
                .execute_cgi(
                    script_path,
                    req_parts,
                    doc_root,
                    script_name,
                    path_info,
                    &body,
                )
                .await
            {
                Ok(output) => {
                    // Parse PHP output (may contain headers)
                    self.parse_php_response(&output)
                }
                Err(e) => {
                    warn!("PHP execution error: {}", e);
                    self.internal_error(&format!("PHP Error: {}", e))
                }
            }
        }
    }

    /// Build HTTP response from embedded PHP output
    fn build_embed_response(&self, resp: PhpResponse) -> Result<Response<Full<Bytes>>> {
        let mut builder = Response::builder();

        let status = StatusCode::from_u16(resp.status_code).unwrap_or(StatusCode::OK);
        builder = builder.status(status);

        let mut content_type_set = false;
        // Headers is a Vec to support multiple headers with same name (e.g., Set-Cookie)
        for (name, value) in &resp.headers {
            if name.eq_ignore_ascii_case("content-type") {
                content_type_set = true;
            }
            builder = builder.header(name.as_str(), value.as_str());
        }

        if !content_type_set {
            builder = builder.header("Content-Type", "text/html; charset=utf-8");
        }

        builder = builder
            .header("Server", crate::SERVER_NAME)
            .header("X-Powered-By", format!("VeloServe/{}", crate::VERSION));

        Ok(builder
            .body(Full::new(Bytes::from(resp.body)))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from("Internal Server Error")))
                    .unwrap()
            }))
    }

    /// Parse PHP response (headers + body)
    ///
    /// PHP CGI can output headers followed by body, separated by a blank line.
    /// But we need to be careful - only valid HTTP headers should be parsed.
    fn parse_php_response(&self, output: &str) -> Result<Response<Full<Bytes>>> {
        let mut builder = Response::builder();
        let mut status = StatusCode::OK;
        let mut content_type = "text/html; charset=utf-8".to_string();
        let mut body = output;

        // Check if output starts with HTTP headers
        // Valid headers start with alphanumeric character, not < (HTML) or whitespace
        let first_char = output.chars().next().unwrap_or(' ');
        let looks_like_headers = first_char.is_ascii_alphabetic();

        if looks_like_headers {
            // Try to find header/body separator
            let separator_pos = if let Some(pos) = output.find("\r\n\r\n") {
                Some((pos, 4))
            } else if let Some(pos) = output.find("\n\n") {
                // Make sure this isn't just empty lines in HTML/CSS
                // Headers should be before position ~500 typically
                if pos < 500 {
                    Some((pos, 2))
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((pos, skip)) = separator_pos {
                let headers_part = &output[..pos];

                // Validate that the first line looks like a header (Name: value)
                let first_line = headers_part.lines().next().unwrap_or("");
                let has_valid_header = first_line.contains(':')
                    && !first_line.starts_with('<')
                    && !first_line.contains('{')
                    && first_line
                        .split(':')
                        .next()
                        .map(|n| {
                            n.chars()
                                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                        })
                        .unwrap_or(false);

                if has_valid_header {
                    body = &output[pos + skip..];

                    // Parse headers
                    for line in headers_part.lines() {
                        if let Some((name, value)) = line.split_once(':') {
                            let name = name.trim();
                            let value = value.trim();

                            // Validate header name
                            if !name
                                .chars()
                                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                            {
                                continue;
                            }

                            match name.to_lowercase().as_str() {
                                "status" => {
                                    if let Some(code) = value.split_whitespace().next() {
                                        if let Ok(code) = code.parse::<u16>() {
                                            status = StatusCode::from_u16(code)
                                                .unwrap_or(StatusCode::OK);
                                        }
                                    }
                                }
                                "content-type" => {
                                    content_type = value.to_string();
                                }
                                "location" => {
                                    if status == StatusCode::OK {
                                        status = StatusCode::FOUND;
                                    }
                                    builder = builder.header("Location", value);
                                }
                                "set-cookie"
                                | "cache-control"
                                | "expires"
                                | "pragma"
                                | "x-powered-by"
                                | "x-frame-options"
                                | "x-content-type-options" => {
                                    builder = builder.header(name, value);
                                }
                                _ => {
                                    // Skip unknown headers from PHP to avoid issues
                                }
                            }
                        }
                    }
                }
            }
        }

        builder
            .status(status)
            .header("Content-Type", &content_type)
            .header("Server", crate::SERVER_NAME)
            .header("X-Powered-By", format!("VeloServe/{}", crate::VERSION))
            .body(Full::new(Bytes::from(body.to_string())))
            .map_err(|e| anyhow!("Failed to build response: {}", e))
    }

    /// Serve a static file
    async fn serve_static(
        &self,
        req: &Request<hyper::body::Incoming>,
        path: &Path,
    ) -> Result<Response<Full<Bytes>>> {
        // Only GET and HEAD for static files
        if req.method() != Method::GET && req.method() != Method::HEAD {
            return self.method_not_allowed();
        }

        self.static_handler.serve(path).await
    }

    /// Serve a static file (using request parts)
    async fn serve_static_parts(
        &self,
        req_parts: &hyper::http::request::Parts,
        path: &Path,
    ) -> Result<Response<Full<Bytes>>> {
        // Only GET and HEAD for static files
        if req_parts.method != Method::GET && req_parts.method != Method::HEAD {
            return self.method_not_allowed();
        }

        self.static_handler.serve(path).await
    }

    /// Handle API requests
    async fn handle_api(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<Full<Bytes>>> {
        let path = req.uri().path();
        let method = req.method().clone();

        match (method, path) {
            (Method::GET, "/api/v1/status") => self.api_status(),
            (Method::GET, "/api/v1/cache/stats") => self.api_cache_stats(),
            (Method::GET, "/api/v1/cache/config") => self.api_cache_config(),
            (Method::GET, "/api/v1/cache/purge") | (Method::POST, "/api/v1/cache/purge") => {
                self.api_cache_purge(&req).await
            }
            (Method::GET, "/api/v1/cache/warm") | (Method::POST, "/api/v1/cache/warm") => {
                self.api_cache_warm(&req).await
            }
            (Method::GET, "/api/v1/metrics") => self.api_metrics(),
            (Method::GET, "/api/v1/workers") => self.api_workers(),
            _ => self.not_found(),
        }
    }

    /// API: Server status
    fn api_status(&self) -> Result<Response<Full<Bytes>>> {
        let status = serde_json::json!({
            "status": "running",
            "version": crate::VERSION,
            "server": crate::SERVER_NAME,
            "php_available": self.php_pool.is_available(),
            "cache_enabled": self.config.cache.enable,
        });

        self.json_response(status)
    }

    /// API: Cache statistics
    fn api_cache_stats(&self) -> Result<Response<Full<Bytes>>> {
        let stats = self.cache.stats();
        self.json_response(stats)
    }

    /// API: Cache configuration
    fn api_cache_config(&self) -> Result<Response<Full<Bytes>>> {
        let vhosts: Vec<serde_json::Value> = self
            .config
            .virtualhost
            .iter()
            .map(|vhost| {
                let (enabled, ttl, exclude) = if let Some(cache) = &vhost.cache {
                    (cache.enable, cache.ttl, cache.exclude.clone())
                } else {
                    (
                        self.config.cache.enable,
                        self.config.cache.default_ttl,
                        Vec::<String>::new(),
                    )
                };

                serde_json::json!({
                    "domain": vhost.domain,
                    "cache_enabled": enabled,
                    "ttl": ttl,
                    "exclude": exclude,
                })
            })
            .collect();

        self.json_response(serde_json::json!({
            "cache": {
                "enabled": self.config.cache.enable,
                "l1_enabled": self.config.cache.l1_enabled,
                "l2_enabled": self.config.cache.l2_enabled,
                "storage": self.config.cache.storage,
                "memory_limit": self.config.cache.memory_limit,
                "default_ttl": self.config.cache.default_ttl,
                "disk_path": self.config.cache.disk_path,
                "redis_url": self.config.cache.redis_url,
            },
            "vhosts": vhosts
        }))
    }

    /// API: Purge cache
    async fn api_cache_purge(
        &self,
        req: &Request<hyper::body::Incoming>,
    ) -> Result<Response<Full<Bytes>>> {
        let query = req.uri().query().unwrap_or("");
        let tag = self.query_param(query, "tag");
        let domain = self.query_param(query, "domain");
        let key = self.query_param(query, "key");
        let path = self.query_param(query, "path");

        let message = if let Some(key) = key {
            self.cache.remove(&key).await;
            format!("Purged cache key: {}", key)
        } else if let (Some(domain), Some(path)) = (domain.clone(), path) {
            let key = build_page_cache_key(&domain, &path);
            self.cache.remove(&key).await;
            format!("Purged page cache entry: {}", key)
        } else if let Some(domain) = domain {
            self.cache.purge_by_tag(&format!("domain:{}", domain)).await;
            format!("Purged cache for domain: {}", domain)
        } else if let Some(tag) = tag {
            self.cache.purge_by_tag(&tag).await;
            format!("Purged cache tag: {}", tag)
        } else {
            self.cache.purge_all().await;
            "Purged all cache entries".to_string()
        };

        self.json_response(serde_json::json!({
            "success": true,
            "message": message
        }))
    }

    /// API: Warm cache endpoints (best-effort)
    async fn api_cache_warm(
        &self,
        req: &Request<hyper::body::Incoming>,
    ) -> Result<Response<Full<Bytes>>> {
        let query = req.uri().query().unwrap_or("");
        let urls: Vec<String> = query
            .split('&')
            .filter_map(|part| part.strip_prefix("url="))
            .map(|value| {
                percent_encoding::percent_decode_str(value)
                    .decode_utf8_lossy()
                    .to_string()
            })
            .collect();

        self.json_response(serde_json::json!({
            "success": true,
            "queued_urls": urls.len(),
            "urls": urls,
            "message": "Warmup accepted. Trigger requests to populate cache entries."
        }))
    }

    /// API: Metrics
    fn api_metrics(&self) -> Result<Response<Full<Bytes>>> {
        let metrics = serde_json::json!({
            "requests_total": 0,
            "cache_hits": self.cache.stats()["hits"],
            "cache_misses": self.cache.stats()["misses"],
            "php_available": self.php_pool.is_available(),
        });

        self.json_response(metrics)
    }

    /// API: Worker status
    fn api_workers(&self) -> Result<Response<Full<Bytes>>> {
        let workers = serde_json::json!({
            "http_workers": self.config.worker_threads(),
            "php_workers": if self.php_pool.is_available() {
                self.config.php.workers
            } else {
                0
            },
            "php_stats": self.php_pool.stats(),
        });

        self.json_response(workers)
    }

    /// Find virtual host for request
    fn find_vhost(
        &self,
        req: &Request<hyper::body::Incoming>,
    ) -> (PathBuf, Option<&crate::config::VirtualHostConfig>) {
        let host = req
            .headers()
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("localhost");

        let host = host.split(':').next().unwrap_or(host);

        for vhost in &self.config.virtualhost {
            if vhost.domain == host || vhost.domain == "*" {
                return (PathBuf::from(&vhost.root), Some(vhost));
            }
        }

        (PathBuf::from("/var/www/html"), None)
    }

    /// Resolve path to file system path (with security checks)
    fn resolve_path(&self, doc_root: &Path, path: &str) -> PathBuf {
        let clean_path = path.trim_start_matches('/');
        let decoded = percent_encoding::percent_decode_str(clean_path)
            .decode_utf8_lossy()
            .to_string();

        // Security: prevent directory traversal
        let path = PathBuf::from(&decoded);
        let normalized: PathBuf = path
            .components()
            .filter(|c| !matches!(c, std::path::Component::ParentDir))
            .collect();

        doc_root.join(normalized)
    }

    /// Generate cache key for request
    fn cache_key(&self, req: &Request<hyper::body::Incoming>) -> String {
        let host = req
            .headers()
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("localhost");
        let path = req
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or(req.uri().path());

        build_page_cache_key(host, path)
    }

    fn query_param(&self, query: &str, key: &str) -> Option<String> {
        query.split('&').find_map(|part| {
            let (name, value) = part.split_once('=')?;
            if name == key {
                Some(
                    percent_encoding::percent_decode_str(value)
                        .decode_utf8_lossy()
                        .to_string(),
                )
            } else {
                None
            }
        })
    }

    fn cache_context(
        &self,
        req: &Request<hyper::body::Incoming>,
        path: &str,
        vhost: Option<&crate::config::VirtualHostConfig>,
    ) -> Option<CacheContext> {
        if !self.config.cache.enable || !self.is_cacheable_request(req, path, vhost) {
            return None;
        }

        let host = req
            .headers()
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("localhost");
        let host = host.split(':').next().unwrap_or(host).to_string();

        let ttl = vhost
            .and_then(|v| v.cache.as_ref().map(|c| c.ttl))
            .unwrap_or(self.config.cache.default_ttl);

        Some(CacheContext {
            key: self.cache_key(req),
            domain: host,
            path: path.to_string(),
            ttl: Duration::from_secs(ttl),
        })
    }

    fn is_cacheable_request(
        &self,
        req: &Request<hyper::body::Incoming>,
        path: &str,
        vhost: Option<&crate::config::VirtualHostConfig>,
    ) -> bool {
        if req.method() != Method::GET && req.method() != Method::HEAD {
            return false;
        }
        if req.uri().query().is_some() {
            return false;
        }
        if self.is_authenticated_request(req) {
            return false;
        }

        if let Some(vhost) = vhost {
            if let Some(vhost_cache) = &vhost.cache {
                if !vhost_cache.enable {
                    return false;
                }
                if self.is_excluded_path(path, &vhost_cache.exclude) {
                    return false;
                }
            }
        }

        true
    }

    fn is_authenticated_request(&self, req: &Request<hyper::body::Incoming>) -> bool {
        if req.headers().contains_key("authorization") {
            return true;
        }

        let Some(cookie_header) = req.headers().get("cookie").and_then(|h| h.to_str().ok()) else {
            return false;
        };
        let cookie = cookie_header.to_ascii_lowercase();
        cookie.contains("wordpress_logged_in")
            || cookie.contains("phpsessid")
            || cookie.contains("session")
            || cookie.contains("auth")
            || cookie.contains("token")
            || cookie.contains("woocommerce")
    }

    fn is_excluded_path(&self, path: &str, rules: &[String]) -> bool {
        rules.iter().any(|rule| {
            if let Some(prefix) = rule.strip_suffix('*') {
                path.starts_with(prefix)
            } else {
                path == rule || path.starts_with(&format!("{}/", rule.trim_end_matches('/')))
            }
        })
    }

    fn cached_response(
        &self,
        method: &Method,
        body: &[u8],
        content_type: &str,
    ) -> Result<Response<Full<Bytes>>> {
        let mut builder = Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, content_type)
            .header("Server", crate::SERVER_NAME)
            .header("X-Powered-By", format!("VeloServe/{}", crate::VERSION))
            .header("X-Cache", "HIT");

        if method == Method::HEAD {
            builder = builder.header(CONTENT_LENGTH, body.len().to_string());
            return builder
                .body(Full::new(Bytes::new()))
                .map_err(|e| anyhow!("Failed to build cached HEAD response: {}", e));
        }

        builder
            .body(Full::new(Bytes::from(body.to_vec())))
            .map_err(|e| anyhow!("Failed to build cached response: {}", e))
    }

    async fn finalize_response(
        &self,
        response: Response<Full<Bytes>>,
        cache_context: Option<&CacheContext>,
        method: &Method,
    ) -> Result<Response<Full<Bytes>>> {
        let Some(context) = cache_context else {
            return Ok(response);
        };

        if method != Method::GET {
            return Ok(response);
        }

        if response.status() != StatusCode::OK {
            return Ok(response);
        }

        if response.headers().contains_key(SET_COOKIE) {
            return Ok(response);
        }

        let cache_control = response
            .headers()
            .get(CACHE_CONTROL)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        if cache_control.contains("no-store") || cache_control.contains("private") {
            return Ok(response);
        }

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("text/html; charset=utf-8")
            .to_string();
        if !content_type.to_ascii_lowercase().starts_with("text/html") {
            return Ok(response);
        }

        let (parts, body) = response.into_parts();
        let body = body.collect().await?.to_bytes();
        let body_vec = body.to_vec();

        self.cache
            .set_with_ttl(
                &context.key,
                body_vec,
                &content_type,
                vec![
                    format!("domain:{}", context.domain),
                    format!("path:{}{}", context.domain, context.path),
                ],
                context.ttl,
            )
            .await;

        let mut response = Response::from_parts(parts, Full::new(body));
        response
            .headers_mut()
            .insert("X-Cache", HeaderValue::from_static("MISS"));
        Ok(response)
    }

    // === Response Helpers ===

    fn health_check(&self) -> Result<Response<Full<Bytes>>> {
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain")
            .header("Server", crate::SERVER_NAME)
            .body(Full::new(Bytes::from("OK")))
            .map_err(|e| anyhow!("Failed to build response: {}", e))
    }

    fn not_found(&self) -> Result<Response<Full<Bytes>>> {
        let body = r#"<!DOCTYPE html>
<html>
<head><title>404 Not Found</title></head>
<body>
<h1>404 Not Found</h1>
<p>The requested resource was not found on this server.</p>
<hr>
<p><em>VeloServe</em></p>
</body>
</html>"#;

        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Server", crate::SERVER_NAME)
            .body(Full::new(Bytes::from(body)))
            .map_err(|e| anyhow!("Failed to build response: {}", e))
    }

    fn forbidden(&self, message: &str) -> Result<Response<Full<Bytes>>> {
        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head><title>403 Forbidden</title></head>
<body>
<h1>403 Forbidden</h1>
<p>{}</p>
<hr>
<p><em>VeloServe</em></p>
</body>
</html>"#,
            message
        );

        Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Server", crate::SERVER_NAME)
            .body(Full::new(Bytes::from(body)))
            .map_err(|e| anyhow!("Failed to build response: {}", e))
    }

    fn method_not_allowed(&self) -> Result<Response<Full<Bytes>>> {
        Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .header("Content-Type", "text/plain")
            .header("Server", crate::SERVER_NAME)
            .header("Allow", "GET, HEAD, POST")
            .body(Full::new(Bytes::from("Method Not Allowed")))
            .map_err(|e| anyhow!("Failed to build response: {}", e))
    }

    fn internal_error(&self, message: &str) -> Result<Response<Full<Bytes>>> {
        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head><title>500 Internal Server Error</title></head>
<body>
<h1>500 Internal Server Error</h1>
<p>{}</p>
<hr>
<p><em>VeloServe</em></p>
</body>
</html>"#,
            message
        );

        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Server", crate::SERVER_NAME)
            .body(Full::new(Bytes::from(body)))
            .map_err(|e| anyhow!("Failed to build response: {}", e))
    }

    fn json_response(&self, data: serde_json::Value) -> Result<Response<Full<Bytes>>> {
        let body = serde_json::to_string_pretty(&data)?;

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("Server", crate::SERVER_NAME)
            .body(Full::new(Bytes::from(body)))
            .map_err(|e| anyhow!("Failed to build response: {}", e))
    }
}
