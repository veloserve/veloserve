# tailwatchd Integration

cPanel's `tailwatchd` daemon runs `chkservd`, which monitors services and automatically restarts them if they go down. Without proper integration, `chkservd` would restart Apache after you stop it — defeating the swap.

## The Problem

By default, cPanel monitors Apache (`httpd`) via `chkservd`. If you stop Apache and start VeloServe, `chkservd` will restart Apache within a few minutes because it sees `httpd` as "down."

## The Solution

The `import-apache-and-swap.sh --swap` script automatically handles this, but here is what it does under the hood.

### Disable Apache Monitoring

Edit `/etc/chkserv.d/chkservd.conf` and set:

```
httpd:0
apache_php_fpm:0
```

This tells `chkservd` to stop monitoring Apache and PHP-FPM.

### Create VeloServe Health Check

Create `/etc/chkserv.d/veloserve`:

```
service[veloserve]=80,GET / HTTP/1.0\r\n\r\n,HTTP/
```

This tells `chkservd` to probe port 80 with an HTTP request and expect an `HTTP/` response. If VeloServe does not respond, `chkservd` will restart it.

### Enable VeloServe Monitoring

Add to `/etc/chkserv.d/chkservd.conf`:

```
veloserve:1
```

### Restart tailwatchd

```bash
systemctl restart tailwatchd
```

This applies the new monitoring configuration immediately.

## Verifying the Setup

### Check monitoring status

```bash
grep -E "httpd|veloserve|apache_php_fpm" /etc/chkserv.d/chkservd.conf
```

Expected output after swap:

```
httpd:0
apache_php_fpm:0
veloserve:1
```

### Check the health check file

```bash
cat /etc/chkserv.d/veloserve
```

### Test auto-restart

```bash
# Kill VeloServe
systemctl stop veloserve

# Wait 30-60 seconds for chkservd to detect and restart
sleep 60

# Check if it came back
systemctl is-active veloserve
```

## WHM Service Manager

VeloServe also registers with WHM's Service Manager during installation:

```bash
echo "veloserve" > /var/cpanel/service_autorestart/veloserve
```

This allows WHM to display and manage VeloServe in **WHM > Service Configuration > Service Manager**.

## Reverting

The `--revert` flag restores Apache monitoring:

```bash
./import-apache-and-swap.sh --revert
```

This sets:

- `httpd:1` (re-enable Apache monitoring)
- `veloserve:0` (disable VeloServe monitoring)
- Starts Apache, stops VeloServe
- Restarts `tailwatchd`

## Manual Configuration

If you need to adjust the health check, edit `/etc/chkserv.d/veloserve`:

```bash
# Check a specific URL
service[veloserve]=80,GET /health HTTP/1.0\r\n\r\n,HTTP/

# Check HTTPS instead
service[veloserve]=443,GET / HTTP/1.0\r\n\r\n,HTTP/
```

After editing, restart `tailwatchd`:

```bash
systemctl restart tailwatchd
```

## Troubleshooting

### Apache keeps coming back

1. Check `chkservd.conf`: `grep httpd /etc/chkserv.d/chkservd.conf` — should show `httpd:0`
2. Restart tailwatchd: `systemctl restart tailwatchd`
3. Check for other services re-enabling Apache: `grep -r httpd /etc/chkserv.d/`

### VeloServe not being monitored

1. Check `chkservd.conf`: `grep veloserve /etc/chkserv.d/chkservd.conf` — should show `veloserve:1`
2. Check health check file exists: `cat /etc/chkserv.d/veloserve`
3. Test the health check manually: `curl -s http://localhost/ | head -1` — should return HTTP response

## Next Steps

- **[Apache Swap](apache-swap.md)** — the full swap process
- **[CLI Commands](cli-commands.md)** — service management commands
- **[Installation](installation.md)** — initial setup
