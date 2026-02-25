# cPanel Hooks

VeloServe registers standardized hooks with cPanel's event system to automatically update its configuration when changes happen through cPanel or WHM.

## How It Works

cPanel triggers hooks at various lifecycle events. VeloServe's hook script (`veloserve-hook.sh`) listens for these events and updates `/etc/veloserve/veloserve.toml` accordingly, then reloads VeloServe.

```
cPanel Event → manage_hooks → veloserve-hook.sh → Update veloserve.toml → Reload VeloServe
```

## Registered Events

| Category | Event | Action |
|----------|-------|--------|
| Accounts | `Accounts::Create` | Add a new virtual host for the account's main domain |
| Accounts | `Accounts::Remove` | Remove all virtual hosts under the user's home directory |
| Addon Domains | `AddonDomain::addaddondomain` | Add a virtual host for the addon domain |
| Addon Domains | `AddonDomain::deladdondomain` | Remove the addon domain's virtual host |
| Subdomains | `SubDomain::addsubdomain` | Add a virtual host for the subdomain |
| Subdomains | `SubDomain::delsubdomain` | Remove the subdomain's virtual host |
| Parked Domains | `Park::park` | Add a virtual host for the parked domain |
| Parked Domains | `Park::unpark` | Remove the parked domain's virtual host |
| SSL | `SSLStorage::add_ssl` | Update SSL certificate paths for the domain |
| SSL | `SSLStorage::delete_ssl` | Remove SSL certificate configuration |

## Event Data

cPanel passes event data as JSON on stdin. The hook script parses the relevant fields:

### Account Creation

```json
{
  "data": {
    "user": "newuser",
    "domain": "newdomain.com",
    "homedir": "/home/newuser"
  }
}
```

The hook adds:

```toml
[[virtualhost]]
domain = "newdomain.com"
root = "/home/newuser/public_html"
platform = "generic"
```

### Account Removal

```json
{
  "data": {
    "user": "olduser"
  }
}
```

!!! note
    The `Accounts::Remove` event only provides the username, not the domain. The hook removes all `[[virtualhost]]` entries whose `root` is under `/home/olduser/`.

### SSL Provisioning

```json
{
  "data": {
    "domain": "example.com",
    "cert_file": "/var/cpanel/ssl/installed/certs/example_com.crt",
    "key_file": "/var/cpanel/ssl/installed/keys/example_com.key"
  }
}
```

The hook updates the matching virtualhost:

```toml
[[virtualhost]]
domain = "example.com"
ssl_certificate = "/var/cpanel/ssl/installed/certs/example_com.crt"
ssl_certificate_key = "/var/cpanel/ssl/installed/keys/example_com.key"
```

## Hook Registration

Hooks are registered during plugin installation via `hooks/install-hooks.sh`:

```bash
/usr/local/cpanel/bin/manage_hooks add script \
  /usr/local/veloserve/cpanel/hooks/veloserve-hook.sh \
  --manual --category Whostmgr --event Accounts::Create --stage post
```

Each event is registered with `--stage post` so the hook runs after cPanel has completed its own processing.

## Hook Script Location

The hook script is installed to:

```
/usr/local/veloserve/cpanel/hooks/veloserve-hook.sh
```

It also responds to cPanel's `--describe` flag, returning a JSON descriptor of all registered events.

## Logging

Hook activity is logged to:

```
/var/log/veloserve/hooks.log
```

View recent hook activity:

```bash
tail -f /var/log/veloserve/hooks.log
```

Or use the WHM Plugin's Logs page.

## Manual Hook Management

### List registered hooks

```bash
/usr/local/cpanel/bin/manage_hooks list
```

### Remove hooks

```bash
/usr/local/cpanel/bin/manage_hooks delete script \
  /usr/local/veloserve/cpanel/hooks/veloserve-hook.sh
```

### Re-register hooks

```bash
cd /path/to/veloserve/cpanel
bash hooks/install-hooks.sh
```

## Troubleshooting

### Hooks not firing

1. Verify registration: `/usr/local/cpanel/bin/manage_hooks list | grep veloserve`
2. Check that the script is executable: `ls -la /usr/local/veloserve/cpanel/hooks/veloserve-hook.sh`
3. Test the describe output: `/usr/local/veloserve/cpanel/hooks/veloserve-hook.sh --describe`

### Config not updating

1. Check the hook log: `tail -20 /var/log/veloserve/hooks.log`
2. Verify `veloserve.toml` permissions: `ls -la /etc/veloserve/veloserve.toml`
3. Run a manual test: create a cPanel account and check if the vhost appears in `veloserve.toml`

## Next Steps

- **[SSL & AutoSSL](ssl-autossl.md)** — how SSL certificate provisioning integrates
- **[tailwatchd](tailwatchd.md)** — service monitoring
- **[WHM Plugin](whm-plugin.md)** — manage hooks from the UI
