# PHP Extensions

VeloServe uses the system PHP installation. This guide covers installing extensions for WordPress, Magento 2, and other applications.

## Extension Requirements

| Extension | WordPress | Magento 2 | Laravel |
|-----------|:---------:|:---------:|:-------:|
| `bcmath` | — | Required | — |
| `curl` | Required | Required | Required |
| `dom` | Required | Required | — |
| `gd` | Required | Required | — |
| `intl` | Recommended | Required | — |
| `json` | Required | Required | Required |
| `mbstring` | Required | Required | Required |
| `mysqli` | Required | — | — |
| `pdo_mysql` | Required | Required | Required |
| `opcache` | Recommended | Required | Recommended |
| `soap` | — | Required | — |
| `xml` | Required | Required | Required |
| `zip` | Required | Required | — |

## Installing Extensions

=== "Ubuntu / Debian"

    ```bash
    # WordPress
    sudo apt install php-cgi php-mysql php-curl php-gd php-mbstring \
        php-xml php-zip php-intl php-imagick php-opcache

    # Magento 2
    sudo apt install php-cgi php-bcmath php-curl php-gd php-intl php-mbstring \
        php-mysql php-soap php-xml php-zip php-opcache php-sodium php-xsl

    # Redis object cache
    sudo apt install php-redis
    ```

=== "AlmaLinux / Rocky / RHEL"

    ```bash
    # WordPress
    sudo dnf install php-cgi php-mysqlnd php-curl php-gd php-mbstring \
        php-xml php-zip php-intl php-pecl-imagick php-opcache

    # Magento 2
    sudo dnf install php-cgi php-bcmath php-curl php-gd php-intl php-mbstring \
        php-mysqlnd php-soap php-xml php-zip php-opcache php-sodium php-xsl
    ```

=== "cPanel (EA-PHP)"

    On cPanel servers, install extensions via WHM:

    1. Go to **WHM > Software > EasyApache 4**
    2. Select the PHP version
    3. Check the required extensions
    4. Apply changes

    Or via command line:

    ```bash
    /usr/local/bin/ea-install-profile --install /etc/cpanel/ea4/profiles/custom/veloserve.json
    ```

## Verifying Extensions

```bash
# List all installed extensions
php -m

# Check a specific extension
php -m | grep -i redis

# Detailed info
php -i | grep -i "extension_dir"

# Via phpinfo()
echo '<?php phpinfo();' > /tmp/test.php
veloserve start --root /tmp --listen 127.0.0.1:9999 &
curl http://127.0.0.1:9999/test.php | grep -i redis
```

## PECL Extensions

For extensions not available in your package manager:

```bash
sudo pecl install redis
echo "extension=redis.so" | sudo tee /etc/php/8.3/cli/conf.d/20-redis.ini
```

Restart VeloServe after installing new extensions:

```bash
systemctl restart veloserve
```
