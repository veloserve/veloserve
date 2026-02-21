# cPanel/WHM Integration Guide

VeloServe provides seamless integration with cPanel/WHM as a drop-in replacement for Apache.

## Overview

This integration allows you to:
- Replace Apache with VeloServe on cPanel servers
- Manage VeloServe through WHM interface
- Maintain full cPanel functionality (domain management, SSL, etc.)
- Use per-user PHP worker pools
- Import existing Apache configurations

## Architecture

```
WHM/cPanel → Apache config → VeloServe (reads Apache config)
                    ↓
            veloserve-php (per-user pools)
                    ↓
              PHP scripts
```

## Installation

### Prerequisites

- cPanel/WHM (version 100 or later recommended)
- Root access to server
- Rust toolchain (for building from source)

### Step 1: Install VeloServe

```bash
# Clone repository
git clone https://github.com/veloserve/veloserve.git
cd veloserve

# Build binaries
cargo build --release

# Install binaries
cp target/release/veloserve /usr/local/bin/
cp target/release/veloserve-php /usr/local/bin/
chmod +x /usr/local/bin/veloserve*
```

### Step 2: Install WHM Plugin

```bash
cd cpanel
chmod +x install-whm-plugin.sh
./install-whm-plugin.sh
```

This will:
- Install WHM plugin interface
- Create systemd/init.d service
- Set up required directories
- Register with cPanel AppConfig

### Step 3: Configure VeloServe

#### Import Apache Configuration

```bash
# Convert all Apache vhosts
veloserve config convert-apache \
  --input /etc/apache2/conf/httpd.conf \
  --output /etc/veloserve/vhosts/imported.toml

# Or import specific vhost
veloserve config convert-apache \
  --input /etc/apache2/conf.d/user_example.com.conf \
  --output /etc/veloserve/vhosts/example.com.toml
```

#### Create Main Configuration

Edit `/etc/veloserve/veloserve.toml`:

```toml
[server]
listen = "0.0.0.0:80"
listen_ssl = "0.0.0.0:443"
workers = "auto"
max_connections = 10000

[php]
handler = "socket"
socket_path = "/run/veloserve/php.sock"

[cache]
enable = true
storage = "memory"
memory_limit = "1G"

# Include all vhost configs
include = "/etc/veloserve/vhosts/*.toml"
```

### Step 4: Set Up PHP Worker Pools

#### Global Pool (Default)

```bash
# Start default PHP worker pool
veloserve-php \
  --socket /run/veloserve/php.sock \
  --workers 16 \
  --memory 256M
```

#### Per-User Pools (cPanel Users)

For each cPanel user, create a dedicated pool:

```bash
# Create pool for user 'john'
veloserve-php \
  --user john \
  --socket /run/veloserve/john.sock \
  --workers 4 \
  --memory 128M

# Add to user's vhost config
cat >> /etc/veloserve/vhosts/john.toml << EOF
[[virtualhost]]
domain = "johnsite.com"
root = "/home/john/public_html"
php_socket = "/run/veloserve/john.sock"
EOF
```

### Step 5: Start VeloServe

```bash
# Using systemd
systemctl start veloserve
systemctl enable veloserve

# Or using init.d
service veloserve start
chkconfig veloserve on
```

### Step 6: Switch from Apache

```bash
# Stop Apache
systemctl stop httpd  # CentOS/RHEL
systemctl stop apache2  # Ubuntu/Debian

# Disable Apache auto-start
systemctl disable httpd

# Ensure VeloServe starts on boot
systemctl enable veloserve

# Update cPanel to not manage Apache (optional)
# In WHM: Service Configuration → Service Manager
# Uncheck "httpd"
```

## WHM Plugin Usage

### Accessing the Plugin

1. Log in to WHM as root
2. Navigate to: **Plugins → VeloServe**

### Dashboard

The dashboard shows:
- Server status (running/stopped)
- Active connections
- Request rate
- Cache hit rate
- Virtual host count
- PHP worker status

### Managing Virtual Hosts

1. Click "Virtual Hosts" in the menu
2. View all configured domains
3. Import from Apache
4. Add/delete virtual hosts

### PHP Worker Pools

1. Click "PHP Pools" in the menu
2. View active pools
3. Start/stop/restart pools
4. Create per-user pools

### Cache Management

1. Click "Cache" in the menu
2. View cache statistics
3. Purge cache globally or per-domain
4. Configure cache rules

## Configuration Reference

### Apache Directives Supported

| Apache Directive | VeloServe Support | Notes |
|-----------------|-------------------|-------|
| VirtualHost | ✅ Full | Port, ServerName, ServerAlias |
| DocumentRoot | ✅ Full | Path resolution |
| SSLEngine | ✅ Full | SSL certificates |
| SSLCertificateFile | ✅ Full | |
| SSLCertificateKeyFile | ✅ Full | |
| DirectoryIndex | ✅ Full | index.php, index.html |
| ErrorLog | ✅ Full | Error logging |
| CustomLog | ✅ Full | Access logging |
| php_admin_value | ✅ Full | PHP settings |
| php_admin_flag | ✅ Full | PHP flags |
| AllowOverride | ⚠️ Partial | .htaccess limited support |
| RewriteEngine | ⚠️ Planned | URL rewriting |

### PHP Settings per User

Create `/etc/veloserve/php/user.conf`:

```ini
memory_limit = 256M
max_execution_time = 30
upload_max_filesize = 64M
post_max_size = 64M
max_input_vars = 3000
```

## Troubleshooting

### Check VeloServe Status

```bash
# Check if running
systemctl status veloserve

# View logs
journalctl -u veloserve -f
tail -f /var/log/veloserve/error.log

# Test configuration
veloserve config test --config /etc/veloserve/veloserve.toml
```

### Check PHP Workers

```bash
# List PHP processes
ps aux | grep veloserve-php

# Check socket
ls -la /run/veloserve/

# Test PHP pool
veloserve-php --socket /run/veloserve/test.sock --verbose
```

### Common Issues

#### Permission Denied on Socket

```bash
# Fix socket permissions
chmod 666 /run/veloserve/*.sock
chown -R root:root /run/veloserve/
```

#### PHP Scripts Not Executing

1. Check PHP worker is running:
   ```bash
   ps aux | grep veloserve-php
   ```

2. Verify socket path in config:
   ```bash
   cat /etc/veloserve/veloserve.toml | grep socket
   ```

3. Check PHP worker logs:
   ```bash
   tail -f /var/log/veloserve/php-error.log
   ```

#### Port Already in Use

```bash
# Find process using port 80
lsof -i :80

# Stop Apache if running
systemctl stop httpd
```

## Uninstallation

```bash
cd /path/to/veloserve/cpanel
./uninstall-whm-plugin.sh

# Optional: Remove VeloServe binaries
rm -f /usr/local/bin/veloserve*
rm -rf /etc/veloserve
rm -rf /var/log/veloserve
```

## Migration from LiteSpeed

If migrating from LiteSpeed:

1. Export LiteSpeed configurations
2. Import to VeloServe using Apache config (LiteSpeed uses similar format)
3. Convert `lsphp` pools to `veloserve-php`
4. Update .htaccess rules (limited support initially)

## Support

- GitHub Issues: https://github.com/veloserve/veloserve/issues
- Documentation: https://docs.veloserve.io/cpanel
- Community: https://discord.gg/veloserve
