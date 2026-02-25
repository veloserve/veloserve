# Cache Configuration

VeloServe includes a built-in multi-layer cache for pages, objects, and static assets.

## Configuration

```toml
[cache]
enable = true
storage = "memory"
memory_limit = "256M"
default_ttl = 3600
cache_static = true
static_ttl = 86400
```

## Options Reference

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable` | bool | `true` | Enable caching |
| `storage` | string | `"memory"` | Backend: `"memory"`, `"disk"`, or `"redis"` |
| `memory_limit` | string | `"256M"` | Max memory for cache (memory backend) |
| `disk_path` | string | none | Directory for disk cache |
| `redis_url` | string | none | Redis connection URL |
| `default_ttl` | int | `3600` | Default TTL in seconds |
| `cache_static` | bool | `true` | Cache static assets (CSS, JS, images) |
| `static_ttl` | int | `86400` | TTL for static assets |

## Storage Backends

### Memory

Fastest option. Cached data lives in VeloServe's process memory:

```toml
[cache]
storage = "memory"
memory_limit = "1G"
```

### Disk

Persistent across restarts. Good for large caches:

```toml
[cache]
storage = "disk"
disk_path = "/var/cache/veloserve"
```

### Redis

Shared cache across multiple VeloServe instances:

```toml
[cache]
storage = "redis"
redis_url = "redis://localhost:6379"
```

## Per-Vhost Cache Rules

```toml
[virtualhost.cache]
enable = true
ttl = 3600
exclude = [
    "/wp-admin/*",
    "/wp-login.php",
    "/admin/*",
    "/checkout/*",
    "/cart/*",
    "/my-account/*"
]
vary_cookies = ["wordpress_logged_in_*"]
vary_headers = ["Accept-Encoding", "Accept-Language"]
```

## Cache Management

```bash
# View cache stats
veloserve cache stats

# Purge all
veloserve cache purge --all

# Purge by domain
veloserve cache purge --domain example.com

# Purge by URL pattern
veloserve cache purge --pattern "/blog/*"

# Warm cache from sitemap
veloserve cache warm --sitemap https://example.com/sitemap.xml
```
