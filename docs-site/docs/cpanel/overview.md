# cPanel Integration Overview

VeloServe integrates deeply with cPanel/WHM as a **drop-in replacement for Apache**. It reads existing Apache configurations, serves all cPanel accounts on ports 80 and 443, and stays in sync with cPanel through event hooks.

## Why Replace Apache?

| Feature | Apache | LiteSpeed | VeloServe |
|---------|--------|-----------|-----------|
| Written in | C | C++ | Rust |
| Memory safety | Manual | Manual | Guaranteed |
| PHP execution | mod_php / PHP-FPM | LSAPI (lsphp) | Embedded SAPI / CGI |
| cPanel integration | Native | Plugin | Plugin |
| Configuration | httpd.conf | Reads Apache conf | Converts Apache conf |
| Per-domain SSL (SNI) | Yes | Yes | Yes |
| HTTP/2 | Yes | Yes | Yes |
| Page cache | Varnish add-on | Built-in (LSCache) | Built-in |
| Auto-restart (chkservd) | Native | Yes | Yes |
| Open source | Yes | No (paid) | Yes |

## What You Get

### One-Click Apache Swap

Run a single command to convert all Apache virtual hosts (including per-domain SSL certificates) and replace Apache with VeloServe on ports 80 and 443:

```bash
./import-apache-and-swap.sh --swap
```

### WHM Management Plugin

A full management UI accessible at **WHM > Plugins > VeloServe** with:

- **Dashboard** — service status, quick stats, service controls
- **Web Server Switch** — one-click swap between VeloServe and Apache
- **Virtual Hosts** — view, import, and manage all domains
- **PHP** — view and switch EA-PHP versions
- **Cache** — manage the built-in page cache
- **SSL/TLS** — view certificate status for all domains
- **Config Editor** — edit `veloserve.toml` directly from WHM
- **Logs** — view error logs, hook activity, and systemd journal
- **About** — version and system information

### Automatic Configuration Sync

cPanel hooks keep VeloServe in sync with all cPanel operations:

- **Account create/remove** — vhosts are added/removed automatically
- **Addon domains, subdomains, parked domains** — reflected in real time
- **SSL provisioning (AutoSSL / Let's Encrypt)** — certificate paths updated instantly

### Service Monitoring

VeloServe registers with cPanel's `chkservd` / `tailwatchd` daemon:

- Apache monitoring is disabled (no more Apache auto-restarts)
- VeloServe health checks run continuously
- Automatic restart if VeloServe goes down

## Architecture

```
                   ┌─────────────────────────────┐
                   │         cPanel / WHM         │
                   │   (account management, SSL)  │
                   └──────────────┬──────────────┘
                                  │ hooks
                   ┌──────────────▼──────────────┐
                   │   veloserve-hook.sh          │
                   │   (auto-updates config)      │
                   └──────────────┬──────────────┘
                                  │
                   ┌──────────────▼──────────────┐
                   │     VeloServe (Rust)         │
                   │   Port 80 (HTTP)             │
                   │   Port 443 (HTTPS + SNI)     │
                   │   ┌────────────────────┐     │
                   │   │  Built-in Cache    │     │
                   │   └────────────────────┘     │
                   └──────────────┬──────────────┘
                                  │
                   ┌──────────────▼──────────────┐
                   │   EA-PHP (php-cgi)           │
                   │   /opt/cpanel/ea-phpXX/      │
                   └─────────────────────────────┘
```

## Requirements

- cPanel/WHM version 100 or later
- AlmaLinux 8/9, CloudLinux 8/9, or Rocky Linux 8/9
- Root access
- EA-PHP installed (any version from 8.0 to 8.4)

## Next Steps

1. **[Installation](installation.md)** — install VeloServe and the WHM plugin
2. **[Apache Swap](apache-swap.md)** — replace Apache with one command
3. **[WHM Plugin](whm-plugin.md)** — explore the management interface
