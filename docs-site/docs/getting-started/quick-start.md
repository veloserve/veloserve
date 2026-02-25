# Quick Start

Get VeloServe running in under 2 minutes.

## One-Line Install

```bash
curl -sSL https://veloserve.io/install.sh | bash
```

This automatically:

- Detects your OS (Linux, macOS, Windows)
- Downloads the correct binary
- Installs to `/usr/local/bin`
- Creates a default config at `/etc/veloserve/veloserve.toml`

Or with wget:

```bash
wget -qO- https://veloserve.io/install.sh | bash
```

## Start Serving

### Quick Test

```bash
# Create a test directory
mkdir -p /tmp/mysite
echo '<?php phpinfo();' > /tmp/mysite/index.php
echo '<h1>Hello VeloServe!</h1>' > /tmp/mysite/index.html

# Start server
veloserve start --root /tmp/mysite --listen 0.0.0.0:8080
```

Visit [http://localhost:8080](http://localhost:8080) — you should see the PHP info page.

### Using a Config File

```bash
# Start with the default config
veloserve --config /etc/veloserve/veloserve.toml

# Or specify a custom config
veloserve --config ./mysite.toml
```

## Minimal Configuration

Create `veloserve.toml`:

```toml
[server]
listen = "0.0.0.0:8080"

[php]
enable = true

[[virtualhost]]
domain = "*"
root = "/var/www/html"
```

This gives you a working server with PHP on port 8080, serving files from `/var/www/html`.

## What's Next?

- **[Installation](installation.md)** — all installation methods (binary, source, Docker)
- **[CGI Mode](../standalone/cgi-mode.md)** — simple and portable PHP execution
- **[SAPI Mode](../standalone/sapi-mode.md)** — maximum performance with embedded PHP
- **[cPanel Integration](../cpanel/overview.md)** — replace Apache on cPanel servers
- **[Configuration Reference](../configuration/reference.md)** — every config option explained
