# Troubleshooting

Common issues and solutions for VeloServe.

## Server Issues

### Port already in use

```
Error: Failed to bind to 0.0.0.0:80 - Address already in use
```

Another process is using the port:

```bash
# Find what's using port 80
ss -tlnp | grep :80
# or
lsof -i :80

# On cPanel, stop Apache first
systemctl stop httpd
```

### Permission denied on port 80/443

Ports below 1024 require root privileges:

```bash
# Run as root
sudo veloserve --config /etc/veloserve/veloserve.toml start

# Or use systemd (runs as root by default)
systemctl start veloserve
```

### Config file not found

```bash
# Test config
veloserve config test --config /etc/veloserve/veloserve.toml

# Check file exists
ls -la /etc/veloserve/veloserve.toml
```

## PHP Issues

### PHP binary not found

```
Error: PHP binary not found at "/usr/bin/php-cgi"
```

Install PHP-CGI or correct the path:

```bash
# Find php-cgi
which php-cgi
find / -name php-cgi 2>/dev/null

# On cPanel
ls /opt/cpanel/ea-php*/root/usr/bin/php-cgi
```

### 502 Bad Gateway

PHP crashed or timed out:

1. Increase `max_execution_time` in config
2. Increase `memory_limit`
3. Check PHP error log: `tail -f /var/log/veloserve/php_errors.log`
4. Test with a minimal script: `<?php echo "OK"; ?>`

### Blank page

PHP executes but produces no output:

1. Set `display_errors = true` temporarily
2. Check error log: `tail -f /var/log/veloserve/php_errors.log`
3. Verify file permissions: `ls -la /var/www/html/index.php`
4. Test with: `<?php phpinfo(); ?>`

### Extensions not loading

```bash
# Check loaded extensions
php -m

# Check extension directory
php -i | grep extension_dir

# Restart VeloServe after installing extensions
systemctl restart veloserve
```

## SSL Issues

### HTTPS not working

1. Verify `listen_ssl` is configured:
   ```bash
   grep listen_ssl /etc/veloserve/veloserve.toml
   ```

2. Check certificate files exist:
   ```bash
   ls -la /var/cpanel/ssl/installed/certs/
   ```

3. Verify VeloServe is listening on 443:
   ```bash
   ss -tlnp | grep 443
   ```

### Certificate mismatch

Wrong certificate served for a domain:

```bash
# Check what cert is served
openssl s_client -connect localhost:443 -servername example.com </dev/null 2>/dev/null | \
    openssl x509 -noout -subject

# Check config
grep -A 4 "example.com" /etc/veloserve/veloserve.toml
```

## cPanel Issues

### Apache keeps restarting

chkservd is re-enabling Apache:

```bash
# Check monitoring status
grep httpd /etc/chkserv.d/chkservd.conf

# Should show httpd:0 after swap
# If not, re-run the swap:
./import-apache-and-swap.sh --swap
```

### WHM plugin shows 403

The app config needs ACLs:

```bash
# Check veloserve.conf
cat /var/cpanel/apps/veloserve.conf
# Should contain: acls=all

# Re-register
/usr/local/cpanel/bin/register_appconfig /var/cpanel/apps/veloserve.conf
```

### Hooks not firing

```bash
# Check hook registration
/usr/local/cpanel/bin/manage_hooks list | grep veloserve

# Re-install hooks
bash /usr/local/veloserve/cpanel/hooks/install-hooks.sh

# Check hook log
tail -20 /var/log/veloserve/hooks.log
```

## Performance Issues

### Slow PHP responses

1. Enable OPcache:
   ```toml
   [php]
   ini_settings = ["opcache.enable=1"]
   ```

2. Increase workers:
   ```toml
   [php]
   workers = 16
   ```

3. Consider SAPI mode for 10-100x improvement

### High memory usage

1. Reduce cache size: `memory_limit = "256M"` in `[cache]`
2. Reduce PHP workers
3. Check for PHP memory leaks in your application

## Diagnostic Commands

```bash
# VeloServe status
systemctl status veloserve

# Live logs
journalctl -u veloserve -f

# Error log
tail -f /var/log/veloserve/error.log

# PHP error log
tail -f /var/log/veloserve/php_errors.log

# Cache stats
veloserve cache stats

# Config validation
veloserve config test --config /etc/veloserve/veloserve.toml

# PHP info
veloserve php info

# Network listeners
ss -tlnp | grep -E "80|443|8080"
```
