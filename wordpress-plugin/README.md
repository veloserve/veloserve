# VeloServe WordPress Plugin (v1)

This directory contains the first shippable iteration of the VeloServe WordPress plugin.

The plugin is **server-agnostic** -- it works on any WordPress installation regardless of hosting
environment (cPanel, VeloPanel standalone, Docker, bare metal, cloud). It uses only standard
WordPress HTTP and options APIs; there are no cPanel-specific dependencies. The separate
cPanel helper script (`cpanel/wordpress/veloserve-wordpress-helper.sh`) provides optional
automation for bulk discovery and deployment on cPanel servers.

Current plugin capabilities include:

- Site registration with VeloServe control plane
- Cache controls (TTL, purge policies, targeted purge automation)
- CDN purge cascading (Cloudflare)
- Page optimization controls (CSS/JS/HTML minify/combine/defer, critical CSS, prefetch hints)

## Layout

- `veloserve-cache/`: installable plugin source
- `tests/`: lightweight flow tests (activation, settings persistence, registration, cache purge)

## Packaging

Create a distributable zip:

```bash
cd wordpress-plugin
zip -r veloserve-cache.zip veloserve-cache
```

## Tests

```bash
wordpress-plugin/tests/run-tests.sh
```

The test runner requires a local `php` binary.
