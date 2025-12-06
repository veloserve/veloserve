# PHP Extensions Guide for VeloServe

VeloServe uses the system PHP installation to execute PHP scripts. This guide explains how to install and configure PHP extensions for WordPress, Magento 2, and other applications.

## Table of Contents

- [How VeloServe Uses PHP](#how-veloserve-uses-php)
- [Pre-installed Extensions](#pre-installed-extensions)
- [WordPress Requirements](#wordpress-requirements)
- [Magento 2 Requirements](#magento-2-requirements)
- [Installing PHP Extensions](#installing-php-extensions)
- [Configuring PHP for VeloServe](#configuring-php-for-veloserve)
- [Custom PHP Settings](#custom-php-settings)

---

## How VeloServe Uses PHP

Unlike Nginx (which requires PHP-FPM) or Apache (with mod_php), VeloServe has **integrated PHP support**. It executes PHP scripts directly using the system PHP CLI binary, eliminating the overhead of separate PHP process management.

**Benefits:**
- No PHP-FPM configuration needed
- Lower memory overhead
- Simpler deployment
- Same PHP extensions work automatically

**How it works:**
1. VeloServe detects PHP files by extension (`.php`)
2. Executes using the configured PHP binary (default: `/usr/bin/php`)
3. Sets CGI environment variables (like Nginx + PHP-FPM)
4. Returns the output as HTTP response

---

## Pre-installed Extensions

A standard VeloServe setup for WordPress/Magento should have these extensions:

| Extension | Purpose | WordPress | Magento 2 |
|-----------|---------|:---------:|:---------:|
| `bcmath` | Arbitrary precision math | - | ✅ Required |
| `curl` | HTTP client | ✅ Required | ✅ Required |
| `dom` | DOM manipulation | ✅ Required | ✅ Required |
| `gd` | Image processing | ✅ Required | ✅ Required |
| `intl` | Internationalization | ✅ Recommended | ✅ Required |
| `json` | JSON encoding | ✅ Required | ✅ Required |
| `mbstring` | Multibyte strings | ✅ Required | ✅ Required |
| `mysqli` | MySQL database | ✅ Required | - |
| `pdo_mysql` | MySQL PDO driver | ✅ Required | ✅ Required |
| `opcache` | Bytecode cache | ✅ Recommended | ✅ Required |
| `soap` | SOAP protocol | - | ✅ Required |
| `xml` | XML parsing | ✅ Required | ✅ Required |
| `zip` | ZIP archives | ✅ Required | ✅ Required |

---

## WordPress Requirements

### Required Extensions

```bash
# Ubuntu/Debian
sudo apt install php php-cli php-mysql php-curl php-gd php-mbstring php-xml php-zip

# RHEL/Rocky/AlmaLinux
sudo dnf install php php-cli php-mysqlnd php-curl php-gd php-mbstring php-xml php-zip
```

### Recommended Extensions (for full functionality)

```bash
# Ubuntu/Debian
sudo apt install php-intl php-imagick php-opcache php-redis php-memcached

# RHEL/Rocky/AlmaLinux
sudo dnf install php-intl php-pecl-imagick php-opcache php-pecl-redis php-pecl-memcached
```

### WordPress-specific Extensions

| Extension | Purpose |
|-----------|---------|
| `imagick` | Better image handling (thumbnails, WebP) |
| `redis` | Object caching with Redis |
| `memcached` | Object caching with Memcached |
| `exif` | Image metadata reading |
| `fileinfo` | MIME type detection |

---

## Magento 2 Requirements

### Required Extensions

```bash
# Ubuntu/Debian
sudo apt install php php-cli php-bcmath php-curl php-gd php-intl php-mbstring \
    php-mysql php-soap php-xml php-zip php-opcache

# RHEL/Rocky/AlmaLinux
sudo dnf install php php-cli php-bcmath php-curl php-gd php-intl php-mbstring \
    php-mysqlnd php-soap php-xml php-zip php-opcache
```

### Additional Magento Extensions

```bash
# Ubuntu/Debian
sudo apt install php-sodium php-xsl php-sockets

# For Elasticsearch
sudo apt install php-json
```

### Magento 2 Requirements Summary

| Extension | Magento 2.4.x |
|-----------|:-------------:|
| `bcmath` | Required |
| `ctype` | Required |
| `curl` | Required |
| `dom` | Required |
| `fileinfo` | Required |
| `gd` | Required |
| `hash` | Required |
| `iconv` | Required |
| `intl` | Required |
| `json` | Required |
| `libxml` | Required |
| `mbstring` | Required |
| `openssl` | Required |
| `pcre` | Required |
| `pdo_mysql` | Required |
| `simplexml` | Required |
| `soap` | Required |
| `sockets` | Required |
| `sodium` | Required |
| `tokenizer` | Required |
| `xmlwriter` | Required |
| `xsl` | Required |
| `zip` | Required |

---

## Installing PHP Extensions

### Ubuntu/Debian

```bash
# Update package list
sudo apt update

# Install individual extension
sudo apt install php-<extension>

# Example: Install multiple extensions
sudo apt install php-gd php-curl php-mbstring php-xml php-zip php-intl

# Install from PECL (for extensions not in apt)
sudo apt install php-pear php-dev
sudo pecl install redis
echo "extension=redis.so" | sudo tee /etc/php/8.3/cli/conf.d/20-redis.ini
```

### RHEL/Rocky/AlmaLinux

```bash
# Enable EPEL and Remi repositories
sudo dnf install epel-release
sudo dnf install https://rpms.remirepo.net/enterprise/remi-release-9.rpm
sudo dnf module enable php:remi-8.3

# Install extensions
sudo dnf install php-<extension>

# Example: Install multiple extensions
sudo dnf install php-gd php-curl php-mbstring php-xml php-zip php-intl
```

### From PECL (Any Distribution)

```bash
# Install PECL
# Ubuntu: sudo apt install php-pear php-dev
# RHEL: sudo dnf install php-pear php-devel

# Install extension
sudo pecl install <extension>

# Enable extension
echo "extension=<extension>.so" | sudo tee /etc/php/8.3/cli/conf.d/20-<extension>.ini

# Example: Install Redis extension
sudo pecl install redis
echo "extension=redis.so" | sudo tee /etc/php/8.3/cli/conf.d/20-redis.ini
```

### Verify Installation

```bash
# List all installed extensions
php -m

# Check specific extension
php -m | grep -i redis

# Get detailed info
php -i | grep -i redis

# Or create a PHP file and access via VeloServe
echo "<?php phpinfo(); ?>" > /var/www/html/info.php
# Then visit http://localhost:8080/info.php
```

---

## Configuring PHP for VeloServe

### VeloServe Configuration

Edit your `veloserve.toml`:

```toml
[php]
enable = true
version = "8.3"
binary_path = "/usr/bin/php"  # Path to PHP binary
workers = 4                    # Concurrent PHP processes
memory_limit = "256M"         # PHP memory limit
max_execution_time = 30       # Max script runtime (seconds)

# Additional PHP ini settings
ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=128",
    "opcache.max_accelerated_files=10000"
]
```

### PHP CLI Configuration

VeloServe uses PHP CLI, so configure `/etc/php/8.3/cli/php.ini`:

```ini
; Memory and execution limits
memory_limit = 256M
max_execution_time = 30
max_input_time = 60

; File uploads
upload_max_filesize = 64M
post_max_size = 64M
max_file_uploads = 20

; OPcache (highly recommended)
opcache.enable=1
opcache.memory_consumption=128
opcache.interned_strings_buffer=8
opcache.max_accelerated_files=10000
opcache.revalidate_freq=2
opcache.fast_shutdown=1

; Error handling
display_errors = Off
log_errors = On
error_log = /var/log/php/error.log

; Security
expose_php = Off
allow_url_fopen = On
allow_url_include = Off
```

---

## Custom PHP Settings

### Per-Site Settings via veloserve.toml

```toml
[php]
enable = true
binary_path = "/usr/bin/php"
memory_limit = "512M"  # Override for memory-hungry sites

# Custom ini settings
ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=256",
    "session.gc_maxlifetime=7200",
    "date.timezone=America/New_York"
]
```

### For WordPress

```toml
[php]
memory_limit = "256M"
max_execution_time = 300

ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=128",
    "upload_max_filesize=64M",
    "post_max_size=64M"
]
```

### For Magento 2

```toml
[php]
memory_limit = "2G"
max_execution_time = 1800

ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=512",
    "opcache.max_accelerated_files=60000",
    "opcache.validate_timestamps=0",
    "realpath_cache_size=10M",
    "realpath_cache_ttl=7200"
]
```

---

## Troubleshooting

### Check if Extension is Loaded

```bash
# Via command line
php -m | grep <extension>

# Via VeloServe
curl http://localhost:8080/info.php | grep <extension>
```

### Extension Not Found

1. **Check installation:**
   ```bash
   dpkg -l | grep php-<extension>  # Debian/Ubuntu
   rpm -qa | grep php-<extension>  # RHEL/Rocky
   ```

2. **Check ini file exists:**
   ```bash
   ls /etc/php/8.3/cli/conf.d/ | grep <extension>
   ```

3. **Enable extension manually:**
   ```bash
   echo "extension=<extension>.so" | sudo tee /etc/php/8.3/cli/conf.d/20-<extension>.ini
   ```

### VeloServe Not Using New Extension

Restart VeloServe after installing new extensions:

```bash
# If running as service
sudo systemctl restart veloserve

# Or kill and restart
pkill veloserve
veloserve --config /etc/veloserve/veloserve.toml
```

---

## Quick Reference Commands

```bash
# List all PHP extensions
php -m

# Check PHP version
php -v

# Show PHP configuration
php -i

# Check specific setting
php -i | grep memory_limit

# Find PHP binary location
which php

# Find PHP ini location
php --ini

# Validate PHP syntax
php -l script.php
```

---

## See Also

- [WordPress Server Requirements](https://wordpress.org/about/requirements/)
- [Magento 2 System Requirements](https://devdocs.magento.com/guides/v2.4/install-gde/system-requirements.html)
- [PHP Manual - Extensions](https://www.php.net/manual/en/extensions.php)

