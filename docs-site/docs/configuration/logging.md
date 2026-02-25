# Logging Configuration

VeloServe supports configurable logging for debugging, access tracking, and error reporting.

## Configuration

```toml
[logging]
level = "info"
format = "combined"
stdout = true
file = "/var/log/veloserve/veloserve.log"
rotate = true
rotate_size = "100M"
rotate_keep = 10
```

## Options Reference

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `level` | string | `"info"` | Log level: `trace`, `debug`, `info`, `warn`, `error` |
| `format` | string | `"combined"` | Log format: `combined`, `common`, `json` |
| `stdout` | bool | `true` | Log to stdout (useful for containers) |
| `file` | string | none | Log file path |
| `rotate` | bool | `false` | Enable log rotation |
| `rotate_size` | string | `"100M"` | Rotate when file reaches this size |
| `rotate_keep` | int | `10` | Number of rotated files to keep |

## Log Levels

| Level | Description |
|-------|-------------|
| `trace` | Very detailed debugging information |
| `debug` | Debugging information |
| `info` | General operational events |
| `warn` | Warnings that may need attention |
| `error` | Errors that need investigation |

## Per-Vhost Access Logs

```toml
[[virtualhost]]
domain = "example.com"
root = "/var/www/example.com"
access_log = "/var/log/veloserve/example.com.access.log"
```

## Server Access Log

```toml
[server]
access_log = "/var/log/veloserve/access.log"
error_log = "/var/log/veloserve/error.log"
```

## Environment Variable

Override log level at runtime:

```bash
RUST_LOG=debug veloserve start
RUST_LOG=veloserve=trace veloserve start
```

## Viewing Logs

```bash
# Live server logs
journalctl -u veloserve -f

# Tail error log
tail -f /var/log/veloserve/error.log

# Search for errors
grep -i error /var/log/veloserve/veloserve.log
```
