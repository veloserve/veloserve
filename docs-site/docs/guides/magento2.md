# Magento 2 Guide

This guide covers running Magento 2 on VeloServe with optimized settings.

## Requirements

- VeloServe installed
- PHP 8.1+ with Magento-required extensions
- MySQL 8.0+ or MariaDB 10.6+
- Elasticsearch 7.x or OpenSearch

### PHP Extensions

```bash
# Ubuntu / Debian
sudo apt install php-cgi php-bcmath php-curl php-gd php-intl php-mbstring \
    php-mysql php-soap php-xml php-zip php-opcache php-sodium php-xsl

# AlmaLinux / Rocky
sudo dnf install php-cgi php-bcmath php-curl php-gd php-intl php-mbstring \
    php-mysqlnd php-soap php-xml php-zip php-opcache php-sodium php-xsl
```

## Configuration

```toml
[server]
listen = "0.0.0.0:80"
workers = "auto"
max_body_size = "100M"

[php]
enable = true
mode = "cgi"
workers = 16
memory_limit = "2G"
max_execution_time = 1800
ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=512",
    "opcache.max_accelerated_files=60000",
    "opcache.validate_timestamps=0",
    "realpath_cache_size=10M",
    "realpath_cache_ttl=7200",
    "upload_max_filesize=64M",
    "post_max_size=64M"
]

[cache]
enable = true
storage = "memory"
memory_limit = "1G"
default_ttl = 7200

[[virtualhost]]
domain = "shop.example.com"
root = "/var/www/magento/pub"
platform = "magento2"

[virtualhost.cache]
enable = true
ttl = 7200
exclude = [
    "/admin/*",
    "/checkout/*",
    "/customer/*",
    "/catalog/product_compare/*"
]
```

## Magento-Specific Settings

Setting `platform = "magento2"` enables:

- ESI (Edge Side Includes) support for block-level caching
- Cache tag awareness via `X-Magento-Tags` header
- Customer session cookie handling (`PHPSESSID`, `private_content_version`, `X-Magento-Vary`)
- Admin area exclusions

## Production Deployment

### Static Content

Deploy static content before starting VeloServe:

```bash
cd /var/www/magento
bin/magento setup:static-content:deploy -f
bin/magento cache:flush
```

### Permissions

```bash
find var generated vendor pub/static pub/media app/etc -type f -exec chmod 644 {} \;
find var generated vendor pub/static pub/media app/etc -type d -exec chmod 755 {} \;
```

## Performance Tips

1. **Use SAPI mode** for maximum performance
2. **Set `opcache.validate_timestamps=0`** — Magento's deployment model makes this safe
3. **Increase `realpath_cache_size`** — Magento has deep directory structures
4. **Use Redis** for Magento's session and cache backends
5. **Enable VeloServe's built-in cache** for full-page caching
