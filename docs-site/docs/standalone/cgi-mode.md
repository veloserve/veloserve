# CGI Mode

CGI mode is the simplest way to run VeloServe with PHP. It uses the standard `php-cgi` binary and works on any platform without special compilation.

## How It Works

```
HTTP Request → VeloServe → spawn php-cgi → Execute Script → HTTP Response
                  ↑
          Process pool with semaphore
```

VeloServe maintains a pool of PHP worker slots. For each PHP request:

1. A semaphore slot is acquired from the pool
2. A `php-cgi` process is spawned with the CGI environment
3. The request body is written to the process's stdin
4. The response is read from stdout
5. The slot is released back to the pool

This is the same model used by traditional web servers, but VeloServe's Rust async runtime makes the I/O handling significantly more efficient.

## Performance Characteristics

| Metric | Value |
|--------|-------|
| Requests/sec | ~500 |
| Latency | ~50ms |
| Memory | Low (on-demand) |
| CPU overhead | Medium (process spawn) |

CGI mode is best suited for development, low-traffic sites, or environments where you cannot install `libphp-embed`.

## Prerequisites

=== "Ubuntu / Debian"

    ```bash
    sudo apt install php-cgi php-mysql php-curl php-gd php-mbstring php-xml php-zip
    ```

=== "Fedora / RHEL / AlmaLinux"

    ```bash
    sudo dnf install php-cli php-cgi php-mysqlnd php-gd php-mbstring php-xml
    ```

=== "macOS"

    ```bash
    brew install php
    ```

=== "Windows"

    Download PHP from [windows.php.net](https://windows.php.net/download/) and ensure `php-cgi.exe` is in your PATH.

## Building VeloServe for CGI

No special build flags needed — CGI is the default mode:

```bash
git clone https://github.com/veloserve/veloserve.git
cd veloserve
cargo build --release
```

Or download a pre-built binary from the [releases page](https://github.com/veloserve/veloserve/releases).

## Configuration

```toml
[server]
listen = "0.0.0.0:8080"

[php]
enable = true
mode = "cgi"
binary_path = "/usr/bin/php-cgi"
workers = 4
memory_limit = "256M"
max_execution_time = 30

[[virtualhost]]
domain = "*"
root = "/var/www/html"
```

### Key Settings

| Setting | Description | Default |
|---------|-------------|---------|
| `binary_path` | Path to `php-cgi` binary | auto-detected |
| `workers` | Max concurrent PHP processes | `4` |
| `memory_limit` | PHP memory limit per request | `256M` |
| `max_execution_time` | Script timeout in seconds | `30` |
| `extensions` | File extensions treated as PHP | `[".php", ".phtml"]` |

!!! tip "Finding your php-cgi path"
    ```bash
    which php-cgi
    # or
    find / -name php-cgi 2>/dev/null
    ```

### Custom php.ini Settings

Pass custom INI directives via the config:

```toml
[php]
ini_settings = [
    "upload_max_filesize=64M",
    "post_max_size=64M",
    "opcache.enable=1",
    "opcache.memory_consumption=128"
]
```

## Running

```bash
# With config file
veloserve --config veloserve.toml start

# Quick start (no config file)
veloserve start --root /var/www/html --listen 0.0.0.0:8080
```

## CGI Environment Variables

VeloServe sets all standard CGI variables:

| Variable | Example |
|----------|---------|
| `SCRIPT_FILENAME` | `/var/www/html/index.php` |
| `SCRIPT_NAME` | `/index.php` |
| `REQUEST_URI` | `/blog/post?id=1` |
| `PATH_INFO` | `/blog/post` |
| `DOCUMENT_ROOT` | `/var/www/html` |
| `QUERY_STRING` | `id=1&page=2` |
| `REQUEST_METHOD` | `GET`, `POST` |
| `HTTP_HOST` | `example.com` |
| `REMOTE_ADDR` | `192.168.1.1` |
| `SERVER_PORT` | `80` |
| `HTTPS` | `on` or `off` |
| `GATEWAY_INTERFACE` | `CGI/1.1` |
| `SERVER_SOFTWARE` | `VeloServe/1.0.4` |
| `REDIRECT_STATUS` | `200` |

## Clean URLs

VeloServe supports clean URLs (WordPress, Laravel, etc.) automatically. When a requested file does not exist and `index.php` is present, the request is routed through `index.php` with `PATH_INFO` set:

| URL | Script | PATH_INFO |
|-----|--------|-----------|
| `/index.php` | `index.php` | (empty) |
| `/blog/post` | `index.php` | `/blog/post` |
| `/api/users/1` | `index.php` | `/api/users/1` |

## Troubleshooting

### "PHP binary not found"

```
PHP binary not found at "/usr/bin/php-cgi"
```

Install PHP-CGI or set the correct path in your config:

```bash
which php-cgi
# Update binary_path in veloserve.toml
```

### 502 Bad Gateway

This usually means PHP crashed or timed out. Try:

1. Increase `max_execution_time`
2. Increase `memory_limit`
3. Check PHP error logs
4. Test with a simple script: `<?php echo "OK"; ?>`

### Blank Page

PHP is executing but producing no output:

1. Temporarily set `display_errors = true` in your config
2. Check the PHP error log
3. Verify file permissions on the document root
4. Test with: `<?php phpinfo(); ?>`

## Next Steps

- **[SAPI Mode](sapi-mode.md)** — 10-100x faster with embedded PHP
- **[PHP Configuration](../configuration/php.md)** — full PHP config reference
- **[Performance Tuning](../guides/performance.md)** — optimize for production
