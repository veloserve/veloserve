# Changelog

## 1.0.6 - 2026-03-08

Fixes for WHM plugin visibility, API token creation, and WordPress plugin registration so Plugin + VeloServe work end-to-end on cPanel.

### WHM plugin

- Fixed plugin not showing in WHM sidebar: added `displayname=VeloServe` and `entryurl=veloserve/veloserve.cgi` to AppConfig so the plugin appears under Plugins and the link no longer 404s (no double `/cgi/` in URL).
- Added **Settings** page with **API Tokens**: create/revoke tokens for the WordPress (and other) plugins; tokens stored in `/etc/veloserve/api-tokens.txt` and synced to `veloserve.toml` `[api] tokens`.
- Token creation uses JSON API from the UI (POST with action in body) to avoid WHM redirect issues; copy token and use in WordPress Connection.
- Fixed wording (generic "Create API token", no WordPress-only labels) and replaced em dash with hyphen to avoid encoding issues.

### VeloServe server

- Added **POST /api/v1/wordpress/register**: WordPress plugin "Register Site with VeloServe" now succeeds; returns `node_id` (from site URL hash) and `registered_at` (JSON 200). Fixes "Registration failed with status 404".

### WordPress plugin

- Endpoint discovery: auto-detect now tries **port 80** first (`http://127.0.0.1`, `http://localhost`) for cPanel/WHM, then port 8080 for standalone, so the plugin connects without "Connection refused" on cPanel.
- Connection placeholder suggests `http://127.0.0.1 (cPanel)` vs `http://127.0.0.1:8080 (standalone)`.
- Docs: clarified cPanel = port 80, standalone = 8080 for Endpoint URL.

### Release note

After merge, tag **v1.0.6** to trigger the Release workflow and publish binaries; users can then use the WordPress plugin with VeloServe on cPanel (WHM → Settings → API Tokens, then wp-admin → VeloServe → Connection).

---

## 1.0.5 - 2026-03-08

This release packages the completed WordPress plugin work from Phase 1 and Phase 2, along with the QA fixes and documentation updates that were finished on `feat/vel-18-wordpress-plugin-v1`.

### Phase 1 completed

- Added local VeloServe server API detection using `/api/v1/status`
- Added cache stats retrieval and cache purge support through the local VeloServe API client
- Added a tabbed WordPress admin shell with sidebar submenu entries and admin bar cache actions
- Added a live dashboard with connection state, server runtime status, cache visibility, quick actions, and environment details
- Added General settings for auto-detect, guest mode, server IP override, notifications, and auto purge
- Added Cache sub-tabs for Cache, TTL, Optimization, and Purge flows
- Added policy-aware purge execution using VeloServe cache purge endpoints
- Added a smart purge engine for WordPress content, theme, plugin, upgrader, and WooCommerce events
- Added CDN purge cascading with Cloudflare provider support and connection testing
- Added documentation and QA hardening for the plugin and cPanel helper workflow

### Phase 2 completed

- Added page optimization controls for CSS, JavaScript, and HTML behavior
- Added settings for minify, combine, defer, critical CSS, and prefetch hints
- Added optimization payload data to WordPress site registration requests
- Added image optimization controls for lazy loading, WebP, AVIF, compression quality, and queueing
- Added image queue processing and cache warming hooks for optimized image targets

### QA and release readiness

- Fixed purge target handling so path-based invalidation is not silently dropped
- Fixed test harness stubs needed for WordPress admin sanitization and database tooling tests
- Passed plugin flow tests, cPanel helper fixture tests, live QEMU cPanel VM validation, browser admin UI verification, uninstall cleanup, and reinstall validation
- Added docs.veloserve.io onboarding for plugin setup, endpoint/API-token discovery, and WordPress operator workflows

### Release note

After merge to production, publish fresh binaries and release artifacts together so the completed Phase 1 and Phase 2 WordPress/plugin/cache changes ship in a single release.
