# SSL & AutoSSL

VeloServe provides full HTTPS support with per-domain SSL certificates and seamless integration with cPanel's AutoSSL feature.

## TLS Architecture

VeloServe uses `rustls` (a pure-Rust TLS implementation) with SNI (Server Name Indication) to serve the correct certificate for each domain:

```
Client connects to port 443
    → TLS handshake includes SNI hostname (e.g., "example.com")
    → VeloServe looks up certificate for "example.com"
    → If found: use per-domain cert
    → If not found: use global fallback cert
    → Complete TLS handshake
    → Serve HTTP response
```

## Configuration

### Global SSL (Fallback)

The `[ssl]` section defines the default certificate used when no per-domain match is found:

```toml
[ssl]
cert = "/var/cpanel/ssl/cpanel/cpanel.pem"
key = "/var/cpanel/ssl/cpanel/cpanel.pem"
```

On cPanel servers, this is typically the server's self-signed certificate.

### Per-Domain SSL

Each virtual host can specify its own certificate:

```toml
[[virtualhost]]
domain = "example.com"
root = "/home/user/public_html"
ssl_certificate = "/var/cpanel/ssl/installed/certs/example_com.crt"
ssl_certificate_key = "/var/cpanel/ssl/installed/keys/example_com.key"
```

### Listening on Port 443

Ensure the server config includes the SSL listener:

```toml
[server]
listen = "0.0.0.0:80"
listen_ssl = "0.0.0.0:443"
```

## AutoSSL Integration

cPanel's AutoSSL automatically provisions free SSL certificates (typically Let's Encrypt or Sectigo) for all domains on the server.

### How It Works with VeloServe

1. **AutoSSL runs** on its regular schedule (or is triggered manually in WHM)
2. **Certificates are issued** and written to `/var/cpanel/ssl/installed/certs/` and `/var/cpanel/ssl/installed/keys/`
3. **cPanel fires the `SSLStorage::add_ssl` hook**
4. **VeloServe's hook script** updates the matching `[[virtualhost]]` with the new certificate paths
5. **VeloServe reloads** and begins serving the new certificate

This is fully automatic — no manual intervention required.

### Verifying AutoSSL

Check AutoSSL status in WHM:

1. Go to **WHM > SSL/TLS > Manage AutoSSL**
2. Verify AutoSSL is enabled
3. Run AutoSSL manually if needed

Or from the command line:

```bash
/usr/local/cpanel/bin/autossl_check --all
```

### Certificate Paths on cPanel

| Type | Path Pattern |
|------|-------------|
| Certificates | `/var/cpanel/ssl/installed/certs/<hash>.crt` |
| Private Keys | `/var/cpanel/ssl/installed/keys/<hash>.key` |
| CA Bundles | `/var/cpanel/ssl/installed/cabundles/<hash>.cabundle` |

VeloServe reads the certificate and key files directly — no intermediate conversion needed.

## Checking Certificate Status

### From the WHM Plugin

The **SSL/TLS** page in the VeloServe WHM plugin shows:

- Global certificate information
- Per-domain certificate table with issuer, expiry, and status
- Warnings for certificates expiring within 30 days

### From the Command Line

```bash
# Check a specific certificate
openssl x509 -in /var/cpanel/ssl/installed/certs/example_com.crt -noout -dates -subject -issuer

# Check what VeloServe is serving
openssl s_client -connect localhost:443 -servername example.com < /dev/null 2>/dev/null | openssl x509 -noout -dates -subject
```

## Renewing Certificates

Certificates provisioned by AutoSSL are renewed automatically before expiry. When a renewal occurs:

1. The new certificate files replace the old ones on disk
2. The `SSLStorage::add_ssl` hook fires
3. VeloServe picks up the new paths and reloads

If you need to force a renewal:

```bash
# Via WHM
# SSL/TLS > Manage AutoSSL > Run AutoSSL

# Via command line
/usr/local/cpanel/bin/autossl_check --user=username
```

## Troubleshooting

### HTTPS not working

1. Verify `listen_ssl` is set in your config:
   ```bash
   grep listen_ssl /etc/veloserve/veloserve.toml
   ```

2. Check that certificate files exist and are readable:
   ```bash
   ls -la /var/cpanel/ssl/installed/certs/
   ls -la /var/cpanel/ssl/installed/keys/
   ```

3. Verify VeloServe is listening on port 443:
   ```bash
   ss -tlnp | grep 443
   ```

### Certificate mismatch

If a browser shows the wrong certificate for a domain, check that the correct certificate is mapped in `veloserve.toml`:

```bash
grep -A 3 "example.com" /etc/veloserve/veloserve.toml
```

Re-import from Apache if needed:

```bash
./import-apache-and-swap.sh --config-only
systemctl reload veloserve
```

## Next Steps

- **[cPanel Hooks](hooks.md)** — the event system that keeps SSL in sync
- **[Configuration: SSL/TLS](../configuration/ssl-tls.md)** — full TLS config reference
- **[Apache Swap](apache-swap.md)** — how certificates are imported from Apache
