# Environment Variables

VeloServe can be configured using environment variables. These override config file settings.

## Priority Order

1. **Command-line arguments** (highest priority)
2. **Environment variables**
3. **Config file** (`veloserve.toml`)
4. **Default values**

## Server Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `VELOSERVE_LISTEN` | Listen address | `0.0.0.0:8080` |
| `VELOSERVE_LISTEN_SSL` | HTTPS listen address | — |
| `VELOSERVE_WORKERS` | Worker threads | `auto` |
| `VELOSERVE_MAX_CONNECTIONS` | Max concurrent connections | `10000` |
| `VELOSERVE_KEEPALIVE_TIMEOUT` | Keep-alive timeout (seconds) | `75` |
| `VELOSERVE_REQUEST_TIMEOUT` | Request timeout (seconds) | `60` |
| `VELOSERVE_MAX_BODY_SIZE` | Max request body size | `100M` |

## PHP Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `VELOSERVE_PHP_ENABLE` | Enable PHP | `true` |
| `VELOSERVE_PHP_BINARY` | PHP binary path | auto-detected |
| `VELOSERVE_PHP_WORKERS` | PHP worker count | `4` |
| `VELOSERVE_PHP_MEMORY_LIMIT` | PHP memory limit | `256M` |
| `VELOSERVE_PHP_MAX_EXECUTION_TIME` | Script timeout (seconds) | `30` |

## Cache Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `VELOSERVE_CACHE_ENABLE` | Enable caching | `true` |
| `VELOSERVE_CACHE_STORAGE` | Storage backend | `memory` |
| `VELOSERVE_CACHE_MEMORY_LIMIT` | Memory cache size | `256M` |
| `VELOSERVE_CACHE_REDIS_URL` | Redis connection | — |
| `VELOSERVE_CACHE_DEFAULT_TTL` | Default TTL (seconds) | `3600` |

## TLS Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `VELOSERVE_TLS_ENABLE` | Enable TLS | `false` |
| `VELOSERVE_TLS_CERT_FILE` | Certificate path | — |
| `VELOSERVE_TLS_KEY_FILE` | Private key path | — |

## Logging Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `VELOSERVE_LOG_LEVEL` | Log level | `info` |
| `VELOSERVE_LOG_FORMAT` | Log format | `combined` |
| `VELOSERVE_LOG_FILE` | Log file path | stdout |
| `RUST_LOG` | Rust tracing level | — |

## Virtual Host Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `VELOSERVE_ROOT` | Document root | — |
| `VELOSERVE_INDEX` | Index files (comma-separated) | `index.php,index.html` |
| `VELOSERVE_PLATFORM` | Platform type | `generic` |

## Usage Examples

### Command Line

```bash
VELOSERVE_LISTEN="0.0.0.0:3000" VELOSERVE_PHP_WORKERS=8 veloserve start --root /var/www
```

### Docker Compose

```yaml
services:
  web:
    image: veloserve/veloserve:latest
    environment:
      VELOSERVE_LISTEN: "0.0.0.0:8080"
      VELOSERVE_PHP_ENABLE: "true"
      VELOSERVE_PHP_WORKERS: "4"
      VELOSERVE_CACHE_ENABLE: "true"
      VELOSERVE_CACHE_REDIS_URL: "redis://redis:6379"
```

### Systemd

```ini
[Service]
Environment="VELOSERVE_LISTEN=0.0.0.0:80"
Environment="VELOSERVE_PHP_WORKERS=8"
Environment="VELOSERVE_LOG_LEVEL=info"
```
