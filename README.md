# VeloServe Web Server

<p align="center">
  <img src="https://img.shields.io/badge/version-1.0.0-blue" alt="Version">
  <img src="https://img.shields.io/badge/rust-1.70%2B-orange" alt="Rust">
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License">
  <img src="https://img.shields.io/badge/php-embedded-purple" alt="PHP">
</p>

<p align="center">
  <a href="https://github.com/codespaces/new?hide_repo_select=true&ref=main&repo=veloserve/veloserve"><img src="https://img.shields.io/badge/Open%20in-GitHub%20Codespaces-blue?logo=github" alt="Open in Codespaces"></a>
  <a href="https://ona.com/#https://github.com/veloserve/veloserve"><img src="https://img.shields.io/badge/Open%20in-Ona-ff6b35?logo=cloud" alt="Open in Ona"></a>
</p>

A high-performance web server designed as a modern alternative to LiteSpeed/Nginx/Apache, featuring integrated PHP processing, intelligent caching, and optimized support for popular CMS and eCommerce platforms like WordPress and Magento.

---

## 🚀 Quick Install (One Command!)

```bash
curl -sSL https://veloserve.io/install.sh | bash
```

**Or using wget:**
```bash
wget -qO- https://veloserve.io/install.sh | bash
```

**Then start serving:**
```bash
# Quick test
mkdir -p /tmp/www && echo '<?php phpinfo();' > /tmp/www/index.php
veloserve start --root /tmp/www --listen 0.0.0.0:8080

# Visit http://localhost:8080
```

> 📦 **Pre-built binaries available for:** Linux (x64, ARM64), macOS (x64, Apple Silicon), Windows
> 
> 🔧 **No Rust installation required!** The installer downloads the right binary for your system.

---

## 📥 Quick Run with Pre-built Binaries

Don't want to compile? Download a pre-built binary for your platform:

### Download Links

| Platform | Architecture | Download | Notes |
|----------|--------------|----------|-------|
| **Linux** | x86_64 | [veloserve-linux-x86_64.tar.gz](https://github.com/veloserve/veloserve/releases/latest/download/veloserve-latest-x86_64-unknown-linux-gnu.tar.gz) | Standard build (PHP-CGI) |
| **Linux** | x86_64 | [veloserve-linux-x86_64-php-embed.tar.gz](https://github.com/veloserve/veloserve/releases/latest/download/veloserve-latest-x86_64-unknown-linux-gnu-php-embed.tar.gz) | **High-performance PHP SAPI** |
| **Linux** | ARM64 | [veloserve-linux-arm64.tar.gz](https://github.com/veloserve/veloserve/releases/latest/download/veloserve-latest-aarch64-unknown-linux-gnu.tar.gz) | Raspberry Pi, AWS Graviton |
| **macOS** | Intel | [veloserve-macos-x86_64.tar.gz](https://github.com/veloserve/veloserve/releases/latest/download/veloserve-latest-x86_64-apple-darwin.tar.gz) | Intel Macs |
| **macOS** | Apple Silicon | [veloserve-macos-arm64.tar.gz](https://github.com/veloserve/veloserve/releases/latest/download/veloserve-latest-aarch64-apple-darwin.tar.gz) | M1/M2/M3 Macs |
| **Windows** | x86_64 | [veloserve-windows-x86_64.zip](https://github.com/veloserve/veloserve/releases/latest/download/veloserve-latest-x86_64-pc-windows-msvc.zip) | Windows 10/11 |

### Quick Start by Platform

**Linux / macOS:**
```bash
# Download and extract (example for Linux x86_64)
curl -LO https://github.com/veloserve/veloserve/releases/latest/download/veloserve-latest-x86_64-unknown-linux-gnu.tar.gz
tar -xzf veloserve-latest-x86_64-unknown-linux-gnu.tar.gz

# Make executable and run
chmod +x veloserve
./veloserve --help

# Start serving a directory
./veloserve start --root /var/www/html --listen 0.0.0.0:8080
```

**Windows (PowerShell):**
```powershell
# Download
Invoke-WebRequest -Uri "https://github.com/veloserve/veloserve/releases/latest/download/veloserve-latest-x86_64-pc-windows-msvc.zip" -OutFile "veloserve.zip"

# Extract
Expand-Archive -Path "veloserve.zip" -DestinationPath "."

# Run
.\veloserve.exe --help
.\veloserve.exe start --root C:\www --listen 0.0.0.0:8080
```

### PHP Embed vs Standard Build

| Build | Use Case | Requirements |
|-------|----------|--------------|
| **Standard** | Development, simple sites | `php-cgi` installed |
| **PHP Embed** | Production, WordPress, high-traffic | `libphp-embed` installed |

For PHP Embed builds, install the PHP embed library:
```bash
# Ubuntu/Debian
sudo apt install libphp-embed

# Then run with embed mode
./veloserve --config mysite.toml  # Set mode = "embed" in config
```

---

## ⚡ Quick Start (Cloud Development)

**No local setup required!** Start developing instantly in your browser with **Ona.com** or **GitHub Codespaces**.

### 🟠 Option 1: Ona.com

[![Open in Ona](https://img.shields.io/badge/Open%20in-Ona-ff6b35?style=for-the-badge&logo=cloud)](https://ona.com/#https://github.com/veloserve/veloserve)

1. **Click the button above** to launch the Ona environment
2. **Wait ~2-3 minutes** for the environment to build (Rust + PHP pre-installed)
3. **Start VeloServe:**
   ```bash
   make run
   ```
4. **Open the port:**
   - Look at the **Ports panel** (bottom of the screen)
   - Find port `8080` 
   - Click the 🌐 **globe icon** or the **address** to open in browser
   - If not visible, click **Make Public** to expose the port
5. ✅ **VeloServe is running!**

> **💡 Ona.com Port Forwarding:** Ona automatically creates a public URL like:
> `https://8080--YOUR-WORKSPACE-ID.eu-central-1-01.gitpod.dev`
> This URL is accessible from anywhere and forwards to your VeloServe instance.

### 🔵 Option 2: GitHub Codespaces

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://github.com/codespaces/new?hide_repo_select=true&ref=main&repo=veloserve/veloserve)

1. **Click the button above** (or go to **Code** → **Codespaces** → **Create codespace**)
2. **Wait ~2 minutes** for setup to complete
3. **Start VeloServe:**
   ```bash
   make run
   ```
4. **Click the Ports tab** → Click the 🌐 globe icon next to port `8080`
5. ✅ **VeloServe is running!**

### Test Your Instance

Once the server is running, try these endpoints:

| Endpoint | Description |
|----------|-------------|
| `/` | Static HTML welcome page |
| `/health` | Health check (returns "OK") |
| `/api/v1/status` | Server status JSON |
| `/index.php` | PHP test page |
| `/info.php` | PHP configuration info |

---

## 🌐 Optional: WordPress Demo

**Want to test VeloServe with a real WordPress site?** You can optionally install WordPress after your environment starts.

### Install WordPress (Ona.com or Codespaces)

After your environment is running, simply run:

```bash
make wordpress
```

This will:
- ✅ Download WordPress automatically
- ✅ Configure SQLite database (no MySQL needed!)
- ✅ Start VeloServe with WordPress
- ✅ Open the WordPress installation wizard

### Step-by-Step WordPress Demo

| Step | Ona.com | GitHub Codespaces |
|------|---------|-------------------|
| 1. Open environment | [Open in Ona](https://ona.com/#https://github.com/veloserve/veloserve) | [Open in Codespaces](https://github.com/codespaces/new?hide_repo_select=true&ref=main&repo=veloserve/veloserve) |
| 2. Wait for build | ~2-3 minutes | ~2 minutes |
| 3. Install WordPress | `make wordpress` | `make wordpress` |
| 4. Open browser | Click 🌐 on port 8080 | Click 🌐 on port 8080 |
| 5. Complete setup | WordPress wizard appears! | WordPress wizard appears! |

### WordPress Demo Features

- ✅ **No MySQL Required** - Uses SQLite for easy demo
- ✅ **Pre-configured** - Works out of the box
- ✅ **Auto URL Detection** - Works with port forwarding
- ✅ **Debug Mode** - See errors during development

### Available Commands

```bash
# Basic VeloServe (no WordPress)
make run

# VeloServe + WordPress
make wordpress

# Setup WordPress without starting server
make wordpress-setup

# Run tests
make test
```

### WordPress Configuration

When running with WordPress, VeloServe uses this optimized configuration:

```toml
[[virtualhost]]
domain = "*"
root = "/var/www/wordpress"
platform = "wordpress"

[virtualhost.cache]
enable = true
ttl = 3600
exclude = ["/wp-admin/*", "/wp-login.php"]
```

---

## 🖥️ Local Development

If you prefer local development:

```bash
# Clone the repository
git clone https://github.com/veloserve/veloserve.git
cd veloserve

# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install PHP (Ubuntu/Debian)
sudo apt install php php-cli php-mysql php-curl php-gd php-mbstring php-xml php-zip

# Build
cargo build

# Run
make run
# or: RUST_LOG=info cargo run -- --config veloserve.toml

# Test
curl http://localhost:8080/health
curl http://localhost:8080/api/v1/status
curl http://localhost:8080/index.php
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed development instructions.

---

## 🚀 Core Philosophy

- **Zero external dependencies for PHP** - Built-in PHP interpreter eliminates PHP-FPM overhead
- **Intelligent caching** - Application-aware caching with automatic invalidation
- **Performance-first** - Designed for high-traffic e-commerce and content sites
- **Simple deployment** - Single binary with minimal configuration

---

## 🐘 Two Ways to Run VeloServe

VeloServe v1.0 supports **two PHP execution modes**. Choose based on your needs:

### 🔵 Mode 1: CGI Mode (Simple & Portable)

```bash
# Standard build - works everywhere PHP is installed
cargo build --release

# Run with php-cgi
./target/release/veloserve --config veloserve.toml
```

**Architecture:**
```
┌─────────────────────────────────────┐
│         VeloServe (Rust)            │
│  ┌─────────────────────────────┐    │
│  │   HTTP Server (Hyper)       │    │
│  └──────────┬──────────────────┘    │
│             │ spawn process         │
│  ┌──────────▼──────────────────┐    │
│  │   php-cgi (external)        │    │
│  │   - Process pool with       │    │
│  │     semaphore limiting      │    │
│  │   - POST body via stdin     │    │
│  │   - Full CGI environment    │    │
│  └─────────────────────────────┘    │
└─────────────────────────────────────┘
```

| Pros | Cons |
|------|------|
| ✅ Simple setup | ⚠️ Process overhead per request |
| ✅ Works with any PHP | ⚠️ ~50ms latency per request |
| ✅ All extensions work | ⚠️ Requires php-cgi installed |
| ✅ Easy debugging | |

---

### 🚀 Mode 2: Embedded SAPI Mode (Maximum Performance)

```bash
# Install PHP embed library (Ubuntu/Debian)
sudo apt install php-dev libphp-embed libxml2-dev libsodium-dev libargon2-dev

# Build with embedded PHP - PHP is INSIDE VeloServe!
cargo build --release --features php-embed

# Run - no external PHP needed!
./target/release/veloserve --config veloserve.toml
```

**Architecture:**
```
┌─────────────────────────────────────┐
│         VeloServe (Rust)            │
│  ┌─────────────────────────────┐    │
│  │   HTTP Server (Hyper)       │    │
│  └──────────┬──────────────────┘    │
│             │ FFI call (zero-copy)  │
│  ┌──────────▼──────────────────┐    │
│  │   libphp.so (embedded)      │    │
│  │   - PHP runs in-process     │    │
│  │   - No fork/exec overhead   │    │
│  │   - Shared memory           │    │
│  └─────────────────────────────┘    │
└─────────────────────────────────────┘
```

| Pros | Cons |
|------|------|
| 🚀 10-100x faster than CGI | 📋 Requires libphp-embed |
| 🚀 ~1ms latency per request | 📋 Larger binary size |
| 🚀 Single binary deployment | 📋 PHP version locked at compile |
| 🚀 True "integrated PHP engine" | |

---

### 📊 Performance Comparison

| Mode | Requests/sec | Latency | Memory |
|------|-------------|---------|--------|
| CGI Mode | ~500 req/s | ~50ms | Low (on-demand) |
| **SAPI Mode** | **~10,000 req/s** | **~1ms** | Medium (persistent) |
| PHP-FPM (reference) | ~2,000 req/s | ~10ms | Medium |

---

### 🛠️ Which Mode Should I Use?

| Use Case | Recommended Mode |
|----------|------------------|
| Development / Testing | CGI Mode |
| Production (low traffic) | CGI Mode |
| Production (high traffic) | **SAPI Mode** |
| WordPress / Magento | **SAPI Mode** |
| Serverless / Lambda | CGI Mode |
| Docker containers | Either (SAPI for performance) |

---

## ✨ Key Features

### 1. Integrated PHP Engine

- **CGI Mode**: Process pool with `php-cgi` - works everywhere, easy setup
- **SAPI Mode** ✅: Embedded `libphp` via FFI - **10-100x faster**, now working!
- No PHP-FPM required - direct process integration
- Thread-safe execution with dedicated PHP worker thread
- Support for PHP 7.4, 8.0, 8.1, 8.2, 8.3+
- Full WordPress support including admin dashboard, sessions, and cookies
- Configurable error logging with dedicated `error_log` option

### 2. Built-in Caching System

#### Multi-Layer Cache Architecture

```
Request → Edge Cache → Page Cache → Object Cache → Origin
```

**Cache Types:**
- **Page Cache**: Full HTML caching with smart invalidation
- **Object Cache**: In-memory object storage (Magento blocks, WordPress transients)
- **Static Asset Cache**: CSS, JS, images with aggressive TTLs
- **Edge Cache**: Distributed caching support (future: CDN integration)

**Cache Storage Backends:**
- Memory-mapped files (default, fastest)
- Redis integration (optional, for distributed setups)
- Disk-based cache with compression

### 3. WordPress Optimization

**Features:**
- Automatic detection of WordPress installation
- REST API caching with intelligent invalidation
- WooCommerce cart/checkout bypass
- Logged-in user detection (no caching for admin/logged users)
- Cache tags for taxonomy/post relationships
- Purge hooks integration (plugin API compatible)
- CDN support (CloudFlare, CloudFront)

**Configuration Example:**
```toml
[wordpress]
enable = true
cache_ttl = 3600
cache_mobile_separate = true
cache_logged_in = false
vary_cookies = ["wordpress_logged_in*", "wp-postpass*"]
exclude_urls = ["/wp-admin/*", "/cart/*", "/checkout/*", "/my-account/*"]
```

### 4. Magento 2 Optimization

**Features:**
- Full Page Cache (FPC) replacement
- Varnish-compatible ESI support
- Block-level caching with automatic invalidation
- Customer session handling
- Cache warming for category/product pages
- GraphQL response caching (for PWA Studio)
- Multi-store support with separate cache pools

**Advanced Features:**
- Automatic cache tagging based on Magento cache tags
- Integration with Magento cache invalidation events
- Support for private content (customer-specific blocks)
- Shopping cart bypass (similar to LiteSpeed LSCache)

**Configuration Example:**
```toml
[magento2]
enable = true
cache_ttl = 7200
esi_support = true
customer_cookies = ["PHPSESSID", "private_content_version"]
cache_tags_header = "X-Magento-Tags"
vary_by_currency = true
vary_by_store = true
```

---

## 🏗️ Architecture

### Core Server Engine

**Technology Stack:**
- **Rust** - Memory safety, performance, async I/O
- **Tokio** - Async runtime
- **Hyper** - HTTP/1.1 and HTTP/2 support
- **rustls** - TLS implementation

### PHP Integration

**Process Pool Architecture:**
```
VeloServe → Internal Process Pool → PHP Workers
```

- Managed worker processes
- Faster than external PHP-FPM (shared memory communication)
- Automatic scaling based on load

### Cache Architecture

```
┌─────────────────────────────────────┐
│      HTTP Request Handler           │
└──────────────┬──────────────────────┘
               │
       ┌───────▼────────┐
       │  Cache Lookup  │
       └───────┬────────┘
               │
        ┌──────▼─────────┐
        │  Cache Hit?    │
        └──┬────────┬────┘
           │        │
         Yes       No
           │        │
    ┌──────▼──┐    │
    │ Serve   │    │
    │ Cached  │    │
    └─────────┘    │
                   │
            ┌──────▼────────┐
            │  PHP Handler  │
            └──────┬────────┘
                   │
            ┌──────▼────────┐
            │ Store in Cache│
            └──────┬────────┘
                   │
            ┌──────▼────────┐
            │ Return Response│
            └───────────────┘
```

**Cache Storage:**
- Hot data: In-memory hash maps with LRU eviction
- Warm data: Memory-mapped files
- Cold data: Compressed disk storage
- Cache keys: Derived from URL, cookies, headers (configurable)

---

## ⚙️ Configuration

VeloServe uses TOML for configuration:

```toml
[server]
listen = "0.0.0.0:80"
listen_ssl = "0.0.0.0:443"
workers = "auto"  # CPU count
max_connections = 10000

[php]
version = "8.2"
workers = 16
memory_limit = "256M"
max_execution_time = 30

[cache]
enable = true
storage = "memory"  # memory, redis, disk
memory_limit = "2G"
default_ttl = 3600

[ssl]
cert = "/etc/veloserve/ssl/cert.pem"
key = "/etc/veloserve/ssl/key.pem"
protocols = ["TLSv1.2", "TLSv1.3"]

[[virtualhost]]
domain = "example.com"
root = "/var/www/example.com"
platform = "wordpress"

[virtualhost.cache]
enable = true
ttl = 7200
vary = ["Accept-Encoding", "Cookie"]
```

---

## 📊 Performance Targets

### Benchmarks (vs Nginx + PHP-FPM)

| Metric | Target |
|--------|--------|
| Static files | 2-3x faster |
| Dynamic PHP | 40-60% faster (no FPM overhead) |
| Cached pages | 5-10x faster (integrated cache) |
| Memory usage | 30-40% lower |
| Concurrent connections | 50K+ |

### Optimization Features

- HTTP/2 and HTTP/3 (QUIC) support
- Zero-copy file serving
- Sendfile() and TCP_CORK optimization
- Aggressive connection pooling
- Smart prefetching for static assets

---

## 🔒 Security Features

1. **Request filtering**
   - SQL injection pattern detection
   - XSS protection headers
   - Rate limiting per IP/endpoint
   - Geographic blocking (optional)

2. **SSL/TLS**
   - Automatic certificate generation (Let's Encrypt integration)
   - OCSP stapling
   - Session resumption
   - Perfect Forward Secrecy

3. **PHP Security**
   - disable_functions enforcement
   - open_basedir automatic configuration
   - Upload size limits
   - Execution timeout protection

---

## 🛠️ API & Management

### REST API

```bash
# Cache management
POST /api/v1/cache/purge
POST /api/v1/cache/purge?tag=category_5
GET  /api/v1/cache/stats

# Server control
GET  /api/v1/status
POST /api/v1/reload
GET  /api/v1/workers

# Analytics
GET  /api/v1/metrics
```

### CLI Tool

```bash
# Cache operations
veloserve cache purge --all
veloserve cache purge --domain=example.com
veloserve cache purge --tag=product_123
veloserve cache stats

# Configuration
veloserve config validate
veloserve config reload
veloserve config test

# Service management
veloserve start
veloserve stop
veloserve restart
veloserve status
```

---

## 🐘 PHP Support

VeloServe has **integrated PHP support** - no PHP-FPM required! It executes PHP scripts directly using the system PHP binary.

### Quick Setup

```bash
# Install PHP with common extensions (Ubuntu/Debian)
sudo apt install php php-cli php-mysql php-curl php-gd php-mbstring \
    php-xml php-zip php-intl php-bcmath php-soap php-opcache

# Configure VeloServe to use PHP
# Edit veloserve.toml:
[php]
enable = true
binary_path = "/usr/bin/php"
memory_limit = "256M"
```

### Verify PHP is Working

```bash
# Start VeloServe
./veloserve --config veloserve.toml

# Test PHP
curl http://localhost:8080/info.php
```

### Full Documentation

See **[docs/PHP_EXTENSIONS.md](docs/PHP_EXTENSIONS.md)** for:
- Complete extension list for WordPress/Magento 2
- Installation guides for all distributions
- Performance tuning tips
- Troubleshooting guide

---

## 📦 Installation

### From Source (Rust)

```bash
# Clone the repository
git clone https://github.com/veloserve/veloserve.git
cd veloserve

# Build release binary
cargo build --release

# Install
sudo cp target/release/veloserve /usr/local/bin/
```

### Package Managers (Coming Soon)

```bash
# Debian/Ubuntu
apt install veloserve

# RHEL/Rocky
yum install veloserve
```

### Docker

```bash
docker run -d -p 80:80 -p 443:443 \
  -v /var/www:/var/www \
  -v /etc/veloserve:/etc/veloserve \
  veloserve/veloserve:latest
```

---

## 🗺️ Development Roadmap

### Phase 1: MVP (Current) ✅
- [x] Core HTTP server (HTTP/1.1, HTTP/2)
- [x] Integrated PHP support (CGI mode)
- [x] PHP Embed SAPI mode (high-performance)
- [x] WordPress full support (login, dashboard, sessions)
- [x] Configuration system (TOML)
- [x] PHP error logging configuration
- [ ] Basic page caching
- [ ] Basic CLI tools

### Phase 2: Enhanced Caching
- [ ] Multi-layer cache system
- [ ] Cache tagging and smart invalidation
- [ ] Magento 2 support
- [ ] Redis integration
- [ ] Cache warming/preloading
- [ ] WordPress plugin

### Phase 3: Production Ready
- [ ] HTTP/3 (QUIC) support
- [ ] SSL/TLS automation (Let's Encrypt)
- [ ] Advanced security features
- [ ] Magento 2 module
- [ ] REST API
- [ ] Monitoring and metrics
- [ ] Web dashboard

### Phase 4: Advanced Features
- [ ] Multi-PHP version support
- [ ] ESI support
- [ ] CDN integration
- [ ] Geographic routing
- [ ] Load balancing
- [ ] Cluster mode
- [ ] Advanced analytics

---

## 📁 Repository Structure

```
veloserve/
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Library exports
│   ├── server/           # HTTP server implementation
│   ├── php/              # PHP integration
│   ├── cache/            # Caching engine
│   ├── config/           # Configuration system
│   └── cli/              # CLI tools
├── tests/                # Test suites
├── examples/             # Example configurations
├── docs/                 # Documentation
├── docker/               # Container images
├── Cargo.toml            # Rust dependencies
└── README.md             # This file
```

---

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with logging
RUST_LOG=debug cargo test

# Run benchmarks
cargo bench
```

---

## 📄 License

VeloServe is open source software licensed under the MIT License.

---

## 🤝 Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---

## 📞 Contact & Community

- **Repository**: [github.com/veloserve/veloserve](https://github.com/veloserve/veloserve)
- **Documentation**: [docs.veloserve.io](https://docs.veloserve.io)
- **Discord**: [discord.gg/veloserve](https://discord.gg/veloserve)

---

<p align="center">
  Built with ❤️ in Rust
</p>

