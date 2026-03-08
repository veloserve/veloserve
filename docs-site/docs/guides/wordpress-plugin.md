# VeloServe WordPress Plugin

The `veloserve-cache` plugin connects WordPress directly to VeloServe for server-level cache management, smart purge automation, CDN purge cascading, image optimization, and operational tooling.

## Requirements

- WordPress 6.0+
- PHP 7.4+
- VeloServe running on the same server or reachable over the network

## Installation

### WordPress Admin upload

1. Build the plugin zip:

```bash
cd wordpress-plugin
zip -r veloserve-cache.zip veloserve-cache
```

2. Open `Plugins -> Add New -> Upload Plugin`
3. Upload `veloserve-cache.zip`
4. Activate `VeloServe Cache`

### WP-CLI

```bash
wp plugin install /path/to/veloserve-cache.zip --activate
```

### cPanel helper deployment

Use the helper when you want to discover and deploy across cPanel accounts in bulk:

```bash
# Discover installs
cpanel/wordpress/veloserve-wordpress-helper.sh discover --home-root /home

# Deploy to one site
cpanel/wordpress/veloserve-wordpress-helper.sh install \
  --site-path /home/alice/public_html \
  --plugin-zip /path/to/veloserve-cache.zip
```

## Connecting to VeloServe

After activation, open `wp-admin -> VeloServe -> Connection`.

### Finding your Endpoint URL

| Deployment | Endpoint URL |
|-----------|--------------|
| Same server (cPanel / WHM) | `http://127.0.0.1` (VeloServe listens on port 80 after Apache swap) |
| Same server (standalone / dev) | `http://127.0.0.1:8080` |
| VeloPanel managed | Shown in VeloPanel under `Server -> API Endpoint` |
| Remote / cloud | Public or internal URL of the VeloServe node, for example `https://veloserve.example.com` |

!!! tip
    When `Auto-Detect Server` is enabled, the plugin probes common local endpoints automatically. If your VeloServe API is not exposed on the probed port, set `Endpoint URL` explicitly.

### Finding your API Token

=== "cPanel / WHM"

    1. Open `WHM -> VeloServe`
    2. Go to `Settings -> API Tokens`
    3. Copy the token for the WordPress site, or create a new token for WordPress plugin usage

=== "VeloPanel"

    1. Open `VeloPanel -> API Keys`
    2. Create a key with cache-management permissions such as `cache:purge` and `site:register`
    3. Copy the generated token into the plugin

=== "Standalone / Docker"

    Configure a token in `veloserve.toml`:

    ```toml
    [api]
    tokens = ["your-secret-token-here"]
    ```

    Generate a secure token with:

    ```bash
    openssl rand -hex 32
    ```

=== "Environment Variable"

    If your deployment reads API tokens from environment variables, use:

    ```bash
    export VELOSERVE_API_TOKEN="your-secret-token-here"
    ```

### Step-by-step connection flow

1. Open `wp-admin -> VeloServe -> Connection`
2. Enter `Endpoint URL`
3. Enter `API Token`
4. Click `Save Settings`
5. Click `Register Site with VeloServe`
6. Confirm the dashboard shows:
   - `Connection: Connected`
   - `Server Status: running`
   - a non-empty `Node ID`

!!! warning
    If connection or registration fails:

    - verify the endpoint is reachable from the WordPress host with `curl`
    - confirm the API token is valid and active
    - check local firewall or security rules blocking the API port
    - if using HTTPS, confirm PHP trusts the certificate chain

## Plugin tabs overview

### Dashboard

Shows live connection status, detected VeloServe version, quick actions, cache visibility, and environment details for the WordPress install.

### Connection

Stores `Endpoint URL` and `API Token`, and exposes the registration action.

### General

| Setting | Purpose |
|--------|---------|
| Auto-Detect Server | Probe local VeloServe runtime / API endpoints automatically |
| Guest Mode | Allow a reduced operator workflow |
| Server IP Override | Force requests to a specific IPv4 / IPv6 address |
| Notifications | Show admin success and error notices |
| Auto Purge | Purge cache automatically after supported WordPress events |

### Cache

The Cache section includes four sub-tabs:

- `Cache`: cache state and auto-purge behavior
- `TTL`: plugin TTL preference and server TTL visibility
- `Optimization`: CSS/JS/HTML optimization plus image settings
- `Purge`: purge policy and manual purge action

### CDN

Cloudflare is currently supported for purge cascading:

1. Open `wp-admin -> VeloServe -> CDN`
2. Enable `CDN Purge Cascade`
3. Select `Cloudflare`
4. Set `Cloudflare Zone ID`
5. Set either:
   - `Cloudflare API Token` (recommended)
   - or `Cloudflare Email` + `Cloudflare API Key`
6. Click `Test CDN Connection`
7. Save settings

### Tools

| Tool | Purpose |
|------|---------|
| Purge Cache Now | Run the current purge flow immediately |
| Optimize Database Tables | Run `OPTIMIZE TABLE` for WordPress-prefixed tables |
| Warm from Sitemap | Crawl sitemap entry points and submit URLs to the warm queue |
| Export Settings | Download plugin settings JSON |
| Import Settings | Upload and apply plugin settings JSON |
| Download Debug Snapshot | Export plugin, environment, and server diagnostics |

## Smart purge events

| WordPress event | Purge targets |
|-----------------|---------------|
| Post publish / update | Homepage, changed post URL, archive URLs, taxonomy URLs when available |
| Post delete / trash / untrash | Homepage and post-type archive |
| Theme switch | Homepage and REST index |
| Customizer save | Homepage and REST index |
| Plugin activate / deactivate | Homepage and REST index |
| Plugin / theme upgrade | Homepage and REST index |
| WooCommerce order status change | Homepage, `/shop/`, `/cart/`, `/checkout/`, `/my-account/` |

When CDN cascading is enabled, equivalent purge targets are sent to the configured CDN provider as well.

## Admin bar actions

The WordPress admin bar exposes a `VeloServe` dropdown with:

- `Register Site`
- `Purge All Cache`

## VeloServe API endpoints used by the plugin

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/v1/status` | `GET` | Detect VeloServe runtime and health |
| `/api/v1/cache/stats` | `GET` | Dashboard cache stats |
| `/api/v1/cache/config` | `GET` | Server cache configuration visibility |
| `/api/v1/cache/purge` | `POST` | Cache invalidation |
| `/api/v1/cache/warm` | `POST` | Warm queue submission |
| `/api/v1/wordpress/register` | `POST` | Site registration |

## Uninstall behavior

Deactivation marks the plugin connection as disconnected. Full uninstall removes plugin data from WordPress:

- `veloserve_settings`
- `veloserve_status`
- `veloserve_image_queue`
