# SAPI Mode (Embedded PHP)

SAPI mode delivers maximum performance by embedding PHP directly inside VeloServe via FFI (Foreign Function Interface). No process spawning, no IPC — PHP runs in-process.

## How It Works

```
HTTP Request → VeloServe → FFI call to libphp → Execute Script → HTTP Response
                  ↑
             Zero-copy, in-process
```

PHP is initialized once at server startup. For each request:

1. Superglobals are set up (`$_SERVER`, `$_GET`, `$_POST`, `$_FILES`, `$_COOKIE`)
2. `php_execute_script()` is called via FFI
3. The output buffer is captured
4. The response is returned to the client

There is no process fork, no socket communication, and no serialization overhead.

## Performance Characteristics

| Metric | CGI Mode | SAPI Mode |
|--------|----------|-----------|
| Requests/sec | ~500 | **~10,000** |
| Latency | ~50ms | **~1ms** |
| Memory | Low | Medium |
| Process model | Fork per request | In-process |
| Setup complexity | Easy | Requires libphp-embed |

## Prerequisites

You need the PHP embed SAPI library (`libphp-embed`) installed on your system.

=== "Ubuntu / Debian"

    ```bash
    sudo apt install php-dev libphp-embed libxml2-dev libsodium-dev libargon2-dev
    ```

=== "Fedora / RHEL / AlmaLinux"

    ```bash
    sudo dnf install php-devel php-embedded libxml2-devel libsodium-devel
    ```

=== "From Source"

    If your distribution does not package the embed SAPI, compile PHP from source:

    ```bash
    wget https://www.php.net/distributions/php-8.3.x.tar.gz
    tar -xzf php-8.3.x.tar.gz && cd php-8.3.x

    ./configure \
        --enable-embed \
        --with-openssl \
        --with-curl \
        --with-gd \
        --with-mysqli \
        --with-pdo-mysql \
        --enable-mbstring \
        --enable-intl \
        --enable-opcache

    make -j$(nproc)
    sudo make install
    ```

## Building VeloServe with SAPI

```bash
git clone https://github.com/veloserve/veloserve.git
cd veloserve
cargo build --release --features php-embed
```

!!! warning "Build requirement"
    The `php-embed` feature requires `libphp` to be discoverable by `pkg-config` or present in standard library paths. If the build fails, ensure `php-config --includes` returns valid paths.

## Configuration

The config is almost identical to CGI mode. VeloServe auto-detects SAPI when compiled with the feature:

```toml
[server]
listen = "0.0.0.0:8080"

[php]
enable = true
workers = 4
memory_limit = "256M"

[[virtualhost]]
domain = "*"
root = "/var/www/html"
```

### SAPI-Specific Settings

| Setting | Description | Default |
|---------|-------------|---------|
| `workers` | Concurrent PHP executions (match CPU cores) | `4` |
| `embed_stack_limit` | Stack size for PHP execution threads | `512M` |

```toml
[php]
enable = true
workers = 16
embed_stack_limit = "512M"
```

!!! tip "Worker count"
    For SAPI mode, set `workers` equal to your CPU core count for optimal throughput. Each worker handles one PHP request at a time.

### OPcache Integration

OPcache works with SAPI mode and provides a significant additional performance boost since compiled bytecode persists across requests:

```toml
[php]
ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=256",
    "opcache.max_accelerated_files=20000",
    "opcache.validate_timestamps=0",
    "opcache.revalidate_freq=0",
    "opcache.interned_strings_buffer=16",
    "opcache.fast_shutdown=1"
]
```

!!! note
    Set `opcache.validate_timestamps=1` during development so file changes are picked up automatically.

## Session Support

SAPI mode fully supports PHP sessions. Sessions are handled via the standard PHP session mechanism (`session_start()`, `$_SESSION`, etc.) — VeloServe does not interfere with session storage.

For file-based sessions, ensure the session save path is writable:

```bash
sudo mkdir -p /var/lib/php/sessions
sudo chown www-data:www-data /var/lib/php/sessions
```

## WordPress & Magento Compatibility

SAPI mode is fully compatible with:

- **WordPress** — including WooCommerce, multisite, and all major plugins
- **Magento 2** — including full page cache and ESI support
- **Laravel** — Artisan, queues, and all framework features
- **Drupal, Joomla** — and other PHP applications

## Error Logging

```toml
[php]
error_log = "/var/log/veloserve/php_errors.log"
display_errors = false
```

Make sure the log directory exists:

```bash
sudo mkdir -p /var/log/veloserve
sudo chown $USER:$USER /var/log/veloserve
```

## Troubleshooting

### Build fails: "cannot find libphp"

Ensure the PHP embed library is installed and discoverable:

```bash
# Check if php-config is available
php-config --includes
php-config --libs

# Check for libphp
find /usr -name "libphp*" 2>/dev/null
```

### Stack overflow with complex scripts

Increase the embed stack limit:

```toml
[php]
embed_stack_limit = "1G"
```

### Extensions not loading

Ensure extensions are installed for the same PHP version used by the embed SAPI:

```bash
php -m        # System PHP modules
php -i | grep "Loaded Configuration File"
```

## Next Steps

- **[CGI Mode](cgi-mode.md)** — simpler alternative for development
- **[PHP Configuration](../configuration/php.md)** — full PHP config reference
- **[Performance Tuning](../guides/performance.md)** — optimize for production
- **[WordPress Guide](../guides/wordpress.md)** — WordPress-specific setup
