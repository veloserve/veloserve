# CLI Commands

VeloServe command-line interface reference.

## Basic Usage

```bash
veloserve [OPTIONS] [COMMAND]
```

## Global Options

| Option | Short | Description |
|--------|-------|-------------|
| `--config <FILE>` | `-c` | Configuration file path |
| `--help` | `-h` | Show help message |
| `--version` | `-V` | Show version |

## Commands

### start

Start the web server.

```bash
veloserve start [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--root <PATH>` | Document root | from config |
| `--listen <ADDR>` | Listen address | `0.0.0.0:8080` |
| `--workers <NUM>` | Worker threads | auto |
| `--daemon` | Run in background | false |
| `--pid-file <PATH>` | PID file location | `/var/run/veloserve.pid` |

**Examples:**

```bash
# Start with defaults
veloserve start

# Quick start with root directory
veloserve start --root /var/www/html

# Start on custom port
veloserve start --root /var/www --listen 0.0.0.0:3000

# Start as daemon
veloserve start --daemon --pid-file /var/run/veloserve.pid

# Start with config file
veloserve --config /etc/veloserve/veloserve.toml start
```

### stop

Stop the running server.

```bash
veloserve stop
```

Sends SIGTERM to the running server process.

### restart

Restart the server.

```bash
veloserve restart
```

Equivalent to `stop` followed by `start`.

### status

Show server status.

```bash
veloserve status
```

**Output:**

```
VeloServe Status
================
Status: Running
PID: 12345
Uptime: 2h 34m 12s
Workers: 4
Active connections: 127
Requests served: 1,234,567
```

### config

Configuration management commands.

#### config test

Test configuration file for errors.

```bash
veloserve config test [--config <FILE>]
```

**Example:**

```bash
veloserve config test --config /etc/veloserve/veloserve.toml
# Output: ✓ Configuration is valid.
```

#### config show

Show parsed configuration.

```bash
veloserve config show [--config <FILE>]
```

#### config show-default

Print default configuration template.

```bash
veloserve config show-default > veloserve.toml
```

#### config reload

Reload configuration without restart (graceful).

```bash
veloserve config reload
```

Sends SIGHUP to reload configuration.

### cache

Cache management commands.

#### cache stats

Show cache statistics.

```bash
veloserve cache stats
```

**Output:**

```
Cache Statistics
================
Backend: memory
Size: 156M / 256M (61%)
Entries: 4,521
Hit rate: 94.2%
Hits: 1,156,789
Misses: 70,234
```

#### cache purge

Purge cache entries.

```bash
# Purge all
veloserve cache purge --all

# Purge specific domain
veloserve cache purge --domain example.com

# Purge by URL pattern
veloserve cache purge --pattern "/blog/*"

# Purge by tag
veloserve cache purge --tag "post-123"
```

#### cache warm

Warm up cache via the internal `/api/v1/cache/warm` queue.

```bash
veloserve cache warm --url https://example.com/
veloserve cache warm --urls warm-targets.txt --api http://127.0.0.1:8080
veloserve cache warm --deterministic --api http://127.0.0.1:8080
```

### php

PHP information commands.

#### php info

Show PHP configuration.

```bash
veloserve php info
```

**Output:**

```
PHP Information
===============
Mode: CGI (php-cgi)
Binary: /usr/bin/php-cgi
Version: 8.3.6
Workers: 4
Memory limit: 256M
Extensions: curl, gd, mbstring, mysql, opcache, xml, zip
```

#### php test

Test PHP execution.

```bash
veloserve php test
```

Runs a simple PHP script to verify PHP is working.

### version

Show version information.

```bash
veloserve version
```

**Output:**

```
VeloServe 1.0.0
PHP mode: CGI (php-cgi 8.3.6)
Built with: Rust 1.75.0
Features: http2, tls, cache
```

## Examples

### Development

```bash
# Quick dev server
veloserve start --root ./public --listen 127.0.0.1:8000

# With debug logging
RUST_LOG=debug veloserve start --root ./public
```

### Production

```bash
# Start with production config
veloserve --config /etc/veloserve/production.toml start --daemon

# Check status
veloserve status

# Graceful reload after config change
veloserve config reload

# View cache stats
veloserve cache stats
```

### WordPress

```bash
# Test WordPress config
veloserve config test --config /etc/veloserve/wordpress.toml

# Start WordPress server
veloserve --config /etc/veloserve/wordpress.toml start

# Purge WordPress cache after update
veloserve cache purge --tag "wordpress"
```

### Troubleshooting

```bash
# Verbose output
RUST_LOG=veloserve=debug veloserve start

# Test config
veloserve config test

# Check PHP
veloserve php test

# View all settings
veloserve config show
```

## Exit Codes

| Code | Description |
|------|-------------|
| 0 | Success |
| 1 | General error |
| 2 | Configuration error |
| 3 | PHP error |
| 4 | Permission error |
| 5 | Port already in use |

## Signals

| Signal | Action |
|--------|--------|
| `SIGTERM` | Graceful shutdown |
| `SIGINT` | Graceful shutdown (Ctrl+C) |
| `SIGHUP` | Reload configuration |
| `SIGUSR1` | Reopen log files |
