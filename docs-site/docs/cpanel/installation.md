# cPanel Installation

This guide covers installing VeloServe and the WHM plugin on a cPanel server.

## Step 1: Download VeloServe

Download the pre-built binary for your platform from [GitHub Releases](https://github.com/veloserve/veloserve/releases):

=== "AlmaLinux 9 / Rocky 9 / CloudLinux 9"

    ```bash
    curl -LO https://github.com/veloserve/veloserve/releases/latest/download/veloserve-latest-x86_64-almalinux9.tar.gz
    tar -xzf veloserve-latest-x86_64-almalinux9.tar.gz
    sudo mv veloserve /usr/local/bin/
    sudo chmod +x /usr/local/bin/veloserve
    ```

=== "AlmaLinux 8 / CentOS 8"

    ```bash
    curl -LO https://github.com/veloserve/veloserve/releases/latest/download/veloserve-latest-x86_64-unknown-linux-gnu.tar.gz
    tar -xzf veloserve-latest-x86_64-unknown-linux-gnu.tar.gz
    sudo mv veloserve /usr/local/bin/
    sudo chmod +x /usr/local/bin/veloserve
    ```

Verify the installation:

```bash
veloserve --version
```

## Step 2: Clone the Integration Scripts

```bash
git clone https://github.com/veloserve/veloserve.git /tmp/veloserve-repo
cd /tmp/veloserve-repo/cpanel
```

## Step 3: Install the WHM Plugin

```bash
chmod +x install-whm-plugin.sh
./install-whm-plugin.sh
```

This script does the following:

1. **Copies the WHM plugin** (`veloserve.cgi`, CSS, JS) to `/usr/local/cpanel/whostmgr/docroot/cgi/veloserve/`
2. **Registers with cPanel AppConfig** — creates `/var/cpanel/apps/veloserve.conf` with `acls=all`
3. **Creates directories** — `/etc/veloserve/`, `/var/log/veloserve/`, `/run/veloserve/`
4. **Installs the systemd service** — `veloserve.service` with auto-restart on failure
5. **Installs cPanel hooks** — auto-sync config on account/domain/SSL changes
6. **Registers with WHM Service Manager** — VeloServe appears in service management
7. **Copies the swap script** to `/usr/local/veloserve/cpanel/import-apache-and-swap.sh`
8. **Creates a default config** at `/etc/veloserve/veloserve.toml` (if none exists), auto-detecting the installed EA-PHP version

## Step 4: Swap Apache for VeloServe

```bash
./import-apache-and-swap.sh --swap
```

See [Apache Swap](apache-swap.md) for full details on what this does.

## Verifying the Installation

### Check the Service

```bash
systemctl status veloserve
```

### Access the WHM Plugin

1. Log in to WHM as root
2. Navigate to **Plugins > VeloServe**
3. You should see the VeloServe Dashboard

### Test a Website

```bash
curl -I http://your-server-ip
```

The response should include `Server: VeloServe`.

## Directory Layout

After installation, VeloServe files are located at:

| Path | Purpose |
|------|---------|
| `/usr/local/bin/veloserve` | Main binary |
| `/etc/veloserve/veloserve.toml` | Configuration file |
| `/etc/veloserve/vhosts/` | Per-user vhost configs (optional) |
| `/var/log/veloserve/` | Log files |
| `/run/veloserve/` | Runtime files (PID, sockets) |
| `/usr/local/veloserve/cpanel/` | Swap script |
| `/usr/local/cpanel/whostmgr/docroot/cgi/veloserve/` | WHM plugin files |
| `/var/cpanel/apps/veloserve.conf` | cPanel app registration |
| `/etc/chkserv.d/veloserve` | Health check for tailwatchd |

## Uninstalling

```bash
# Stop the service
systemctl stop veloserve
systemctl disable veloserve

# Revert to Apache
./import-apache-and-swap.sh --revert

# Remove files
rm -f /usr/local/bin/veloserve
rm -rf /etc/veloserve
rm -rf /var/log/veloserve
rm -rf /usr/local/cpanel/whostmgr/docroot/cgi/veloserve
rm -f /var/cpanel/apps/veloserve.conf
rm -f /etc/systemd/system/veloserve.service
systemctl daemon-reload
```

## Next Steps

- **[Apache Swap](apache-swap.md)** — understand the swap process in detail
- **[WHM Plugin](whm-plugin.md)** — explore the management interface
- **[cPanel Hooks](hooks.md)** — how auto-sync works
