# Security Configuration

VeloServe includes built-in security features to protect your applications.

## Security Headers

```toml
[security]
hide_version = true
headers = [
    { name = "X-Frame-Options", value = "SAMEORIGIN" },
    { name = "X-Content-Type-Options", value = "nosniff" },
    { name = "X-XSS-Protection", value = "1; mode=block" },
    { name = "Referrer-Policy", value = "strict-origin-when-cross-origin" }
]
```

## Pattern Blocking

Block requests matching known attack patterns:

```toml
[security]
block_patterns = [
    "etc/passwd",
    "wp-config.php.bak",
    ".git/",
    ".env",
    ".htaccess",
    "xmlrpc.php"
]
```

Requests matching these patterns receive a `403 Forbidden` response.

## Rate Limiting

```toml
[security]
rate_limit = 100
```

Limits each IP address to the specified number of requests per second.

## Options Reference

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `hide_version` | bool | `true` | Hide VeloServe version in `Server` header |
| `headers` | array | `[]` | Custom response headers added to all responses |
| `block_patterns` | array | `[]` | URL patterns to block with 403 |
| `rate_limit` | int | none | Max requests per second per IP |
