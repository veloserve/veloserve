# VeloPanel Installation

## Requirements

- **OS**: Linux (x86_64 or aarch64) — Ubuntu 22.04+, Debian 12+, AlmaLinux 9+, Rocky 9+
- **Database**: One of MySQL 8+, MariaDB 10.5+, SQLite 3, or PostgreSQL 14+
- **Privileges**: Root access

## Quick Install

The interactive installer handles everything:

```bash
curl -sSL https://raw.githubusercontent.com/veloserve/velopanel/main/install.sh | bash
```

The installer will:

1. Detect your OS and architecture
2. Prompt for your database backend (MySQL, MariaDB, SQLite, PostgreSQL)
3. Download the correct VeloPanel binary
4. Download and configure VeloServe (HTTP/HTTPS web server)
5. Create the database, config file, and systemd services
6. Start both VeloPanel and VeloServe

After installation, open `http://your-server:7070` and create your admin account.

## Manual Install

### 1. Download the binary

=== "x86_64"

    ```bash
    wget https://github.com/veloserve/velopanel/releases/latest/download/velopanel-linux-x86_64
    chmod +x velopanel-linux-x86_64
    mv velopanel-linux-x86_64 /usr/local/bin/velopanel
    ```

=== "aarch64 (ARM64)"

    ```bash
    wget https://github.com/veloserve/velopanel/releases/latest/download/velopanel-linux-aarch64
    chmod +x velopanel-linux-aarch64
    mv velopanel-linux-aarch64 /usr/local/bin/velopanel
    ```

### 2. Create directories

```bash
mkdir -p /etc/veloserve/ssl /etc/veloserve/vhosts /var/lib/velopanel/backups /var/log/veloserve
```

### 3. Set up the database

=== "MySQL / MariaDB"

    ```bash
    mysql -u root <<SQL
    CREATE DATABASE velopanel CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
    CREATE USER 'velopanel'@'localhost' IDENTIFIED BY 'your-strong-password';
    GRANT ALL PRIVILEGES ON velopanel.* TO 'velopanel'@'localhost';
    FLUSH PRIVILEGES;
    SQL
    ```

=== "PostgreSQL"

    ```bash
    sudo -u postgres psql <<SQL
    CREATE USER velopanel WITH PASSWORD 'your-strong-password';
    CREATE DATABASE velopanel OWNER velopanel;
    SQL
    ```

=== "SQLite"

    No setup needed — the database file is created automatically.

### 4. Create the config

```bash
cat > /etc/veloserve/velopanel.toml <<'TOML'
bind_address = "0.0.0.0"
bind_port = 7070

# Choose your database backend:
# MySQL:      mysql://velopanel:password@localhost/velopanel
# PostgreSQL: postgres://velopanel:password@localhost/velopanel
# SQLite:     sqlite:///var/lib/velopanel/velopanel.db?mode=rwc
database_url = "mysql://velopanel:your-strong-password@localhost/velopanel"

# ACME / Let's Encrypt (set your email to enable SSL provisioning)
acme_email = "admin@yourdomain.com"
acme_staging = false

# Auto-generated if left empty
jwt_secret = ""

home_base = "/home"
veloserve_config_path = "/etc/veloserve/veloserve.toml"

# Community tier limits (override with a license key)
max_accounts_free = 5
max_domains_free = 10
TOML
```

### 5. Install the systemd service

```bash
cat > /etc/systemd/system/velopanel.service <<'EOF'
[Unit]
Description=VeloPanel - Web Hosting Control Panel
After=network.target mysql.service veloserve.service
Wants=network.target veloserve.service

[Service]
Type=simple
ExecStart=/usr/local/bin/velopanel
Restart=always
RestartSec=5
User=root
Group=root
Environment=VELOPANEL_CONFIG=/etc/veloserve/velopanel.toml
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now velopanel
```

### 6. Verify

```bash
systemctl status velopanel
curl -s http://localhost:7070/api/auth/setup \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"YourSecurePassword"}'
```

## Configuration Reference

| Key | Default | Description |
|---|---|---|
| `bind_address` | `0.0.0.0` | Listen address |
| `bind_port` | `7070` | Listen port |
| `database_url` | `mysql://velopanel:velopanel@localhost/velopanel` | Database connection URL |
| `jwt_secret` | (auto-generated) | Secret for JWT signing |
| `server_hostname` | (auto-detected) | Server hostname |
| `home_base` | `/home` | Base directory for account home dirs |
| `veloserve_config_path` | `/etc/veloserve/veloserve.toml` | Path to VeloServe config |
| `acme_email` | (empty) | Email for Let's Encrypt registration |
| `acme_staging` | `false` | Use Let's Encrypt staging environment |
| `max_accounts_free` | `5` | Max accounts on community tier |
| `max_domains_free` | `10` | Max domains on community tier |
| `license_key_path` | (empty) | Path to license key file for Pro/Business |

## Uninstall

```bash
curl -sSL https://raw.githubusercontent.com/veloserve/velopanel/main/uninstall.sh | bash
```

This stops and removes the VeloPanel service, binary, and config. Optionally removes all data.
