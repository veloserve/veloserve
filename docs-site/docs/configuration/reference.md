# Configuration Reference

VeloServe uses TOML configuration files. The default location is `/etc/veloserve/veloserve.toml`.

## Full Configuration Example

```toml
# Server Settings
[server]
listen = "0.0.0.0:8080"
listen_ssl = "0.0.0.0:443"
workers = "auto"
max_connections = 10000
keepalive_timeout = 75
request_timeout = 60
max_body_size = "100M"
server_header = "VeloServe"

# TLS
[tls]
enable = false
cert_file = "/etc/veloserve/ssl/cert.pem"
key_file = "/etc/veloserve/ssl/key.pem"
min_version = "1.2"
alpn = ["h2", "http/1.1"]

# PHP
[php]
enable = true
mode = "cgi"
version = "8.3"
binary_path = "/usr/bin/php-cgi"
workers = 4
memory_limit = "256M"
max_execution_time = 30
embed_stack_limit = "512M"
error_log = "/var/log/veloserve/php_errors.log"
display_errors = false
ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=128",
]
extensions = [".php", ".phtml"]

# Cache
[cache]
enable = true
storage = "memory"
memory_limit = "256M"
default_ttl = 3600
cache_static = true
static_ttl = 86400

# SSL (global fallback)
[ssl]
cert = "/etc/veloserve/ssl/cert.pem"
key = "/etc/veloserve/ssl/key.pem"

# Virtual Hosts
[[virtualhost]]
domain = "example.com"
aliases = ["www.example.com"]
root = "/var/www/example.com"
index = ["index.php", "index.html"]
platform = "wordpress"
ssl_certificate = "/path/to/cert.pem"
ssl_certificate_key = "/path/to/key.pem"

[virtualhost.cache]
enable = true
ttl = 3600
exclude = ["/wp-admin/*", "/wp-login.php"]

# Security
[security]
hide_version = true
headers = [
    { name = "X-Frame-Options", value = "SAMEORIGIN" },
    { name = "X-Content-Type-Options", value = "nosniff" },
]
block_patterns = ["etc/passwd", ".git/", ".env"]

# Logging
[logging]
level = "info"
format = "combined"
stdout = true
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

## Section Reference

Each configuration section is documented in detail:

- **[Virtual Hosts](virtual-hosts.md)** — domain routing, document roots, per-vhost settings
- **[PHP](php.md)** — CGI/SAPI mode, workers, INI settings
- **[Cache](cache.md)** — caching backends, TTL, exclusions
- **[SSL/TLS](ssl-tls.md)** — certificates, TLS versions, ALPN
- **[Security](security.md)** — headers, rate limiting, pattern blocking
- **[Logging](logging.md)** — log levels, formats, rotation

## Config File Location

| Context | Default Path |
|---------|-------------|
| Standalone | `/etc/veloserve/veloserve.toml` |
| cPanel | `/etc/veloserve/veloserve.toml` |
| Custom | Pass `--config /path/to/file.toml` |

## Testing Configuration

```bash
veloserve config test --config /etc/veloserve/veloserve.toml
```

## Reloading Configuration

Apply config changes without restarting:

```bash
# Via CLI
veloserve config reload

# Via systemd
systemctl reload veloserve

# Via signal
kill -HUP $(pidof veloserve)
```
