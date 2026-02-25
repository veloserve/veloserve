# Apache Swap

The `import-apache-and-swap.sh` script converts all Apache virtual hosts to VeloServe configuration and replaces Apache as the web server on ports 80 and 443.

## Usage

```bash
# Preview: generate config and show instructions (no services touched)
./import-apache-and-swap.sh

# Generate config only (write veloserve.toml, don't touch services)
./import-apache-and-swap.sh --config-only

# Full swap: generate config + stop Apache + start VeloServe + update chkservd
./import-apache-and-swap.sh --swap

# Revert: stop VeloServe, re-enable Apache + chkservd monitoring
./import-apache-and-swap.sh --revert
```

## What `--swap` Does

The swap process performs these steps in order:

### 1. Detect Apache Configuration

Searches for `httpd.conf` in standard cPanel locations:

- `/usr/local/apache/conf/httpd.conf` (cPanel default)
- `/etc/httpd/conf/httpd.conf`
- `/etc/apache2/apache2.conf`

### 2. Backup Existing Config

If `/etc/veloserve/veloserve.toml` exists, it is backed up to `/etc/veloserve/veloserve.toml.bak`.

### 3. Detect EA-PHP

Scans for the newest installed EA-PHP version:

```
/opt/cpanel/ea-php84/root/usr/bin/php-cgi
/opt/cpanel/ea-php83/root/usr/bin/php-cgi
/opt/cpanel/ea-php82/root/usr/bin/php-cgi
...
```

### 4. Generate Base Configuration

Writes a base `veloserve.toml` with:

- `[server]` listening on `0.0.0.0:80` and `0.0.0.0:443`
- `[php]` configured with the detected EA-PHP binary
- `[ssl]` global fallback certificate (cPanel self-signed)
- `[cache]` enabled with 1 GB memory cache

### 5. Convert Virtual Hosts

Runs `veloserve config convert-apache --vhosts-only` to parse every `<VirtualHost>` block in Apache's config and append the corresponding `[[virtualhost]]` sections, including:

- Domain name and aliases
- Document root
- Per-domain SSL certificate and key paths

### 6. Stop Apache, Start VeloServe

```bash
systemctl stop httpd
systemctl disable httpd
systemctl start veloserve
systemctl enable veloserve
```

### 7. Update chkservd / tailwatchd

This is critical — without this step, cPanel's `tailwatchd` daemon would restart Apache within minutes:

- Sets `httpd:0` in `/etc/chkserv.d/chkservd.conf` (disables Apache monitoring)
- Sets `apache_php_fpm:0` (disables PHP-FPM monitoring)
- Creates `/etc/chkserv.d/veloserve` with an HTTP health check on port 80
- Sets `veloserve:1` in `chkservd.conf` (enables VeloServe monitoring)
- Restarts `tailwatchd` to apply changes

## SSL Certificate Handling

cPanel stores SSL certificates at:

- Certificates: `/var/cpanel/ssl/installed/certs/<id>.crt`
- Keys: `/var/cpanel/ssl/installed/keys/<id>.key`

The converter reads `SSLCertificateFile` and `SSLCertificateKeyFile` from each Apache `<VirtualHost *:443>` block and maps them to VeloServe config:

```toml
[[virtualhost]]
domain = "example.com"
root = "/home/user/public_html"
ssl_certificate = "/var/cpanel/ssl/installed/certs/example_com.crt"
ssl_certificate_key = "/var/cpanel/ssl/installed/keys/example_com.key"
```

VeloServe uses SNI (Server Name Indication) to serve the correct certificate for each domain. The global `[ssl]` certificate is used as a fallback for unmatched requests.

## Reverting to Apache

```bash
./import-apache-and-swap.sh --revert
```

This reverses the swap:

1. Stops VeloServe
2. Re-enables `httpd:1` in `chkservd.conf`
3. Disables VeloServe monitoring (`veloserve:0`)
4. Starts Apache
5. Restarts `tailwatchd`

## Manual Conversion

You can also convert Apache configs without the full swap:

```bash
# Full conversion (server block + vhosts)
veloserve config convert-apache \
  --input /usr/local/apache/conf/httpd.conf \
  --output /etc/veloserve/veloserve.toml

# Vhosts only (append to existing config)
veloserve config convert-apache \
  --input /usr/local/apache/conf/httpd.conf \
  --output /etc/veloserve/vhosts.toml \
  --vhosts-only
```

## Apache Directive Support

| Apache Directive | VeloServe Support |
|-----------------|-------------------|
| `<VirtualHost>` | Full |
| `ServerName` | Full |
| `ServerAlias` | Full |
| `DocumentRoot` | Full |
| `SSLEngine` | Full |
| `SSLCertificateFile` | Full |
| `SSLCertificateKeyFile` | Full |
| `DirectoryIndex` | Full |
| `ErrorLog` | Full |
| `CustomLog` | Full |
| `php_admin_value` | Full |
| `php_admin_flag` | Full |
| `AllowOverride` | Partial (.htaccess limited) |
| `RewriteEngine` | Planned |

## Next Steps

- **[WHM Plugin](whm-plugin.md)** — manage VeloServe from the WHM UI
- **[SSL & AutoSSL](ssl-autossl.md)** — certificate management details
- **[tailwatchd](tailwatchd.md)** — service monitoring details
