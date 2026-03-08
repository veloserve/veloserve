# VeloServe WordPress Plugin v1

This document covers plugin architecture, install/activation flow, cPanel discovery automation, rollback, and troubleshooting.

## Overview

Plugin slug: `veloserve-cache`

The plugin is **server-agnostic** and works on any WordPress installation -- cPanel, VeloPanel
standalone, Docker, bare metal, or cloud hosting. It uses only standard WordPress APIs with no
hosting-environment dependencies. The cPanel helper script is an optional automation layer for
bulk deployment on cPanel servers.

Primary outcomes in v1:

- registers WordPress site with VeloServe endpoint
- exposes admin status and action controls with success/error notices
- purges cache on content mutations, theme switches, and customizer saves (when enabled)
- supports optional CDN purge cascading with Cloudflare provider integration
- provides manual Purge All Cache button
- supports cPanel automation through helper contract

## Plugin Architecture

Path: `wordpress-plugin/veloserve-cache`

- `veloserve-cache.php`: plugin bootstrap and lifecycle hooks
- `includes/class-veloserve-client.php`: endpoint communication for registration
- `includes/class-veloserve-admin.php`: admin UI/settings/actions
- `includes/class-veloserve-plugin.php`: state model and content-change purge hooks
- `includes/class-veloserve-cdn-manager.php`: CDN provider abstraction and routing
- `includes/class-veloserve-cdn-cloudflare-provider.php`: Cloudflare API integration (test + purge)
- `uninstall.php`: removes plugin options

## Prerequisites

- WordPress 6.0+
- PHP 7.4+
- Outbound HTTPS from WordPress host to VeloServe endpoint
- Admin credentials for plugin activation/configuration

## Install and Activation

### Manual installation

1. Build package:

```bash
cd wordpress-plugin
zip -r veloserve-cache.zip veloserve-cache
```

2. In WordPress Admin: `Plugins -> Add New -> Upload Plugin`
3. Upload `veloserve-cache.zip`
4. Activate plugin

### Initial configuration

1. Open `wp-admin -> VeloServe`
2. Set `Endpoint URL` and `API Token`
3. Open `General` tab and configure:
- `Auto-Detect Server` for runtime/API discovery
- `Guest Mode` for read-only operator workflows
- optional `Server IP Override` (IPv4/IPv6)
- `Notifications` for admin success/error notices
4. Keep `Auto Purge` enabled unless debugging cache behavior
5. Click `Register Site with VeloServe`
6. Verify status shows:
- `Connection: Connected`
- non-empty `Node ID`

## cPanel Discovery and Install Flow

Helper script path: `cpanel/wordpress/veloserve-wordpress-helper.sh`

### Discovery

```bash
cpanel/wordpress/veloserve-wordpress-helper.sh discover --home-root /home
```

This scans for `wp-config.php` under account web roots and returns JSON suitable for orchestration.

### Plugin deployment for one discovered site

```bash
cpanel/wordpress/veloserve-wordpress-helper.sh install \
  --site-path /home/alice/public_html \
  --plugin-zip /usr/local/src/veloserve/wordpress-plugin/veloserve-cache.zip
```

### Validation fixture

```bash
cpanel/tests/wordpress-helper-fixture.sh
```

## Rollback

### Plugin rollback only

1. `Plugins -> Installed Plugins`
2. Deactivate `VeloServe Cache`
3. Delete plugin from WordPress UI (or remove plugin folder)

### Full rollback via filesystem

```bash
rm -rf /path/to/site/wp-content/plugins/veloserve-cache
```

Optional cleanup:
- if uninstall flow did not run, remove `veloserve_settings` and `veloserve_status` options from WordPress DB.

## Troubleshooting

### Registration fails

Checks:

- verify endpoint URL is reachable from host
- confirm API token validity
- inspect WordPress HTTP transport restrictions/firewall

### Status stays disconnected

Checks:

- click `Register Site with VeloServe` manually
- inspect plugin settings for blank endpoint/token
- inspect any reverse proxy rules blocking outbound API calls

### cPanel helper install fails

Checks:

- `wp-config.php` exists at `--site-path`
- plugin zip contains `veloserve-cache/veloserve-cache.php`
- rerun with `--force` when replacing existing install

## Cache Purge

The plugin automatically sends cache purge requests to the VeloServe endpoint when:

- content is published/updated (`save_post`) with targeted invalidation of:
  - homepage
  - changed post URL
  - relevant archive/taxonomy URLs (when available)
- content is deleted/trashed/untrashed (`deleted_post`, `trashed_post`, `untrashed_post`)
- the active theme is switched (`switch_theme`)
- customizer settings are saved (`customize_save_after`)
- plugins are activated/deactivated (`activated_plugin`, `deactivated_plugin`)
- plugin/theme upgrades complete (`upgrader_process_complete`)
- WooCommerce order state changes (`woocommerce_order_status_changed`) with targeted storefront path purges (`/shop/`, `/cart/`, `/checkout/`, `/my-account/`)

A **Purge All Cache** button is available on the admin settings page for manual full-site purges.

## CDN Integration (Cloudflare)

The plugin includes a CDN abstraction layer with a Cloudflare provider.

Configuration path:

1. Open `wp-admin -> VeloServe -> CDN`
2. Enable `CDN Purge Cascade`
3. Select provider `Cloudflare`
4. Set `Cloudflare Zone ID`
5. Set either:
- `Cloudflare API Token` (recommended), or
- `Cloudflare Email` + `Cloudflare API Key` (legacy fallback)
6. Click `Test CDN Connection`

When enabled, purge operations cascade to Cloudflare using equivalent targets:
- URL/domain+path purge -> Cloudflare file purge
- domain purge -> Cloudflare host purge
- tag purge -> Cloudflare tag purge
- full purge policy -> Cloudflare `purge_everything`

Auto-purge can be disabled via the `Auto Purge` checkbox in plugin settings.

## Test Coverage

Plugin flow tests: `wordpress-plugin/tests/plugin-flows-test.php`

Covered:

- activation creates options
- option persistence in settings store
- successful endpoint registration updates node state
- non-2xx registration is reported as failure
- content change triggers purge request
- plugin lifecycle events trigger purge requests
- CDN connection test validates Cloudflare API auth and zone lookup
- CDN purge cascade triggers Cloudflare API purge requests when enabled
- WooCommerce order status change triggers storefront purges
- theme switch triggers purge request
- auto_purge disabled suppresses purge
- deactivation marks disconnected state

cPanel helper fixture tests: `cpanel/tests/wordpress-helper-fixture.sh`

Covered:

- discovery finds WordPress installs under home directories
- install deploys plugin zip to correct location
- installed plugin files are present after deployment

Run:

```bash
wordpress-plugin/tests/run-tests.sh
cpanel/tests/wordpress-helper-fixture.sh
```
