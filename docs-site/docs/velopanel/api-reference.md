# VeloPanel API Reference

VeloPanel exposes a comprehensive REST API on port **7070** that powers both the web UI and external automation. Every response uses the same JSON envelope:

```json
// Success
{ "ok": true,  "data": { ... } }

// Error
{ "ok": false, "error": "Human-readable error message" }
```

## Authentication

All endpoints (except login, logout, and initial setup) require a JWT token. Pass it as either:

- **Bearer token**: `Authorization: Bearer <token>`
- **Cookie**: `velopanel_token=<token>` (set automatically by the login endpoint)

### Get a token

```bash
# First-time setup — create the admin account (only works once)
curl -s -X POST http://localhost:7070/api/auth/setup \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"MySecurePass123"}'

# Login
TOKEN=$(curl -s -X POST http://localhost:7070/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"MySecurePass123"}' \
  | jq -r '.data.token')

# Use on all subsequent requests
AUTH="Authorization: Bearer $TOKEN"
```

Tokens expire after **24 hours**.

### Auth endpoints

| Method | Endpoint | Auth | Description |
|---|---|---|---|
| POST | `/api/auth/setup` | No | Create first admin — disabled after first use |
| POST | `/api/auth/login` | No | Login, returns JWT + sets `velopanel_token` cookie |
| POST | `/api/auth/logout` | No | Clears session cookie |
| GET | `/api/auth/me` | Yes | Returns current user's `username` and `role` |

---

## Accounts

Hosting accounts map 1:1 to Linux system users. Creating an account also creates the primary domain, generates a VeloServe vhost, and reloads the web server.

### Create an account

```bash
curl -s -X POST http://localhost:7070/api/accounts \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{
    "username": "johndoe",
    "domain": "example.com",
    "email": "john@example.com",
    "password": "UserPass123",
    "plan": "free"
  }'
```

**Request body:**

| Field | Type | Required | Description |
|---|---|---|---|
| `username` | string | Yes | 3–16 alphanumeric characters |
| `domain` | string | Yes | Primary domain name |
| `email` | string | Yes | Contact email |
| `password` | string | Yes | Min 8 characters |
| `plan` | string | Yes | `free`, `pro`, or `business` |

**Response:**

```json
{
  "ok": true,
  "data": {
    "id": 1,
    "username": "johndoe",
    "primary_domain": "example.com",
    "email": "john@example.com",
    "plan": "free",
    "status": "active",
    "disk_quota_mb": 500,
    "bandwidth_quota_mb": 10000,
    "disk_used_mb": 0,
    "created_at": "2026-02-28 15:44:05",
    "updated_at": "2026-02-28 15:44:05"
  }
}
```

!!! warning "License Limits"
    On the Community tier, account creation is capped at 5 accounts. Attempting to create a 6th returns:
    `{"ok":false,"error":"Account limit reached (5/5). Upgrade your license for more."}`

### All account endpoints

| Method | Endpoint | Description |
|---|---|---|
| GET | `/api/accounts` | List all accounts |
| POST | `/api/accounts` | Create account |
| GET | `/api/accounts/:id` | Get account by ID |
| DELETE | `/api/accounts/:id` | Delete account (removes system user, vhosts, reloads VeloServe) |
| POST | `/api/accounts/:id/suspend` | Suspend account |
| POST | `/api/accounts/:id/unsuspend` | Re-activate account |

### Examples

```bash
# List all accounts
curl -s http://localhost:7070/api/accounts -H "$AUTH"

# Get account #1
curl -s http://localhost:7070/api/accounts/1 -H "$AUTH"

# Suspend
curl -s -X POST http://localhost:7070/api/accounts/1/suspend -H "$AUTH"

# Unsuspend
curl -s -X POST http://localhost:7070/api/accounts/1/unsuspend -H "$AUTH"

# Delete (cascades to all domains, vhosts, system user)
curl -s -X DELETE http://localhost:7070/api/accounts/1 -H "$AUTH"
```

---

## Domains

Each account can have a primary domain (created with the account) plus addon domains, subdomains, and aliases. Every domain gets a VeloServe vhost config auto-generated.

### Create a domain

```bash
curl -s -X POST http://localhost:7070/api/domains \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{
    "account_id": 1,
    "domain_name": "blog.example.com",
    "domain_type": "addon",
    "document_root": "/home/johndoe/blog"
  }'
```

**Request body:**

| Field | Type | Required | Description |
|---|---|---|---|
| `account_id` | integer | Yes | Parent account ID |
| `domain_name` | string | Yes | Fully qualified domain name |
| `domain_type` | string | Yes | `primary`, `addon`, `subdomain`, or `alias` |
| `document_root` | string | Yes | Absolute path to document root |

!!! warning "License Limits"
    Community tier is limited to 10 total domains across all accounts.

### All domain endpoints

| Method | Endpoint | Description |
|---|---|---|
| GET | `/api/domains` | List all domains |
| POST | `/api/domains` | Create domain |
| GET | `/api/domains/account/:id` | List domains for a specific account |
| DELETE | `/api/domains/:id` | Delete domain (removes vhost, reloads VeloServe) |

---

## SSL / Let's Encrypt

VeloPanel includes a built-in ACME client for Let's Encrypt. Certificates are provisioned via HTTP-01 challenge and stored in `/etc/veloserve/ssl/`.

!!! note "Prerequisites"
    - Set `acme_email` in your config
    - The domain must resolve to this server on port 80

### Provision a certificate

```bash
curl -s -X POST http://localhost:7070/api/ssl/provision \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"domain":"example.com"}'
```

**Request body:**

| Field | Type | Required | Description |
|---|---|---|---|
| `domain` | string | Yes | Domain to provision |
| `webroot` | string | No | Custom webroot for challenge files |

### All SSL endpoints

| Method | Endpoint | Description |
|---|---|---|
| GET | `/api/ssl/certificates` | List all certificates with issuer, expiry, and file info |
| POST | `/api/ssl/provision` | Provision Let's Encrypt certificate for a domain |
| GET | `/api/ssl/certificate/:domain` | Detailed cert info (subject, issuer, dates, serial, renewal status) |
| GET | `/api/ssl/renew/:domain` | Renew certificate for a specific domain |
| POST | `/api/ssl/auto-renew` | Bulk renew all certificates expiring within 30 days |

### Auto-renewal response

```json
{
  "ok": true,
  "data": {
    "renewed": ["example.com", "blog.example.com"],
    "skipped": ["fresh-cert.com"],
    "errors": []
  }
}
```

---

## Databases

Create and manage MySQL and PostgreSQL databases per hosting account. Each database gets a dedicated user with access restricted to that database only.

### Create a database

```bash
curl -s -X POST http://localhost:7070/api/databases \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{
    "account_id": 1,
    "db_name": "wordpress",
    "db_password": "DbSecure12345",
    "engine": "mysql"
  }'
```

**Request body:**

| Field | Type | Required | Description |
|---|---|---|---|
| `account_id` | integer | Yes | Owning account ID |
| `db_name` | string | Yes | Database name (prefixed with account username) |
| `db_password` | string | Yes | Min 8 characters |
| `engine` | string | No | `mysql` (default), `mariadb`, `postgresql`, or `postgres` |

**Response:**

```json
{
  "ok": true,
  "data": {
    "engine": "mysql",
    "database": "johndoe_wordpress",
    "user": "johndoe_wordpress",
    "host": "localhost"
  }
}
```

### All database endpoints

| Method | Endpoint | Description |
|---|---|---|
| GET | `/api/databases/engines` | List available DB engines and their versions |
| GET | `/api/databases/:account_id` | List all databases for an account |
| POST | `/api/databases` | Create database + user |
| DELETE | `/api/databases/:account_id/:db_name` | Drop database and user |

---

## File Manager

Browse, read, write, and manage files within account home directories. All paths are sandboxed — you cannot escape the account's home directory.

### Examples

```bash
# List directory contents
curl -s "http://localhost:7070/api/files/list?account_id=1&path=/" -H "$AUTH"

# Read a file
curl -s "http://localhost:7070/api/files/read?account_id=1&path=/public_html/index.html" -H "$AUTH"

# Write a file
curl -s -X POST http://localhost:7070/api/files/write \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{
    "account_id": 1,
    "path": "/public_html/index.html",
    "content": "<html><body><h1>Hello World</h1></body></html>"
  }'

# Create directory
curl -s -X POST http://localhost:7070/api/files/mkdir \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"account_id":1,"path":"/public_html/assets"}'

# Set permissions
curl -s -X POST http://localhost:7070/api/files/chmod \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"account_id":1,"path":"/public_html/index.html","mode":"644"}'

# Delete
curl -s -X DELETE http://localhost:7070/api/files/delete \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"account_id":1,"path":"/public_html/old-page.html"}'
```

### All file endpoints

| Method | Endpoint | Params | Description |
|---|---|---|---|
| GET | `/api/files/list` | `account_id`, `path` (query) | List directory entries with size, type, permissions |
| GET | `/api/files/read` | `account_id`, `path` (query) | Read file contents |
| POST | `/api/files/write` | `account_id`, `path`, `content` (body) | Create or overwrite file |
| POST | `/api/files/mkdir` | `account_id`, `path` (body) | Create directory |
| POST | `/api/files/chmod` | `account_id`, `path`, `mode` (body) | Set permissions (octal, e.g. `"755"`) |
| DELETE | `/api/files/delete` | `account_id`, `path` (body) | Delete file or directory |

---

## PHP Management

Detect and manage PHP versions installed on the server. Set per-account PHP version and execution mode.

### Examples

```bash
# List installed PHP versions
curl -s http://localhost:7070/api/php/versions -H "$AUTH"

# Get PHP config for account #1
curl -s http://localhost:7070/api/php/config/1 -H "$AUTH"

# Set PHP version and mode
curl -s -X POST http://localhost:7070/api/php/version/1 \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"version":"8.3","mode":"fpm"}'
```

### Endpoints

| Method | Endpoint | Description |
|---|---|---|
| GET | `/api/php/versions` | Detected PHP versions on the server |
| GET | `/api/php/config/:account_id` | Current PHP version and mode for an account |
| POST | `/api/php/version/:account_id` | Set PHP `version` and `mode` (`cgi`, `fpm`, or `lsapi`) |

!!! info "LSAPI Mode"
    The `lsapi` mode requires a Pro or Business license.

---

## Backups

Full account backups include the home directory (tar.gz) and MySQL database dumps. Backups can be stored locally or uploaded to S3/SFTP remote targets.

### Examples

```bash
# List backups for account #1
curl -s http://localhost:7070/api/backups/accounts/1 -H "$AUTH"

# Create a full backup
curl -s -X POST http://localhost:7070/api/backups/accounts/1 -H "$AUTH"

# Restore from backup
curl -s -X POST \
  http://localhost:7070/api/backups/accounts/1/restore/johndoe-20260228-120000.tar.gz \
  -H "$AUTH"

# Delete a backup
curl -s -X DELETE \
  http://localhost:7070/api/backups/accounts/1/johndoe-20260228-120000.tar.gz \
  -H "$AUTH"

# Cleanup backups older than 30 days
curl -s -X POST http://localhost:7070/api/backups/cleanup \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"retention_days":30}'
```

### Endpoints

| Method | Endpoint | Description |
|---|---|---|
| GET | `/api/backups/accounts/:id` | List backups for account |
| POST | `/api/backups/accounts/:id` | Create full backup |
| POST | `/api/backups/accounts/:id/restore/:filename` | Restore from backup |
| DELETE | `/api/backups/accounts/:id/:filename` | Delete backup file |
| POST | `/api/backups/cleanup` | Remove backups older than `retention_days` (default: 30) |
| GET | `/api/backups/remote-targets` | List configured S3/SFTP remote targets |

---

## System & VeloServe

Monitor server resources and control the VeloServe web server.

### System info

```bash
# Server details
curl -s http://localhost:7070/api/system/info -H "$AUTH"
```

```json
{
  "ok": true,
  "data": {
    "hostname": "web01.example.com",
    "os": "Linux (Ubuntu 24.04)",
    "cpu_count": 4,
    "cpu_usage_percent": 12.5,
    "memory_total_mb": 8192,
    "memory_used_mb": 3200,
    "disk_total_mb": 100000,
    "disk_used_mb": 45000
  }
}
```

### Dashboard stats

```bash
curl -s http://localhost:7070/api/system/stats -H "$AUTH"
```

```json
{
  "ok": true,
  "data": {
    "accounts": { "count": 3, "limit": 5 },
    "domains": { "count": 7, "limit": 10 },
    "cpu": { "count": 4, "usage_percent": 12.5 },
    "memory": { "total_mb": 8192, "used_mb": 3200 },
    "disk": { "total_mb": 100000, "used_mb": 45000 },
    "license": { "tier": "community", "valid": true },
    "uptime_secs": 86400,
    "load_average": [0.5, 0.3, 0.2]
  }
}
```

### VeloServe management

```bash
# Check status (installed, running, version, PID)
curl -s http://localhost:7070/api/veloserve/status -H "$AUTH"

# Get version
curl -s http://localhost:7070/api/veloserve/version -H "$AUTH"

# Regenerate ALL vhost configs from database and reload
curl -s -X POST http://localhost:7070/api/veloserve/regenerate -H "$AUTH"

# Just reload (after manual config edits)
curl -s -X POST http://localhost:7070/api/veloserve/reload -H "$AUTH"
```

### Endpoints

| Method | Endpoint | Description |
|---|---|---|
| GET | `/api/system/info` | Server hostname, OS, CPU, RAM, disk |
| GET | `/api/system/stats` | Dashboard stats (accounts, domains, limits, resource usage) |
| GET | `/api/veloserve/status` | VeloServe installed/running/version/PID/uptime |
| GET | `/api/veloserve/version` | VeloServe binary version string |
| POST | `/api/veloserve/regenerate` | Regenerate all vhost configs from DB + reload |
| POST | `/api/veloserve/reload` | Reload VeloServe configuration |

---

## Error Codes

| HTTP Status | Meaning |
|---|---|
| `200` | Success |
| `400` | Bad request — validation error, missing fields |
| `401` | Unauthorized — missing or invalid JWT token |
| `403` | Forbidden — license limit exceeded or feature not available |
| `500` | Internal server error |

---

## Quick-start Script

A complete example that sets up admin, creates an account, adds a domain, writes a page, creates a database, and provisions SSL:

```bash
#!/usr/bin/env bash
set -euo pipefail
PANEL="http://localhost:7070"

# 1. Setup admin (first run only)
curl -s -X POST "$PANEL/api/auth/setup" \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"SecureAdmin123"}'

# 2. Login
TOKEN=$(curl -s -X POST "$PANEL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"SecureAdmin123"}' \
  | jq -r '.data.token')
AUTH="Authorization: Bearer $TOKEN"

# 3. Create a hosting account
curl -s -X POST "$PANEL/api/accounts" \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"username":"johndoe","domain":"example.com","email":"john@example.com","password":"UserPass123","plan":"free"}'

# 4. Add an addon domain
curl -s -X POST "$PANEL/api/domains" \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"account_id":1,"domain_name":"blog.example.com","domain_type":"addon","document_root":"/home/johndoe/blog"}'

# 5. Write an index page
curl -s -X POST "$PANEL/api/files/write" \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"account_id":1,"path":"/public_html/index.html","content":"<h1>Welcome to example.com</h1>"}'

# 6. Create a MySQL database
curl -s -X POST "$PANEL/api/databases" \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"account_id":1,"db_name":"wordpress","db_password":"DbPass12345","engine":"mysql"}'

# 7. Provision SSL (requires DNS pointing to this server)
curl -s -X POST "$PANEL/api/ssl/provision" \
  -H "$AUTH" -H "Content-Type: application/json" \
  -d '{"domain":"example.com"}'

# 8. Check dashboard
curl -s "$PANEL/api/system/stats" -H "$AUTH" | jq .
```
