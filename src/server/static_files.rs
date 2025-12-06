//! Static File Handler
//!
//! Serves static files like Nginx/Apache/LiteSpeed with:
//! - Proper MIME type detection
//! - ETag and Last-Modified headers
//! - Conditional requests (If-None-Match, If-Modified-Since)
//! - Cache-Control headers based on file type
//! - Content-Length header

use anyhow::{anyhow, Result};
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use std::path::Path;
use std::time::SystemTime;
use tokio::fs::{self, File};
use tokio::io::AsyncReadExt;
use tracing::debug;

/// Handler for serving static files
/// 
/// Implements static file serving similar to Nginx/Apache:
/// - Automatic MIME type detection
/// - ETag generation for cache validation  
/// - Last-Modified headers
/// - Configurable cache control
pub struct StaticFileHandler {
    /// Maximum file size to serve (prevents memory issues)
    max_file_size: u64,
}

impl StaticFileHandler {
    /// Create a new static file handler
    pub fn new() -> Self {
        Self {
            max_file_size: 100 * 1024 * 1024, // 100MB
        }
    }

    /// Serve a static file
    pub async fn serve(&self, path: &Path) -> Result<Response<Full<Bytes>>> {
        // Check if file exists
        if !path.exists() {
            return Err(anyhow!("File not found: {:?}", path));
        }

        // Check if it's a file (not a directory)
        if !path.is_file() {
            return Err(anyhow!("Not a file: {:?}", path));
        }

        // Get file metadata
        let metadata = fs::metadata(path).await?;
        let file_size = metadata.len();

        // Check file size
        if file_size > self.max_file_size {
            return Err(anyhow!("File too large: {} bytes", file_size));
        }

        // Get modification time for Last-Modified and ETag
        let modified = metadata.modified().ok();
        let etag = self.generate_etag(path, file_size, modified);
        let last_modified = modified.map(|t| format_http_date(t));

        // Determine MIME type
        let mime_type = self.guess_mime_type(path);

        debug!(
            "Serving {:?} ({}, {} bytes, etag={})",
            path, mime_type, file_size, etag
        );

        // Read file contents
        let mut file = File::open(path).await?;
        let mut contents = Vec::with_capacity(file_size as usize);
        file.read_to_end(&mut contents).await?;

        // Build response with headers like Nginx/Apache
        let mut builder = Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", mime_type)
            .header("Content-Length", file_size)
            .header("Server", crate::SERVER_NAME)
            .header("Accept-Ranges", "bytes")
            .header("ETag", format!("\"{}\"", etag))
            .header("X-Content-Type-Options", "nosniff");

        // Add Last-Modified header
        if let Some(ref lm) = last_modified {
            builder = builder.header("Last-Modified", lm);
        }

        // Add Cache-Control based on file type
        builder = builder.header("Cache-Control", self.cache_control(mime_type));

        // Add Vary header for encoded content
        builder = builder.header("Vary", "Accept-Encoding");

        builder
            .body(Full::new(Bytes::from(contents)))
            .map_err(|e| anyhow!("Failed to build response: {}", e))
    }

    /// Serve with conditional request support (304 Not Modified)
    pub async fn serve_conditional(
        &self,
        path: &Path,
        if_none_match: Option<&str>,
        if_modified_since: Option<&str>,
    ) -> Result<Response<Full<Bytes>>> {
        // Get file metadata first
        let metadata = fs::metadata(path).await?;
        let file_size = metadata.len();
        let modified = metadata.modified().ok();
        let etag = self.generate_etag(path, file_size, modified);

        // Check If-None-Match (ETag)
        if let Some(client_etag) = if_none_match {
            let client_etag = client_etag.trim_matches('"');
            if client_etag == etag || client_etag == "*" {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .header("Server", crate::SERVER_NAME)
                    .header("ETag", format!("\"{}\"", etag))
                    .body(Full::new(Bytes::new()))
                    .unwrap());
            }
        }

        // Check If-Modified-Since
        if let (Some(ims), Some(file_modified)) = (if_modified_since, modified) {
            if let Ok(client_time) = parse_http_date(ims) {
                if file_modified <= client_time {
                    return Ok(Response::builder()
                        .status(StatusCode::NOT_MODIFIED)
                        .header("Server", crate::SERVER_NAME)
                        .header("ETag", format!("\"{}\"", etag))
                        .body(Full::new(Bytes::new()))
                        .unwrap());
                }
            }
        }

        // Serve the full file
        self.serve(path).await
    }

    /// Generate ETag from file metadata
    fn generate_etag(&self, path: &Path, size: u64, modified: Option<SystemTime>) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        size.hash(&mut hasher);

        if let Some(t) = modified {
            if let Ok(duration) = t.duration_since(SystemTime::UNIX_EPOCH) {
                duration.as_secs().hash(&mut hasher);
            }
        }

        format!("{:x}", hasher.finish())
    }

    /// Guess MIME type from file extension
    fn guess_mime_type(&self, path: &Path) -> &'static str {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            // HTML & Templates
            "html" | "htm" => "text/html; charset=utf-8",
            "xhtml" => "application/xhtml+xml; charset=utf-8",

            // CSS
            "css" => "text/css; charset=utf-8",

            // JavaScript
            "js" | "mjs" => "application/javascript; charset=utf-8",
            "json" => "application/json; charset=utf-8",
            "map" => "application/json",

            // Images
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "ico" => "image/x-icon",
            "avif" => "image/avif",
            "bmp" => "image/bmp",
            "tiff" | "tif" => "image/tiff",

            // Fonts
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "ttf" => "font/ttf",
            "otf" => "font/otf",
            "eot" => "application/vnd.ms-fontobject",

            // Documents
            "pdf" => "application/pdf",
            "xml" => "application/xml",
            "txt" => "text/plain; charset=utf-8",
            "md" => "text/markdown; charset=utf-8",
            "csv" => "text/csv; charset=utf-8",
            "rtf" => "application/rtf",

            // Media - Video
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "ogv" => "video/ogg",
            "avi" => "video/x-msvideo",
            "mov" => "video/quicktime",
            "mkv" => "video/x-matroska",

            // Media - Audio
            "mp3" => "audio/mpeg",
            "ogg" | "oga" => "audio/ogg",
            "wav" => "audio/wav",
            "flac" => "audio/flac",
            "aac" => "audio/aac",
            "m4a" => "audio/mp4",

            // Archives
            "zip" => "application/zip",
            "gz" | "gzip" => "application/gzip",
            "tar" => "application/x-tar",
            "rar" => "application/vnd.rar",
            "7z" => "application/x-7z-compressed",
            "bz2" => "application/x-bzip2",

            // Web Assembly
            "wasm" => "application/wasm",

            // Manifest files
            "webmanifest" => "application/manifest+json",
            "appcache" => "text/cache-manifest",

            // Data formats
            "yaml" | "yml" => "text/yaml",
            "toml" => "text/toml",

            // Source code (for syntax highlighting)
            "php" => "text/x-php",
            "py" => "text/x-python",
            "rb" => "text/x-ruby",
            "rs" => "text/x-rust",
            "go" => "text/x-go",
            "java" => "text/x-java",
            "c" | "h" => "text/x-c",
            "cpp" | "hpp" | "cc" => "text/x-c++",
            "sh" | "bash" => "text/x-shellscript",

            // Default
            _ => "application/octet-stream",
        }
    }

    /// Get appropriate Cache-Control header based on MIME type
    /// Similar to Nginx/Apache defaults
    fn cache_control(&self, mime_type: &str) -> &'static str {
        // Static assets that rarely change - aggressive caching
        if mime_type.starts_with("image/")
            || mime_type.starts_with("font/")
            || mime_type == "application/javascript; charset=utf-8"
            || mime_type == "text/css; charset=utf-8"
            || mime_type == "application/wasm"
        {
            // 1 year cache for static assets (like Nginx)
            "public, max-age=31536000, immutable"
        }
        // HTML files - no caching (let application handle it)
        else if mime_type.starts_with("text/html") {
            "no-cache, no-store, must-revalidate"
        }
        // JSON/API responses - short cache
        else if mime_type == "application/json" || mime_type == "application/json; charset=utf-8" {
            "public, max-age=0, must-revalidate"
        }
        // Media files - moderate caching
        else if mime_type.starts_with("video/") || mime_type.starts_with("audio/") {
            "public, max-age=86400"
        }
        // Default - moderate cache
        else {
            "public, max-age=3600"
        }
    }
}

impl Default for StaticFileHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a SystemTime as an HTTP date (RFC 7231)
fn format_http_date(time: SystemTime) -> String {
    use chrono::{DateTime, Utc};
    
    let datetime: DateTime<Utc> = time.into();
    datetime.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

/// Parse an HTTP date string
fn parse_http_date(s: &str) -> Result<SystemTime> {
    use chrono::{DateTime, Utc};
    
    // Try RFC 7231 format first
    if let Ok(dt) = DateTime::parse_from_str(s, "%a, %d %b %Y %H:%M:%S GMT") {
        return Ok(dt.with_timezone(&Utc).into());
    }
    
    // Try other common formats
    if let Ok(dt) = DateTime::parse_from_rfc2822(s) {
        return Ok(dt.with_timezone(&Utc).into());
    }
    
    Err(anyhow!("Invalid date format"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_types() {
        let handler = StaticFileHandler::new();

        assert_eq!(
            handler.guess_mime_type(Path::new("test.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            handler.guess_mime_type(Path::new("style.css")),
            "text/css; charset=utf-8"
        );
        assert_eq!(
            handler.guess_mime_type(Path::new("app.js")),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(handler.guess_mime_type(Path::new("image.png")), "image/png");
        assert_eq!(handler.guess_mime_type(Path::new("font.woff2")), "font/woff2");
        assert_eq!(
            handler.guess_mime_type(Path::new("unknown.xyz")),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_cache_control() {
        let handler = StaticFileHandler::new();

        // Static assets should have long cache
        assert!(handler.cache_control("image/png").contains("31536000"));
        assert!(handler.cache_control("font/woff2").contains("31536000"));
        
        // HTML should not be cached
        assert!(handler.cache_control("text/html; charset=utf-8").contains("no-cache"));
    }

    #[test]
    fn test_etag_generation() {
        let handler = StaticFileHandler::new();
        
        let etag1 = handler.generate_etag(Path::new("/test.html"), 1000, None);
        let etag2 = handler.generate_etag(Path::new("/test.html"), 1000, None);
        
        // Same inputs should produce same ETag
        assert_eq!(etag1, etag2);
        
        // Different size should produce different ETag
        let etag3 = handler.generate_etag(Path::new("/test.html"), 2000, None);
        assert_ne!(etag1, etag3);
    }
}
