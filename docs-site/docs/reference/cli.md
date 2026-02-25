# CLI Reference

VeloServe command-line interface reference.

## Usage

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

| Option | Description | Default |
|--------|-------------|---------|
| `--root <PATH>` | Document root | from config |
| `--listen <ADDR>` | Listen address | `0.0.0.0:8080` |
| `--workers <NUM>` | Worker threads | auto |
| `--daemon` | Run in background | false |
| `--foreground` | Run in foreground | false |
| `--pid-file <PATH>` | PID file location | `/var/run/veloserve.pid` |

```bash
# Quick start
veloserve start --root /var/www/html

# Custom port
veloserve start --root /var/www --listen 0.0.0.0:3000

# With config file
veloserve --config /etc/veloserve/veloserve.toml start

# As daemon
veloserve start --daemon --pid-file /var/run/veloserve.pid
```

### stop

Stop the running server (sends SIGTERM).

```bash
veloserve stop
```

### restart

Restart the server (stop + start).

```bash
veloserve restart
```

### status

Show server status.

```bash
veloserve status
```

### config

Configuration management commands.

```bash
# Test config for errors
veloserve config test [--config <FILE>]

# Show parsed config
veloserve config show [--config <FILE>]

# Print default config template
veloserve config show-default

# Reload config without restart
veloserve config reload

# Convert Apache config
veloserve config convert-apache --input <FILE> --output <FILE> [--vhosts-only]
```

### cache

Cache management commands.

```bash
# Show cache statistics
veloserve cache stats

# Purge all cache
veloserve cache purge --all

# Purge by domain
veloserve cache purge --domain example.com

# Purge by URL pattern
veloserve cache purge --pattern "/blog/*"

# Purge by tag
veloserve cache purge --tag "post-123"

# Warm cache from sitemap
veloserve cache warm --sitemap https://example.com/sitemap.xml
```

### php

PHP information commands.

```bash
# Show PHP configuration
veloserve php info

# Test PHP execution
veloserve php test
```

### version

Show detailed version information.

```bash
veloserve version
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
