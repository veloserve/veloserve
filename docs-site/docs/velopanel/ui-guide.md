# VeloPanel UI Guide

VeloPanel ships with a modern dark-themed web interface built with Svelte 5. Access it at `http://your-server:7070` after installation.

## Initial Setup

On first launch, VeloPanel shows the **Setup** screen where you create your administrator account.

1. Open `http://your-server:7070` in your browser
2. Enter a **username** (min 3 characters) and **password** (min 8 characters)
3. Confirm the password and click **Create Admin Account**
4. You'll be automatically logged in and redirected to the Dashboard

!!! tip
    The setup screen is only available once. After the admin is created, all subsequent visits show the login page.

## Login

After setup, use the **Login** page to sign in:

- Enter your **username** and **password**
- Click **Sign In**
- A JWT session token is stored as an HTTP-only cookie (expires after 24 hours)

## Navigation

The sidebar provides access to all sections:

| Icon | Section | Description |
|---|---|---|
| ◈ | **Dashboard** | System overview and resource stats |
| ⊡ | **Accounts** | Manage hosting accounts |
| ◎ | **Domains** | Manage domains per account |
| ⊘ | **SSL** | SSL certificate management |
| ⟐ | **PHP** | PHP version configuration |
| ⊞ | **Files** | Web-based file manager |
| ⊟ | **Databases** | MySQL/PostgreSQL database management |
| ⊙ | **System** | Server info and VeloServe controls |

The sidebar also shows:

- **License tier** badge (Community, Pro, Business)
- **Current user** with avatar and role
- **Logout** button

---

## Dashboard

The dashboard provides a real-time overview of your server:

### Stat Cards

Four summary cards at the top:

- **Accounts** — Number of hosting accounts (e.g. "3 / 5" on Community tier)
- **Domains** — Total domains across all accounts (e.g. "7 / 10")
- **Disk Usage** — Used vs total disk space
- **License** — Current license tier

### Server Information

A detail panel showing:

- Hostname, OS, Kernel version
- Uptime
- CPU model and core count
- RAM usage (used / total)

### VeloServe Status

Shows whether VeloServe is running, its version, PID, and uptime. Includes a **Restart** button to reload the web server.

---

## Accounts

### Viewing Accounts

The accounts page shows a table with all hosting accounts:

| Column | Description |
|---|---|
| Username | System username (links to account) |
| Domain | Primary domain |
| Email | Contact email |
| Plan | Account plan (free, pro, business) |
| Status | Active / Suspended badge |
| Actions | Suspend, Unsuspend, Delete buttons |

### Creating an Account

Click **+ Create Account** to open the creation form:

1. **Username** — 3-16 alphanumeric characters (becomes the Linux system user)
2. **Primary Domain** — The main domain for this account
3. **Email** — Contact email address
4. **Password** — Min 8 characters (used for the account's system password)
5. **Plan** — Select from Basic, Standard, Premium, Enterprise

When you create an account, VeloPanel automatically:

- Creates a Linux system user with a home directory
- Creates `/home/{username}/public_html`
- Adds the primary domain to the database
- Generates a VeloServe vhost configuration
- Reloads VeloServe to serve the new site

!!! warning "Community Tier Limit"
    On the free Community license, you can create up to **5 accounts**. The UI will show an error when the limit is reached.

### Suspending / Unsuspending

- Click **Suspend** to deactivate an account (status changes to "Suspended")
- Click **Unsuspend** to re-activate it

### Deleting an Account

Click **Delete** and confirm the dialog. This removes:

- The Linux system user and home directory
- All associated domains and vhost configs
- Reloads VeloServe

---

## Domains

### Viewing Domains

The domains page lists all domains across all accounts:

| Column | Description |
|---|---|
| Domain | Fully qualified domain name |
| Type | Primary, Addon, Subdomain, or Alias |
| Account | Owning account username |
| SSL | Active (green badge) or None |
| Document Root | Path on disk |
| Actions | Delete button |

### Adding a Domain

Click **+ Add Domain** to open the modal:

1. **Account** — Select the owning account from the dropdown
2. **Domain Name** — Fully qualified domain (e.g. `blog.example.com`)
3. **Type** — Addon, Subdomain, or Alias
4. **Document Root** — Absolute path (e.g. `/home/user1/blog`)

VeloPanel automatically generates a VeloServe vhost config and reloads the web server.

!!! warning "Community Tier Limit"
    The Community license allows up to **10 total domains** across all accounts.

### Deleting a Domain

Click **Delete** and confirm. The vhost config is removed and VeloServe is reloaded.

---

## SSL Certificates

### Viewing Certificates

Lists all SSL certificates in `/etc/veloserve/ssl/` with:

| Column | Description |
|---|---|
| Domain | Certificate domain name |
| Issuer | Certificate authority (e.g. Let's Encrypt) |
| Issued | Issue date |
| Expires | Expiry date |
| Status | Valid (green), Expiring soon (yellow), Expired (red) |

### Provisioning a Certificate

Click **+ Provision SSL** to open the modal:

1. Enter the **domain name**
2. Click **Provision**

VeloPanel uses the built-in ACME client to:

- Register with Let's Encrypt (using the configured `acme_email`)
- Place HTTP-01 challenge files in the domain's webroot
- Obtain and save the certificate and private key
- Update the VeloServe vhost config with SSL paths
- Reload VeloServe

!!! note "Requirements"
    - The domain's DNS must point to your server
    - Port 80 must be accessible from the internet
    - `acme_email` must be set in the VeloPanel config

### Auto-Renewal

Use the API endpoint `POST /api/ssl/auto-renew` to bulk-renew all certificates expiring within 30 days. Set up a daily cron job for automated renewal:

```bash
0 3 * * * curl -s -X POST http://localhost:7070/api/ssl/auto-renew \
  -H "Authorization: Bearer YOUR_TOKEN"
```

---

## File Manager

The file manager provides a web-based interface to browse and edit files within account home directories.

### Browsing

- Select an **account** from the dropdown at the top
- Navigate using the **breadcrumb trail** or by clicking folders
- Click **..** to go up one level

Each file/folder shows:

| Column | Description |
|---|---|
| Name | File/folder name with icon (📁/📄) |
| Size | File size (formatted) |
| Modified | Last modified date |

### Editing Files

Click any file to open the built-in code editor:

- Full-width textarea with monospace font
- Click **Save** to write changes
- Click **Cancel** to discard and return to the directory view

### Other Operations

- **Upload** — Click the Upload button in the header to upload files
- **Create Directory** — Use the API: `POST /api/files/mkdir`
- **Set Permissions** — Use the API: `POST /api/files/chmod`
- **Delete** — Use the API: `DELETE /api/files/delete`

!!! info "Security"
    All file operations are sandboxed to the account's home directory. Path traversal attempts are blocked.

---

## Databases

### Selecting an Account

Use the **account dropdown** at the top to switch between accounts.

### Viewing Databases

The table shows databases for the selected account:

| Column | Description |
|---|---|
| Database Name | Full database name (prefixed with account username) |
| User | Database user |
| Size | Database size (if available) |
| Tables | Number of tables (if available) |
| Actions | Delete button |

### Creating a Database

Click **+ Create Database** to open the modal:

1. **Database Name** — Name for the database (will be prefixed with `{username}_`)
2. **Database User** — Username for database access
3. **User Password** — Min 8 characters

VeloPanel creates both the database and a dedicated user with full privileges on that database only.

### Deleting a Database

Click **Delete** and confirm. Both the database and its user are dropped.

### Available Engines

The page shows which database engines are available on the server (MySQL, MariaDB, PostgreSQL).

---

## PHP Configuration

### Detected PHP Versions

The top section lists all PHP versions installed on the server, showing:

- Version number (e.g. `8.3.6`)
- Binary path (e.g. `/usr/bin/php`)
- **Default** badge for the system default version

### Per-Account Configuration

A table lists each account with a dropdown to select the PHP version:

| Column | Description |
|---|---|
| Account | Account username |
| Domain | Primary domain |
| PHP Version | Dropdown to select version |
| Action | **Apply** button to save changes |

Select a version and click **Apply** to change the PHP version for that account.

!!! info "LSAPI Mode"
    The LSAPI execution mode is available only with a Pro or Business license.

---

## System Overview

### Server Information

Displays detailed server specs:

- Hostname, Operating System, Kernel, Architecture
- CPU model and core count
- System uptime

### Resource Usage

Four resource cards with visual progress bars:

- **Memory** — Used / Total with percentage bar
- **Disk** — Used / Total with percentage bar
- **Load Average** — 1m, 5m, 15m averages
- **Processes** — Total running processes

### VeloServe Control

Shows VeloServe status with a colored indicator:

- **Running** (green) / **Stopped** (red)
- Version, PID, Uptime
- **Restart VeloServe** button — stops and restarts the web server service

Click **Refresh** in the page header to reload all data.

---

## Keyboard Shortcuts

The VeloPanel UI is fully mouse-driven. All modals can be closed by:

- Clicking the backdrop (area outside the modal)
- Clicking the **Cancel** button

## Responsive Design

VeloPanel adapts to different screen sizes:

- **Desktop** — Full sidebar with labels + main content area
- **Tablet** — Sidebar collapses to icons only
- **Mobile** — Sidebar hidden, accessible via hamburger menu
