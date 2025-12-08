# Configuration Reference

VeloServe uses TOML configuration files. Default location: `/etc/veloserve/veloserve.toml`

## Full Configuration Example

```toml
# =============================================================================
# VeloServe Configuration
# Documentation: https://veloserve.io/docs/configuration
# =============================================================================

# -----------------------------------------------------------------------------
# Server Settings
# -----------------------------------------------------------------------------
[server]
# IP and port to listen on (required)
listen = "0.0.0.0:8080"

# HTTPS listener (optional, requires TLS config)
# listen_ssl = "0.0.0.0:443"

# Number of worker threads
# Options: "auto" (uses CPU cores), or specific number like "4"
workers = "auto"

# Maximum concurrent connections
max_connections = 10000

# Keep-alive timeout in seconds
keepalive_timeout = 75

# Request timeout in seconds
request_timeout = 60

# Request body size limit (e.g., "10M", "100K", "1G")
max_body_size = "100M"

# Server header (set to empty string to hide)
server_header = "VeloServe"

# Access log path (optional)
# access_log = "/var/log/veloserve/access.log"

# Error log path (optional)
# error_log = "/var/log/veloserve/error.log"

# -----------------------------------------------------------------------------
# TLS/HTTPS Settings
# -----------------------------------------------------------------------------
[tls]
# Enable TLS
enable = false

# Certificate file path
# cert_file = "/etc/veloserve/ssl/cert.pem"

# Private key file path
# key_file = "/etc/veloserve/ssl/key.pem"

# Minimum TLS version: "1.2" or "1.3"
min_version = "1.2"

# ALPN protocols
alpn = ["h2", "http/1.1"]

# -----------------------------------------------------------------------------
# PHP Settings
# -----------------------------------------------------------------------------
[php]
# Enable PHP processing
enable = true

# PHP execution mode: "cgi" or "embed"
# "cgi" - Uses php-cgi binary (default, works everywhere)
# "embed" - Uses embedded PHP SAPI (requires --features php-embed)
mode = "cgi"

# PHP version (for display/logging)
version = "8.3"

# Path to PHP binary
# CGI mode: use php-cgi
# SAPI mode: uses embedded libphp (when compiled with --features php-embed)
binary_path = "/usr/bin/php-cgi"

# Number of PHP worker processes
workers = 4

# PHP memory limit per request
memory_limit = "256M"

# Maximum script execution time in seconds
max_execution_time = 30

# Stack limit for embed SAPI (e.g., "16M", "512M")
# Increase this if you encounter stack overflow errors with complex PHP scripts
embed_stack_limit = "512M"

# -----------------------------------------------------------------------------
# PHP Error Logging
# -----------------------------------------------------------------------------
# Path to PHP error log file
# All PHP errors, warnings, and notices will be logged here
# error_log = "/var/log/veloserve/php_errors.log"

# Display PHP errors in browser output
# WARNING: Set to false in production to avoid exposing sensitive information
display_errors = false

# Custom php.ini settings (passed as -d arguments)
# Note: error_log and display_errors are configured above, don't duplicate them here
ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=128",
    "opcache.max_accelerated_files=10000",
    "upload_max_filesize=64M",
    "post_max_size=64M"
]

# File extensions treated as PHP
extensions = [".php", ".phtml"]

# -----------------------------------------------------------------------------
# Cache Settings
# -----------------------------------------------------------------------------
[cache]
# Enable caching
enable = true

# Cache storage backend: "memory", "disk", or "redis"
storage = "memory"

# Memory cache size limit (for memory backend)
memory_limit = "256M"

# Disk cache directory (for disk backend)
# disk_path = "/var/cache/veloserve"

# Redis connection (for redis backend)
# redis_url = "redis://localhost:6379"

# Default TTL in seconds
default_ttl = 3600

# Cache static assets
cache_static = true

# Static asset TTL
static_ttl = 86400

# Cache control header for browsers
# browser_cache_ttl = 3600

# Gzip cached responses
# compress = true

# -----------------------------------------------------------------------------
# Virtual Host Configuration
# -----------------------------------------------------------------------------
[[virtualhost]]
# Domain name (* for catch-all)
domain = "*"

# Aliases (additional domains)
# aliases = ["www.example.com", "example.org"]

# Document root (required)
root = "/var/www/html"

# Index files (in order of priority)
index = ["index.php", "index.html", "index.htm"]

# Platform optimization: "wordpress", "magento2", "laravel", "generic"
# platform = "generic"

# PHP enabled for this vhost (overrides global)
# php_enable = true

# Custom error pages
# error_pages = { 404 = "/404.html", 500 = "/500.html" }

# Access log for this vhost
# access_log = "/var/log/veloserve/example.com.access.log"

# Rewrite rules (Nginx-style)
# rewrites = [
#     { pattern = "^/old/(.*)$", replacement = "/new/$1", flags = "redirect" }
# ]

# Per-vhost cache settings
[virtualhost.cache]
enable = true
ttl = 3600

# URLs to exclude from caching
exclude = [
    "/wp-admin/*",
    "/wp-login.php",
    "/admin/*",
    "/checkout/*",
    "/cart/*",
    "/my-account/*"
]

# Vary cache by these cookies
# vary_cookies = ["wordpress_logged_in_*"]

# Vary cache by these headers
# vary_headers = ["Accept-Encoding", "Accept-Language"]

# -----------------------------------------------------------------------------
# WordPress Optimization (when platform = "wordpress")
# -----------------------------------------------------------------------------
[wordpress]
# Enable WordPress-specific optimizations
enable = true

# Cache TTL for pages
cache_ttl = 3600

# Separate cache for mobile devices
cache_mobile_separate = false

# Cache logged-in users (not recommended)
cache_logged_in = false

# Cookies that indicate logged-in state
logged_in_cookies = ["wordpress_logged_in_*", "wp-postpass_*", "comment_author_*"]

# URLs to never cache
exclude_urls = [
    "/wp-admin/*",
    "/wp-login.php",
    "/wp-cron.php",
    "/xmlrpc.php",
    "/wp-json/*",
    "/cart/*",
    "/checkout/*",
    "/my-account/*"
]

# Query strings that bypass cache
exclude_query_strings = ["add-to-cart", "removed_item", "wc-ajax"]

# -----------------------------------------------------------------------------
# Magento 2 Optimization (when platform = "magento2")
# -----------------------------------------------------------------------------
[magento2]
# Enable Magento 2 optimizations
enable = false

# Full Page Cache TTL
cache_ttl = 7200

# Enable ESI support for hole punching
esi_support = true

# Customer session cookies
customer_cookies = ["PHPSESSID", "private_content_version", "X-Magento-Vary"]

# Cache tags header
cache_tags_header = "X-Magento-Tags"

# Exclude patterns
exclude_urls = [
    "/admin/*",
    "/checkout/*",
    "/customer/*",
    "/catalog/product_compare/*"
]

# -----------------------------------------------------------------------------
# Security Settings
# -----------------------------------------------------------------------------
[security]
# Hide server version in headers
hide_version = true

# Security headers
headers = [
    { name = "X-Frame-Options", value = "SAMEORIGIN" },
    { name = "X-Content-Type-Options", value = "nosniff" },
    { name = "X-XSS-Protection", value = "1; mode=block" },
    { name = "Referrer-Policy", value = "strict-origin-when-cross-origin" }
]

# Block common attack patterns
block_patterns = [
    "etc/passwd",
    "wp-config.php.bak",
    ".git/",
    ".env"
]

# Rate limiting (requests per second per IP)
# rate_limit = 100

# -----------------------------------------------------------------------------
# Logging Settings
# -----------------------------------------------------------------------------
[logging]
# Log level: "trace", "debug", "info", "warn", "error"
level = "info"

# Log format: "combined", "common", "json"
format = "combined"

# Log to stdout (useful for containers)
stdout = true

# Log file path
# file = "/var/log/veloserve/veloserve.log"

# Rotate logs
# rotate = true
# rotate_size = "100M"
# rotate_keep = 10
```

## Minimal Configuration

```toml
[server]
listen = "0.0.0.0:8080"

[php]
enable = true

[[virtualhost]]
domain = "*"
root = "/var/www/html"
```

## Multiple Virtual Hosts

```toml
[server]
listen = "0.0.0.0:80"

[[virtualhost]]
domain = "example.com"
aliases = ["www.example.com"]
root = "/var/www/example.com"
platform = "wordpress"

[[virtualhost]]
domain = "shop.example.com"
root = "/var/www/magento"
platform = "magento2"

[[virtualhost]]
domain = "*"
root = "/var/www/default"
```

## See Also

- [Environment Variables](environment-variables.md)
- [PHP Configuration](php.md)
- [Caching Guide](caching.md)

