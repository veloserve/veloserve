# VeloPanel Overview

VeloPanel is a lightweight, high-performance web hosting control panel powered by [VeloServe](https://veloserve.io). It ships as a single compiled binary — no PHP, no Node.js runtime, no external dependencies — and provides everything you need to manage hosting accounts, domains, databases, SSL certificates, and more through a modern web UI and REST API.

## Key Features

| Feature | Description |
|---|---|
| **Account Management** | Create, suspend, delete hosting accounts with Linux system user isolation |
| **Domain Management** | Primary domains, addon domains, subdomains with automatic VeloServe vhost generation |
| **SSL / Let's Encrypt** | One-click ACME certificate provisioning, auto-renewal for all domains |
| **Database Management** | Create and manage MySQL, MariaDB, and PostgreSQL databases per account |
| **File Manager** | Web-based file browser with editor, permission management |
| **PHP Management** | Multi-version PHP support, CGI/FPM mode switching per account |
| **Backup System** | Full account backups with S3 and SFTP remote target support |
| **System Monitoring** | CPU, RAM, disk, load average, VeloServe service status |
| **License Tiers** | Community (free, 5 accounts / 10 domains), Pro, Business |

## Architecture

```
VeloPanel (:7070)
├── Rust Backend (Axum)
│   ├── REST API (46 endpoints, JWT auth)
│   ├── Account / Domain CRUD → auto-generates VeloServe vhosts
│   ├── ACME Client (Let's Encrypt HTTP-01)
│   ├── Backup Manager (tar.gz + mysqldump)
│   ├── File Manager (sandboxed per account)
│   ├── Database Manager (MySQL + PostgreSQL)
│   ├── License Validator
│   └── System Monitor (sysinfo)
├── Svelte 5 Frontend (embedded in binary)
│   ├── Dashboard with live stats
│   ├── Accounts / Domains / SSL pages
│   ├── PHP / Files / Databases managers
│   └── System & VeloServe controls
└── Database Backend (runtime-selected)
    └── MySQL | MariaDB | SQLite | PostgreSQL
```

## How It Works

1. VeloPanel runs as a systemd service on port **7070**
2. When you create an account or domain, VeloPanel:
    - Creates a Linux system user and home directory
    - Generates a VeloServe vhost configuration (TOML)
    - Reloads VeloServe to pick up the new site
3. VeloServe handles all HTTP/HTTPS traffic on ports **80/443**
4. SSL certificates are provisioned via Let's Encrypt ACME and written to `/etc/veloserve/ssl/`

## License Tiers

| Tier | Accounts | Domains | Backups | PHP Modes | Price |
|---|---|---|---|---|---|
| **Community** | 5 | 10 | Local only | CGI, FPM | Free |
| **Pro** | 50 | Unlimited | S3 / SFTP | + LSAPI | $9.99/mo |
| **Business** | Unlimited | Unlimited | Full | All modes | $24.99/mo |

!!! tip "Getting Started"
    Head to the [Installation Guide](installation.md) to get VeloPanel running in under 5 minutes, or jump straight to the [API Reference](api-reference.md) if you want to automate everything via scripts.
