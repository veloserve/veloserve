# Performance Tuning

This guide covers optimizing VeloServe for maximum throughput and minimum latency.

## Quick Wins

1. **Enable OPcache** — immediate 2-5x PHP improvement
2. **Use SAPI mode** — 10-100x faster than CGI
3. **Enable the built-in cache** — eliminates PHP for repeated requests
4. **Match workers to CPU cores** — optimal thread utilization

## Server Tuning

```toml
[server]
workers = "auto"           # Uses all CPU cores
max_connections = 10000    # Increase for high-traffic sites
keepalive_timeout = 75     # Keep connections alive
request_timeout = 60       # Prevent slow clients from blocking
```

### Worker Count

- **`"auto"`** — uses the number of CPU cores (recommended)
- Set a specific number if you want to reserve cores for other processes

## PHP Tuning

### CGI Mode

```toml
[php]
workers = 16              # Match to CPU cores
memory_limit = "256M"
max_execution_time = 30
```

### SAPI Mode (Maximum Performance)

```toml
[php]
mode = "embed"
workers = 16              # Concurrent PHP threads
embed_stack_limit = "512M"
ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=256",
    "opcache.max_accelerated_files=20000",
    "opcache.validate_timestamps=0",
    "opcache.interned_strings_buffer=16",
    "opcache.fast_shutdown=1",
    "output_buffering=4096",
    "realpath_cache_size=4096K",
    "realpath_cache_ttl=600"
]
```

## Cache Tuning

```toml
[cache]
enable = true
storage = "memory"
memory_limit = "1G"      # Larger = more pages cached
default_ttl = 3600
cache_static = true
static_ttl = 86400       # 24 hours for static assets
```

### Cache Hit Rate

Monitor your cache performance:

```bash
veloserve cache stats
```

Aim for a hit rate above 90%. If lower:

- Increase `memory_limit`
- Review cache exclusion rules
- Increase TTL where appropriate

## Operating System Tuning

### File Descriptors

Increase the file descriptor limit for high-concurrency:

```bash
# /etc/security/limits.conf
* soft nofile 65536
* hard nofile 65536
```

Or in the systemd service:

```ini
[Service]
LimitNOFILE=65536
```

### TCP Tuning

```bash
# /etc/sysctl.conf
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 65535
net.ipv4.tcp_tw_reuse = 1
net.core.netdev_max_backlog = 65535
```

Apply with:

```bash
sudo sysctl -p
```

## Benchmarking

Use `wrk` or `hey` to benchmark:

```bash
# Install wrk
sudo apt install wrk

# Benchmark static file
wrk -t4 -c100 -d30s http://localhost:8080/

# Benchmark PHP
wrk -t4 -c100 -d30s http://localhost:8080/index.php

# Using hey (Go-based)
hey -n 10000 -c 100 http://localhost:8080/
```

## Comparison Benchmarks

| Setup | Req/sec | Latency (p50) | Latency (p99) |
|-------|---------|---------------|----------------|
| VeloServe SAPI + cache | ~50,000 | <1ms | ~5ms |
| VeloServe SAPI no cache | ~10,000 | ~1ms | ~10ms |
| VeloServe CGI + cache | ~20,000 | <1ms | ~5ms |
| VeloServe CGI no cache | ~500 | ~50ms | ~200ms |
| Nginx + PHP-FPM | ~2,000 | ~10ms | ~50ms |
| Apache + mod_php | ~1,500 | ~15ms | ~75ms |

Results measured on a 4-core server with a simple WordPress page.
