# PHP Integration

VeloServe supports two PHP execution modes.

## PHP Modes Comparison

| Feature | CGI Mode | SAPI Mode |
|---------|----------|-----------|
| **Performance** | ~500 req/s | ~10,000 req/s |
| **Latency** | ~50ms | ~1ms |
| **Setup** | Easy | Requires libphp-embed |
| **Process model** | Fork per request | In-process |
| **Memory** | Low (on-demand) | Medium (persistent) |
| **PHP extensions** | All supported | All supported |
| **Build command** | `cargo build` | `cargo build --features php-embed` |

## CGI Mode (Default)

Uses `php-cgi` binary with process pooling.

### Requirements

```bash
# Ubuntu/Debian
sudo apt install php-cgi

# Fedora/RHEL
sudo dnf install php-cli php-cgi

# macOS
brew install php
```

### Configuration

```toml
[php]
enable = true
binary_path = "/usr/bin/php-cgi"
workers = 4
memory_limit = "256M"
max_execution_time = 30
```

### How It Works

```
Request → VeloServe → spawn php-cgi → Execute → Response
                ↑
        Process pool with semaphore
```

VeloServe maintains a pool of PHP worker slots. Each request:
1. Acquires a semaphore slot
2. Spawns `php-cgi` with CGI environment
3. Writes POST body to stdin
4. Reads response from stdout
5. Releases slot

## SAPI Mode (Maximum Performance)

PHP runs embedded inside VeloServe via FFI.

### Requirements

```bash
# Ubuntu/Debian
sudo apt install php-dev libphp-embed libxml2-dev libsodium-dev libargon2-dev

# Fedora/RHEL
sudo dnf install php-devel php-embedded libxml2-devel libsodium-devel

# From source (with embed SAPI)
./configure --enable-embed --with-openssl --with-curl --with-gd
make && sudo make install
```

### Build VeloServe with SAPI

```bash
cargo build --release --features php-embed
```

### Configuration

Same as CGI mode - VeloServe auto-detects SAPI when compiled with the feature.

```toml
[php]
enable = true
workers = 4  # Concurrent PHP executions
memory_limit = "256M"
```

### How It Works

```
Request → VeloServe → FFI call to libphp → Execute → Response
                ↑
           Zero-copy, in-process
```

PHP is initialized once at startup. Each request:
1. Sets up superglobals ($_SERVER, $_GET, $_POST)
2. Calls `php_execute_script()` via FFI
3. Captures output buffer
4. Returns response

## PHP Configuration

### Error Logging

VeloServe provides dedicated configuration options for PHP error logging:

```toml
[php]
# Path to PHP error log file
# All PHP errors, warnings, and notices will be written here
error_log = "/var/log/veloserve/php_errors.log"

# Display errors in browser output
# Set to true for development, false for production
display_errors = false
```

**Important:** Make sure the log directory exists and is writable:

```bash
sudo mkdir -p /var/log/veloserve
sudo chown www-data:www-data /var/log/veloserve
# Or for the user running VeloServe:
sudo chown $USER:$USER /var/log/veloserve
```

### Viewing PHP Logs

```bash
# Watch PHP errors in real-time
tail -f /var/log/veloserve/php_errors.log

# View last 100 errors
tail -100 /var/log/veloserve/php_errors.log

# Search for specific errors
grep -i "fatal" /var/log/veloserve/php_errors.log
```

### php.ini Settings

For additional customization, pass custom PHP INI settings:

```toml
[php]
ini_settings = [
    "upload_max_filesize=64M",
    "post_max_size=64M",
    "max_input_vars=3000",
    "memory_limit=256M",
    "max_execution_time=30",
    "opcache.enable=1",
    "opcache.memory_consumption=128",
    "opcache.max_accelerated_files=10000",
    "opcache.validate_timestamps=0"
]
```

**Note:** The `error_log`, `display_errors`, and `log_errors` settings are automatically configured via the dedicated options above. You don't need to include them in `ini_settings`.

### Common Extensions

For WordPress/Magento, ensure these are installed:

```bash
# Ubuntu/Debian
sudo apt install \
    php-curl \
    php-gd \
    php-intl \
    php-mbstring \
    php-mysql \
    php-xml \
    php-zip \
    php-opcache \
    php-redis \
    php-imagick

# Verify
php -m | grep -E "curl|gd|intl|mbstring|mysql|xml|zip"
```

### OPcache (Recommended)

Enable OPcache for significant performance boost:

```toml
[php]
ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=256",
    "opcache.max_accelerated_files=20000",
    "opcache.validate_timestamps=0",  # Set to 1 in development
    "opcache.revalidate_freq=0",
    "opcache.interned_strings_buffer=16",
    "opcache.fast_shutdown=1"
]
```

## CGI Environment Variables

VeloServe sets all standard CGI environment variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `SCRIPT_FILENAME` | Full path to PHP script | `/var/www/html/index.php` |
| `SCRIPT_NAME` | URI path to script | `/index.php` |
| `REQUEST_URI` | Full request URI | `/blog/post?id=1` |
| `PATH_INFO` | Extra path after script | `/blog/post` |
| `PATH_TRANSLATED` | Filesystem path for PATH_INFO | `/var/www/html/blog/post` |
| `DOCUMENT_ROOT` | Document root | `/var/www/html` |
| `QUERY_STRING` | Query string | `id=1&page=2` |
| `REQUEST_METHOD` | HTTP method | `GET`, `POST` |
| `CONTENT_TYPE` | Request content type | `application/json` |
| `CONTENT_LENGTH` | Request body length | `1234` |
| `HTTP_HOST` | Host header | `example.com` |
| `HTTP_*` | All HTTP headers | Prefixed with `HTTP_` |
| `REMOTE_ADDR` | Client IP | `192.168.1.1` |
| `REMOTE_PORT` | Client port | `54321` |
| `SERVER_NAME` | Server hostname | `example.com` |
| `SERVER_PORT` | Server port | `80` |
| `SERVER_PROTOCOL` | Protocol version | `HTTP/1.1` |
| `HTTPS` | Is HTTPS | `on` or `off` |
| `GATEWAY_INTERFACE` | CGI version | `CGI/1.1` |
| `SERVER_SOFTWARE` | Server name | `VeloServe/1.0.0` |
| `REDIRECT_STATUS` | Required by PHP-CGI | `200` |

## Clean URLs / PATH_INFO

VeloServe supports clean URLs like WordPress/Laravel:

```
/blog/post/123 → index.php with PATH_INFO=/blog/post/123
```

This works automatically when:
1. The requested file doesn't exist
2. `index.php` exists in document root
3. The request isn't for a static file

### Example URLs

| URL | Script | PATH_INFO |
|-----|--------|-----------|
| `/index.php` | `index.php` | (empty) |
| `/blog/post` | `index.php` | `/blog/post` |
| `/api/users/1` | `index.php` | `/api/users/1` |
| `/admin.php/dashboard` | `admin.php` | `/dashboard` |

## Troubleshooting

### PHP not found

```
PHP binary not found at "/usr/bin/php-cgi"
```

**Solution:** Install PHP-CGI or specify correct path:

```bash
# Find php-cgi
which php-cgi

# Update config
[php]
binary_path = "/usr/bin/php-cgi"
```

### Permission denied

```
Failed to spawn PHP: Permission denied
```

**Solution:** Check binary permissions:

```bash
ls -la /usr/bin/php-cgi
sudo chmod +x /usr/bin/php-cgi
```

### 502 Bad Gateway

Usually means PHP crashed or timed out.

**Solutions:**
1. Increase `max_execution_time`
2. Increase `memory_limit`
3. Check PHP error logs
4. Test with simple script: `<?php echo "OK"; ?>`

### Blank page

PHP is executing but output is empty.

**Solutions:**
1. Enable `display_errors` temporarily
2. Check PHP error log
3. Verify file permissions
4. Test with: `<?php phpinfo(); ?>`

## Performance Tips

1. **Use OPcache** - 2-5x faster
2. **Increase workers** for high traffic
3. **Use SAPI mode** for best performance
4. **Enable output buffering**
5. **Disable unused extensions**

```toml
[php]
workers = 16  # Match CPU cores for SAPI mode
ini_settings = [
    "opcache.enable=1",
    "output_buffering=4096",
    "realpath_cache_size=4096K",
    "realpath_cache_ttl=600"
]
```

