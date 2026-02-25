# WHM Plugin

The VeloServe WHM plugin provides a full management interface accessible at **WHM > Plugins > VeloServe**. It includes 9 pages for managing every aspect of VeloServe on your cPanel server.

## Accessing the Plugin

1. Log in to WHM as root (`https://your-server:2087`)
2. Navigate to **Plugins** in the left sidebar
3. Click **VeloServe**

## Dashboard

The dashboard is the landing page and provides an at-a-glance view of your VeloServe installation:

- **Service Status** — whether VeloServe is running or stopped, with the active web server indicated
- **Quick Stats** — number of virtual hosts, SSL certificates, active PHP version, cache status
- **Service Controls** — Start, Stop, Restart, and Reload buttons
- **Quick Links** — jump to any other section

## Web Server Switch

The Switch page provides one-click swapping between VeloServe and Apache:

- **Switch to VeloServe** — runs `import-apache-and-swap.sh --swap`, converting all Apache vhosts and updating chkservd
- **Switch to Apache** — runs `import-apache-and-swap.sh --revert`, restoring Apache and its monitoring
- **chkservd Status** — shows the current monitoring state for both `httpd` and `veloserve`

!!! warning
    Switching web servers affects all websites on the server. Ensure you have tested VeloServe with your sites before switching in production.

## Virtual Hosts

The Virtual Hosts page displays all configured domains:

| Column | Description |
|--------|-------------|
| Domain | The primary domain name |
| Document Root | Filesystem path to the website files |
| Owner | The cPanel account that owns the domain |
| Platform | Detected CMS (WordPress, Magento, generic) |
| SSL | Whether an SSL certificate is configured |

**Actions:**

- **Import from Apache** — re-reads Apache's `httpd.conf` and imports any new virtual hosts
- **Remove** — removes a specific virtual host from VeloServe's config

## PHP Configuration

The PHP page shows:

- **Current PHP Settings** — mode, binary path, workers, memory limit
- **Installed EA-PHP Versions** — all PHP versions detected in `/opt/cpanel/ea-phpXX/`
- **Switch PHP Version** — change the active EA-PHP version used by VeloServe

When you switch PHP versions, VeloServe updates the `binary_path` in `veloserve.toml` and reloads.

## Cache Management

The Cache page displays:

- **Cache Status** — enabled/disabled, storage backend, memory limit
- **Purge All Cache** — clears the entire page cache immediately

Cache settings are configured in `veloserve.toml` under the `[cache]` section and can be edited via the Config Editor page.

## SSL/TLS Status

The SSL page provides a comprehensive view of certificates:

- **Global SSL Certificate** — the fallback certificate from the `[ssl]` section (typically cPanel's self-signed cert)
- **Per-Domain Certificates** — a table of all virtual hosts with SSL configured, showing:
    - Certificate file path
    - Issuer (Let's Encrypt, cPanel, Sectigo, etc.)
    - Expiry date
    - Status (valid, expiring soon, expired)
- **AutoSSL Integration** — status of cPanel's automatic SSL provisioning

When AutoSSL renews a certificate, the certificate files on disk are updated in place. VeloServe picks up the new certificates on reload.

## Configuration Editor

The Config page provides a text editor for `/etc/veloserve/veloserve.toml`:

- **Edit** — modify any configuration directive directly
- **Save & Reload** — writes the file and sends a reload signal to VeloServe
- **Discard** — reverts unsaved changes

!!! tip
    Use this for quick tweaks. For major changes (adding vhosts, changing PHP), use the dedicated pages or re-run the Apache import.

## Logs Viewer

The Logs page provides tabbed access to:

| Tab | Source |
|-----|--------|
| Error Log | `/var/log/veloserve/error.log` |
| Hook Activity | `/var/log/veloserve/hooks.log` |
| Systemd Journal | `journalctl -u veloserve` |

Features:

- **Refresh** — manually reload the log view
- **Auto-refresh** — poll for new log entries every few seconds
- **Line limit** — configurable number of lines to display

## About

The About page displays:

- VeloServe version and build information
- Operating system and kernel version
- cPanel version
- Links to documentation and GitHub

## Next Steps

- **[Apache Swap](apache-swap.md)** — details on the swap process
- **[cPanel Hooks](hooks.md)** — how auto-sync keeps config current
- **[SSL & AutoSSL](ssl-autossl.md)** — certificate management
