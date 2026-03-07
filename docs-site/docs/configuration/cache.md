# Cache Configuration

VeloServe includes a built-in multi-layer cache system. This page focuses on the current page-cache runtime behavior and cache management interfaces.

## Configuration

```toml
[cache]
enable = true
l1_enabled = true
l2_enabled = true
storage = "memory"
memory_limit = "256M"
default_ttl = 3600
disk_path = "/var/cache/veloserve"
warm_enabled = true
warm_schedule_secs = 0
warm_max_queue_size = 2048
warm_max_concurrency = 4
warm_request_timeout_ms = 5000
warm_max_retries = 2
warm_retry_backoff_ms = 250
warm_dedupe_window_secs = 120
warm_batch_size = 64
```

## Options Reference

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable` | bool | `true` | Enable caching |
| `l1_enabled` | bool | `true` | Enable L1 in-process memory cache |
| `l2_enabled` | bool | `true` | Enable L2 persistent cache layer |
| `storage` | string | `"memory"` | Backend: `"memory"`, `"disk"`, or `"redis"` |
| `memory_limit` | string | `"256M"` | Max memory for cache (memory backend) |
| `disk_path` | string | none | Directory for disk cache |
| `redis_url` | string | none | Redis connection URL |
| `default_ttl` | int | `3600` | Default TTL in seconds |
| `warm_enabled` | bool | `true` | Enable warm queue + workers |
| `warm_schedule_secs` | int | `0` | Deterministic scheduled warm interval (seconds) |
| `warm_max_queue_size` | int | `2048` | Warm target queue capacity |
| `warm_max_concurrency` | int | `4` | Concurrent warm workers |
| `warm_request_timeout_ms` | int | `5000` | Timeout per warm request |
| `warm_max_retries` | int | `2` | Retry attempts per failed target |
| `warm_retry_backoff_ms` | int | `250` | Base exponential backoff |
| `warm_dedupe_window_secs` | int | `120` | Duplicate target suppression window |
| `warm_batch_size` | int | `64` | Max deterministic targets per run |

## Storage Backends

### Layer Model

VeloServe uses a read-through/write-through flow:

- L1: in-process memory cache (fast path)
- L2: persistent cache (disk backend for now; redis reserved)

On read: L1 miss falls back to L2, and L2 hits are promoted to L1.  
On write: entries are written to both enabled layers.

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

# Warmup request list
POST /api/v1/cache/warm?url=/&url=/shop&url=/blog

# Warm with JSON payload
POST /api/v1/cache/warm
# { "urls": ["/", "/shop"], "domain": "example.com", "trigger": "manual" }
```

## Cache Management CLI

```bash
veloserve cache stats
veloserve cache purge --all
veloserve cache purge --domain example.com
veloserve cache purge --tag category_5
veloserve cache warm --url https://example.com/
veloserve cache warm --urls warm-targets.txt --api http://127.0.0.1:8080
veloserve cache warm --deterministic --api http://127.0.0.1:8080
```
