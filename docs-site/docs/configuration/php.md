# PHP Configuration

Complete reference for the `[php]` section of `veloserve.toml`.

## Options

```toml
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
extensions = [".php", ".phtml"]
ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=128",
    "upload_max_filesize=64M",
    "post_max_size=64M"
]
```

## Options Reference

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable` | bool | `true` | Enable PHP processing |
| `mode` | string | `"cgi"` | `"cgi"` or `"embed"` |
| `version` | string | `"8.3"` | PHP version (for display) |
| `binary_path` | string | auto-detected | Path to `php-cgi` binary |
| `workers` | int | `4` | Max concurrent PHP processes/threads |
| `memory_limit` | string | `"256M"` | PHP memory limit per request |
| `max_execution_time` | int | `30` | Script timeout in seconds |
| `embed_stack_limit` | string | `"512M"` | Stack size for SAPI threads |
| `error_log` | string | none | Path to PHP error log |
| `display_errors` | bool | `false` | Show errors in browser |
| `extensions` | array | `[".php", ".phtml"]` | File extensions treated as PHP |
| `ini_settings` | array | `[]` | Custom php.ini directives |

## CGI Mode

```toml
[php]
enable = true
mode = "cgi"
binary_path = "/usr/bin/php-cgi"
workers = 4
```

### cPanel EA-PHP

On cPanel servers, use the EA-PHP binary:

```toml
[php]
binary_path = "/opt/cpanel/ea-php83/root/usr/bin/php-cgi"
```

## SAPI Mode

```toml
[php]
enable = true
mode = "embed"
workers = 16
embed_stack_limit = "512M"
```

SAPI mode requires VeloServe to be compiled with `--features php-embed`.

## Error Logging

```toml
[php]
error_log = "/var/log/veloserve/php_errors.log"
display_errors = false
```

!!! warning
    Always set `display_errors = false` in production to avoid exposing sensitive information.

Create the log directory:

```bash
sudo mkdir -p /var/log/veloserve
sudo chown $USER:$USER /var/log/veloserve
```

## OPcache

Recommended for production:

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

!!! tip
    Set `opcache.validate_timestamps=1` during development so PHP picks up file changes automatically.
