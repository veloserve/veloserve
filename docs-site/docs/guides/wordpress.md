# WordPress Guide

This guide covers running WordPress on VeloServe with optimal performance settings.

## Requirements

- VeloServe installed ([Installation Guide](../getting-started/installation.md))
- PHP 8.0+ with required extensions
- MySQL or MariaDB

### PHP Extensions

```bash
# Ubuntu / Debian
sudo apt install php-cgi php-mysql php-curl php-gd php-mbstring \
    php-xml php-zip php-intl php-imagick php-opcache

# AlmaLinux / Rocky
sudo dnf install php-cgi php-mysqlnd php-curl php-gd php-mbstring \
    php-xml php-zip php-intl php-pecl-imagick php-opcache
```

## Configuration

```toml
[server]
listen = "0.0.0.0:80"
workers = "auto"

[php]
enable = true
mode = "cgi"
binary_path = "/usr/bin/php-cgi"
workers = 8
memory_limit = "256M"
max_execution_time = 300
ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=128",
    "opcache.max_accelerated_files=10000",
    "upload_max_filesize=64M",
    "post_max_size=64M"
]

[cache]
enable = true
storage = "memory"
memory_limit = "512M"
default_ttl = 3600

[[virtualhost]]
domain = "example.com"
aliases = ["www.example.com"]
root = "/var/www/example.com"
platform = "wordpress"

[virtualhost.cache]
enable = true
ttl = 3600
exclude = [
    "/wp-admin/*",
    "/wp-login.php",
    "/wp-cron.php",
    "/xmlrpc.php",
    "/wp-json/*",
    "/cart/*",
    "/checkout/*",
    "/my-account/*"
]
```

## WordPress-Specific Settings

### Platform Detection

Setting `platform = "wordpress"` enables:

- Automatic cache exclusions for admin pages and logged-in users
- Cookie-based cache bypassing for `wordpress_logged_in_*` cookies
- WooCommerce-aware cart and checkout exclusions
- Clean URL routing through `index.php`

### WooCommerce

For WooCommerce sites, increase PHP limits:

```toml
[php]
memory_limit = "512M"
max_execution_time = 600
ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=256",
    "upload_max_filesize=128M",
    "post_max_size=128M",
    "max_input_vars=5000"
]
```

### Multisite

For WordPress Multisite, add all domains as aliases:

```toml
[[virtualhost]]
domain = "example.com"
aliases = ["www.example.com", "site2.example.com", "site3.example.com"]
root = "/var/www/example.com"
platform = "wordpress"
```

## Performance Tips

1. **Enable OPcache** — 2-5x improvement in PHP execution
2. **Use SAPI mode** if possible — 10-100x faster than CGI
3. **Enable the built-in cache** — eliminates PHP execution for cached pages
4. **Use a Redis object cache** plugin alongside VeloServe's page cache
5. **Set `opcache.validate_timestamps=0`** in production (use `1` in development)

## Troubleshooting

### Permalinks not working

VeloServe handles clean URLs automatically when `platform = "wordpress"` is set. If permalinks don't work, verify the `.htaccess` file is not required (VeloServe does not read `.htaccess`).

### White screen of death

1. Enable error display temporarily: `display_errors = true`
2. Check PHP error log: `tail -f /var/log/veloserve/php_errors.log`
3. Increase `memory_limit` if seeing "Allowed memory size exhausted"

### Plugin or theme errors

Some plugins expect Apache-specific features. Check:

- `mod_rewrite` rules — VeloServe handles URL routing natively
- `.htaccess` directives — not supported; move rules to VeloServe config
