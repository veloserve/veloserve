# WordPress Helper Contract for cPanel

`veloserve-wordpress-helper.sh` provides a stable interface for cPanel/WHM automation when managing the VeloServe WordPress plugin.

## Commands

```bash
# Enumerate candidate installs (JSON)
cpanel/wordpress/veloserve-wordpress-helper.sh discover --home-root /home

# Install plugin package into one site
cpanel/wordpress/veloserve-wordpress-helper.sh install \
  --site-path /home/alice/public_html \
  --plugin-zip /usr/local/src/veloserve/wordpress-plugin/veloserve-cache.zip
```

## Discovery Contract

Response shape:

```json
{
  "generated_at": "2026-03-07T21:00:00Z",
  "sites": [
    {
      "user": "alice",
      "path": "/home/alice/public_html",
      "wp_config": "/home/alice/public_html/wp-config.php",
      "status": "discovered"
    }
  ]
}
```

## Install Contract

Successful install response:

```json
{
  "status": "installed",
  "site_path": "/home/alice/public_html",
  "plugin_dir": "/home/alice/public_html/wp-content/plugins/veloserve-cache"
}
```

### Error semantics

- non-zero exit with stderr message for invalid args or missing `wp-config.php`
- non-zero exit if plugin already installed and `--force` is not supplied
- non-zero exit if plugin zip does not contain `veloserve-cache/veloserve-cache.php`

## Executable fixture

Run:

```bash
cpanel/tests/wordpress-helper-fixture.sh
```

This verifies both discovery and install behavior using temp directories.
