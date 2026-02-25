# SSL / TLS Configuration

VeloServe supports HTTPS via `rustls`, a pure-Rust TLS implementation, with HTTP/2 and SNI.

## Enabling HTTPS

```toml
[server]
listen = "0.0.0.0:80"
listen_ssl = "0.0.0.0:443"

[ssl]
cert = "/etc/veloserve/ssl/cert.pem"
key = "/etc/veloserve/ssl/key.pem"
```

## TLS Options

```toml
[tls]
enable = true
cert_file = "/etc/veloserve/ssl/cert.pem"
key_file = "/etc/veloserve/ssl/key.pem"
min_version = "1.2"
alpn = ["h2", "http/1.1"]
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable` | bool | `false` | Enable TLS listener |
| `cert_file` | string | required | Path to certificate PEM file |
| `key_file` | string | required | Path to private key PEM file |
| `min_version` | string | `"1.2"` | Minimum TLS version (`"1.2"` or `"1.3"`) |
| `alpn` | array | `["h2", "http/1.1"]` | ALPN protocol negotiation |

## Global SSL (Fallback Certificate)

The `[ssl]` section provides a fallback certificate for requests that don't match any per-vhost certificate:

```toml
[ssl]
cert = "/etc/veloserve/ssl/server.crt"
key = "/etc/veloserve/ssl/server.key"
```

## Per-Domain SSL (SNI)

Each virtual host can specify its own certificate for SNI-based resolution:

```toml
[[virtualhost]]
domain = "example.com"
root = "/var/www/example.com"
ssl_certificate = "/etc/ssl/certs/example.com.crt"
ssl_certificate_key = "/etc/ssl/private/example.com.key"

[[virtualhost]]
domain = "shop.example.com"
root = "/var/www/shop"
ssl_certificate = "/etc/ssl/certs/shop.example.com.crt"
ssl_certificate_key = "/etc/ssl/private/shop.example.com.key"
```

When a TLS client connects with SNI, VeloServe looks up the matching virtualhost's certificate. If no match is found, the global `[ssl]` certificate is used.

## Let's Encrypt / ACME

VeloServe works with any ACME client. For example, with `certbot`:

```bash
certbot certonly --webroot -w /var/www/example.com -d example.com
```

Then configure:

```toml
[[virtualhost]]
domain = "example.com"
ssl_certificate = "/etc/letsencrypt/live/example.com/fullchain.pem"
ssl_certificate_key = "/etc/letsencrypt/live/example.com/privkey.pem"
```

Reload VeloServe after certificate renewal:

```bash
systemctl reload veloserve
```

## Self-Signed Certificates

For development or testing:

```bash
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes \
  -subj "/CN=localhost"
```

```toml
[ssl]
cert = "./cert.pem"
key = "./key.pem"
```
