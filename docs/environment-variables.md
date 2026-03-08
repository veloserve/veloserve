# Environment Variables

VeloServe can be configured using environment variables. These override config file settings.

## Server Variables

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `VELOSERVE_LISTEN` | Listen address | `0.0.0.0:8080` | `127.0.0.1:3000` |
| `VELOSERVE_LISTEN_SSL` | HTTPS listen address | - | `0.0.0.0:443` |
| `VELOSERVE_WORKERS` | Worker threads | `auto` | `4` |
| `VELOSERVE_MAX_CONNECTIONS` | Max concurrent connections | `10000` | `50000` |
| `VELOSERVE_KEEPALIVE_TIMEOUT` | Keep-alive timeout (seconds) | `75` | `120` |
| `VELOSERVE_REQUEST_TIMEOUT` | Request timeout (seconds) | `60` | `30` |
| `VELOSERVE_MAX_BODY_SIZE` | Max request body size | `100M` | `1G` |

## PHP Variables

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `VELOSERVE_PHP_ENABLE` | Enable PHP | `true` | `false` |
| `VELOSERVE_PHP_BINARY` | PHP binary path | auto-detected | `/usr/bin/php-cgi` |
| `VELOSERVE_PHP_WORKERS` | PHP worker processes | `4` | `16` |
| `VELOSERVE_PHP_MEMORY_LIMIT` | PHP memory limit | `256M` | `512M` |
| `VELOSERVE_PHP_MAX_EXECUTION_TIME` | Script timeout (seconds) | `30` | `300` |
| `VELOSERVE_PHP_UPLOAD_MAX_FILESIZE` | Upload size limit | `64M` | `256M` |
| `VELOSERVE_PHP_POST_MAX_SIZE` | POST data limit | `64M` | `256M` |

## Cache Variables

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `VELOSERVE_CACHE_ENABLE` | Enable caching | `true` | `false` |
| `VELOSERVE_CACHE_STORAGE` | Storage backend | `memory` | `redis` |
| `VELOSERVE_CACHE_MEMORY_LIMIT` | Memory cache size | `256M` | `1G` |
| `VELOSERVE_CACHE_DISK_PATH` | Disk cache path | - | `/var/cache/veloserve` |
| `VELOSERVE_CACHE_REDIS_URL` | Redis connection | - | `redis://localhost:6379` |
| `VELOSERVE_CACHE_DEFAULT_TTL` | Default TTL (seconds) | `3600` | `7200` |

## TLS Variables

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `VELOSERVE_TLS_ENABLE` | Enable TLS | `false` | `true` |
| `VELOSERVE_TLS_CERT_FILE` | Certificate path | - | `/etc/ssl/cert.pem` |
| `VELOSERVE_TLS_KEY_FILE` | Private key path | - | `/etc/ssl/key.pem` |
| `VELOSERVE_TLS_MIN_VERSION` | Minimum TLS version | `1.2` | `1.3` |

## Logging Variables

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `VELOSERVE_LOG_LEVEL` | Log level | `info` | `debug` |
| `VELOSERVE_LOG_FORMAT` | Log format | `combined` | `json` |
| `VELOSERVE_LOG_FILE` | Log file path | stdout | `/var/log/veloserve.log` |
| `RUST_LOG` | Rust logging (detailed) | - | `veloserve=debug` |

## Virtual Host Variables

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `VELOSERVE_ROOT` | Document root | - | `/var/www/html` |
| `VELOSERVE_INDEX` | Index files (comma-separated) | `index.php,index.html` | `index.php` |
| `VELOSERVE_PLATFORM` | Platform type | `generic` | `wordpress` |

## Installer Variables

Used by `install.sh`:

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `VELOSERVE_VERSION` | Version to install | `latest` | `v1.0.5` |
| `VELOSERVE_INSTALL_DIR` | Installation directory | `/usr/local/bin` | `/opt/veloserve` |

## Usage Examples

### Command Line

```bash
# Set variables before running
export VELOSERVE_LISTEN="0.0.0.0:3000"
export VELOSERVE_PHP_WORKERS=8
export VELOSERVE_CACHE_ENABLE=true
veloserve --config veloserve.toml

# Or inline
VELOSERVE_LISTEN="0.0.0.0:3000" veloserve start --root /var/www
```

### Docker

```dockerfile
FROM veloserve/veloserve:latest

ENV VELOSERVE_LISTEN="0.0.0.0:8080"
ENV VELOSERVE_PHP_ENABLE="true"
ENV VELOSERVE_PHP_WORKERS="8"
ENV VELOSERVE_CACHE_ENABLE="true"
ENV VELOSERVE_CACHE_STORAGE="memory"
ENV VELOSERVE_CACHE_MEMORY_LIMIT="512M"

COPY ./html /var/www/html
CMD ["veloserve", "start", "--root", "/var/www/html"]
```

### Docker Compose

```yaml
version: '3.8'
services:
  web:
    image: veloserve/veloserve:latest
    ports:
      - "8080:8080"
    environment:
      VELOSERVE_LISTEN: "0.0.0.0:8080"
      VELOSERVE_PHP_ENABLE: "true"
      VELOSERVE_PHP_WORKERS: "4"
      VELOSERVE_PHP_MEMORY_LIMIT: "256M"
      VELOSERVE_CACHE_ENABLE: "true"
      VELOSERVE_CACHE_REDIS_URL: "redis://redis:6379"
      VELOSERVE_LOG_LEVEL: "info"
    volumes:
      - ./html:/var/www/html
    depends_on:
      - redis

  redis:
    image: redis:alpine
```

### Systemd Service

```ini
# /etc/systemd/system/veloserve.service
[Unit]
Description=VeloServe Web Server
After=network.target

[Service]
Type=simple
User=www-data
Group=www-data

Environment="VELOSERVE_LISTEN=0.0.0.0:80"
Environment="VELOSERVE_PHP_ENABLE=true"
Environment="VELOSERVE_PHP_WORKERS=8"
Environment="VELOSERVE_CACHE_ENABLE=true"
Environment="VELOSERVE_LOG_LEVEL=info"

ExecStart=/usr/local/bin/veloserve --config /etc/veloserve/veloserve.toml
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: veloserve
spec:
  replicas: 3
  selector:
    matchLabels:
      app: veloserve
  template:
    metadata:
      labels:
        app: veloserve
    spec:
      containers:
      - name: veloserve
        image: veloserve/veloserve:latest
        ports:
        - containerPort: 8080
        env:
        - name: VELOSERVE_LISTEN
          value: "0.0.0.0:8080"
        - name: VELOSERVE_PHP_ENABLE
          value: "true"
        - name: VELOSERVE_PHP_WORKERS
          value: "4"
        - name: VELOSERVE_CACHE_ENABLE
          value: "true"
        - name: VELOSERVE_CACHE_REDIS_URL
          valueFrom:
            secretKeyRef:
              name: redis-secret
              key: url
        resources:
          limits:
            memory: "512Mi"
            cpu: "500m"
```

## Priority Order

Configuration values are resolved in this order (highest priority first):

1. **Command-line arguments** (`--listen`, `--root`, etc.)
2. **Environment variables** (`VELOSERVE_*`)
3. **Config file** (`veloserve.toml`)
4. **Default values**

## Debug Configuration

To see the final resolved configuration:

```bash
# Show config with env vars applied
RUST_LOG=debug veloserve config test

# Print effective settings
veloserve config show-effective
```

