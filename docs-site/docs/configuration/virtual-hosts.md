# Virtual Hosts

Virtual hosts allow VeloServe to serve multiple websites from a single server, each with its own domain, document root, and settings.

## Basic Virtual Host

```toml
[[virtualhost]]
domain = "example.com"
root = "/var/www/example.com"
```

## Catch-All

Use `*` as the domain to match all requests that don't match a specific virtual host:

```toml
[[virtualhost]]
domain = "*"
root = "/var/www/default"
```

## Full Options

```toml
[[virtualhost]]
domain = "example.com"
aliases = ["www.example.com", "example.org"]
root = "/var/www/example.com"
index = ["index.php", "index.html", "index.htm"]
platform = "wordpress"
php_enable = true
ssl_certificate = "/path/to/cert.pem"
ssl_certificate_key = "/path/to/key.pem"

[virtualhost.cache]
enable = true
ttl = 3600
exclude = ["/wp-admin/*", "/wp-login.php", "/cart/*"]
```

## Options Reference

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `domain` | string | required | Primary domain name (`*` for catch-all) |
| `aliases` | array | `[]` | Additional domain names |
| `root` | string | required | Document root path |
| `index` | array | `["index.php", "index.html"]` | Index file names in priority order |
| `platform` | string | `"generic"` | CMS optimization: `wordpress`, `magento2`, `laravel`, `generic` |
| `php_enable` | bool | global setting | Override global PHP enable/disable |
| `ssl_certificate` | string | none | Path to SSL certificate for this domain |
| `ssl_certificate_key` | string | none | Path to SSL private key for this domain |
| `access_log` | string | none | Per-vhost access log path |

## Per-Vhost Cache

```toml
[virtualhost.cache]
enable = true
ttl = 3600
exclude = ["/wp-admin/*", "/wp-login.php"]
vary_cookies = ["wordpress_logged_in_*"]
vary_headers = ["Accept-Encoding"]
```

## Multiple Virtual Hosts

```toml
[[virtualhost]]
domain = "blog.example.com"
root = "/var/www/blog"
platform = "wordpress"

[[virtualhost]]
domain = "shop.example.com"
root = "/var/www/shop"
platform = "magento2"

[[virtualhost]]
domain = "*"
root = "/var/www/default"
```

VeloServe matches requests in the order virtual hosts are defined. The first match wins. Place the catch-all (`*`) last.

## Platform Optimizations

Setting `platform` enables CMS-specific behaviors:

| Platform | Optimizations |
|----------|--------------|
| `wordpress` | Smart cache rules, logged-in user detection, WooCommerce exclusions |
| `magento2` | ESI support, cache tags, customer session handling |
| `laravel` | Artisan-friendly routing, session handling |
| `generic` | No CMS-specific behavior |
