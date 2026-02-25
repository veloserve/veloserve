# VeloServe Documentation

Welcome to the official documentation for **VeloServe** — a high-performance web server written in Rust with embedded PHP support.

VeloServe can run as a standalone server in CGI or SAPI mode, or as a **drop-in Apache replacement on cPanel servers** with full WHM integration.

---

## Choose Your Path

<div class="grid cards" markdown>

-   :material-lightning-bolt:{ .lg .middle } **Quick Start**

    ---

    Get VeloServe running in under 2 minutes with the one-line installer.

    [:octicons-arrow-right-24: Quick Start](getting-started/quick-start.md)

-   :material-cog:{ .lg .middle } **Standalone CGI**

    ---

    Simple and portable — works everywhere with any `php-cgi` binary.

    [:octicons-arrow-right-24: CGI Mode](standalone/cgi-mode.md)

-   :material-rocket-launch:{ .lg .middle } **Standalone SAPI**

    ---

    Maximum performance with PHP embedded via FFI. 10-100x faster than CGI.

    [:octicons-arrow-right-24: SAPI Mode](standalone/sapi-mode.md)

-   :material-server:{ .lg .middle } **cPanel Integration**

    ---

    Replace Apache on cPanel servers. WHM plugin, AutoSSL, hook auto-sync.

    [:octicons-arrow-right-24: cPanel Overview](cpanel/overview.md)

</div>

## Key Features

| Feature | Description |
|---------|-------------|
| **Rust Core** | Memory-safe, async I/O with Tokio and Hyper |
| **Embedded PHP** | PHP runs inside VeloServe via FFI — no process spawning |
| **Multi-Layer Cache** | Page, object, and static asset caching with auto-invalidation |
| **HTTP/2 & TLS** | Modern protocols with SNI-based certificate resolution |
| **cPanel Integration** | One-click Apache swap, WHM management UI, cPanel hook auto-sync |
| **WordPress & Magento** | Platform-specific optimizations out of the box |

## Quick Install

```bash
curl -sSL https://veloserve.io/install.sh | bash
```

## Links

- [GitHub Repository](https://github.com/veloserve/veloserve)
- [Website](https://veloserve.io)
- [Releases](https://github.com/veloserve/veloserve/releases)
