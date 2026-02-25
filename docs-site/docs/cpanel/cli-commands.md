# CLI Commands on cPanel

This page covers VeloServe CLI commands commonly used on cPanel servers.

## Service Management

```bash
# Start VeloServe
systemctl start veloserve

# Stop VeloServe
systemctl stop veloserve

# Restart VeloServe
systemctl restart veloserve

# Reload configuration (graceful, no downtime)
systemctl reload veloserve

# Check status
systemctl status veloserve

# Enable auto-start on boot
systemctl enable veloserve

# View logs
journalctl -u veloserve -f
```

## Apache Swap Operations

```bash
# Full swap: Apache → VeloServe
./import-apache-and-swap.sh --swap

# Revert: VeloServe → Apache
./import-apache-and-swap.sh --revert

# Regenerate config only (no service changes)
./import-apache-and-swap.sh --config-only

# Preview what would happen (dry run)
./import-apache-and-swap.sh
```

## Configuration

```bash
# Test config for errors
veloserve config test --config /etc/veloserve/veloserve.toml

# Show parsed config
veloserve config show --config /etc/veloserve/veloserve.toml

# Print default config template
veloserve config show-default

# Convert Apache config to VeloServe format
veloserve config convert-apache \
  --input /usr/local/apache/conf/httpd.conf \
  --output /etc/veloserve/veloserve.toml

# Convert vhosts only (append to existing)
veloserve config convert-apache \
  --input /usr/local/apache/conf/httpd.conf \
  --output /tmp/vhosts.toml \
  --vhosts-only
```

## Cache Operations

```bash
# Show cache statistics
veloserve cache stats

# Purge all cached content
veloserve cache purge --all

# Purge a specific domain
veloserve cache purge --domain example.com

# Purge by URL pattern
veloserve cache purge --pattern "/blog/*"

# Warm cache from sitemap
veloserve cache warm --sitemap https://example.com/sitemap.xml
```

## PHP Information

```bash
# Show PHP configuration
veloserve php info

# Test PHP execution
veloserve php test

# Find installed EA-PHP versions
ls /opt/cpanel/ea-php*/root/usr/bin/php-cgi
```

## Diagnostics

```bash
# Check which web server is active
systemctl is-active httpd 2>/dev/null && echo "Apache" || echo "Apache stopped"
systemctl is-active veloserve 2>/dev/null && echo "VeloServe" || echo "VeloServe stopped"

# Check what's listening on port 80
ss -tlnp | grep :80

# Check what's listening on port 443
ss -tlnp | grep :443

# View VeloServe error log
tail -f /var/log/veloserve/error.log

# View PHP error log
tail -f /var/log/veloserve/php-error.log

# View hook activity log
tail -f /var/log/veloserve/hooks.log

# Check chkservd monitoring status
grep -E "httpd|veloserve" /etc/chkserv.d/chkservd.conf
```

## Hook Management

```bash
# List all registered hooks
/usr/local/cpanel/bin/manage_hooks list

# Re-register VeloServe hooks
bash /usr/local/veloserve/cpanel/hooks/install-hooks.sh

# Test hook describe output
/usr/local/veloserve/cpanel/hooks/veloserve-hook.sh --describe
```

## Version and Build Info

```bash
# Show version
veloserve --version

# Detailed version info
veloserve version
```

## Next Steps

- **[Configuration Reference](../configuration/reference.md)** — full config file documentation
- **[Troubleshooting](../troubleshooting.md)** — common issues and solutions
