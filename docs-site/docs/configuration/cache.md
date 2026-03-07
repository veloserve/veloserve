# Cache Configuration

VeloServe includes a built-in multi-layer cache system. This page focuses on the current page-cache runtime behavior and cache management interfaces.

## Configuration

```toml
[cache]
enable = true
storage = "memory"
memory_limit = "256M"
default_ttl = 3600
disk_path = "/var/cache/veloserve"
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
```

When `virtualhost.cache.enable = false`, page cache is disabled for that vhost.  
The `exclude` list supports exact matches and prefix rules with `*`.

## Runtime Behavior

- Caches only `GET/HEAD` responses with `200` status and `text/html` content type
- Skips requests with query strings
- Skips requests with auth/session cookies (WordPress/PHP session style cookies)
- Skips responses with `Set-Cookie`, `Cache-Control: private`, or `Cache-Control: no-store`
- Adds `X-Cache: HIT` or `X-Cache: MISS` response headers

## Cache Management API

```bash
# Server status
GET /api/v1/status

# Cache config and stats (useful for cPanel/WHM integrations)
GET /api/v1/cache/config
GET /api/v1/cache/stats

# Purge all entries
POST /api/v1/cache/purge

# Purge by domain tag
POST /api/v1/cache/purge?domain=example.com

# Purge one page key (domain + path)
POST /api/v1/cache/purge?domain=example.com&path=/shop

# Purge by custom tag
POST /api/v1/cache/purge?tag=category_5

# Warmup request list (best-effort helper endpoint)
POST /api/v1/cache/warm?url=/&url=/shop&url=/blog
```

## Cache Management CLI

```bash
veloserve cache stats
veloserve cache purge --all
veloserve cache purge --domain example.com
veloserve cache purge --tag category_5
```
