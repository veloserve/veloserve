# VeloServe Web Server

<p align="center">
  <img src="https://img.shields.io/badge/version-0.1.0--alpha-blue" alt="Version">
  <img src="https://img.shields.io/badge/rust-1.70%2B-orange" alt="Rust">
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License">
  <img src="https://img.shields.io/badge/status-development-yellow" alt="Status">
</p>

<p align="center">
  <a href="https://github.com/codespaces/new?hide_repo_select=true&ref=main&repo=veloserve/veloserve"><img src="https://img.shields.io/badge/Open%20in-GitHub%20Codespaces-blue?logo=github" alt="Open in Codespaces"></a>
  <a href="https://ona.com/#https://github.com/veloserve/veloserve"><img src="https://img.shields.io/badge/Open%20in-Ona-ff6b35?logo=cloud" alt="Open in Ona"></a>
</p>

A high-performance web server designed as a modern alternative to LiteSpeed/Nginx/Apache, featuring integrated PHP processing, intelligent caching, and optimized support for popular CMS and eCommerce platforms like WordPress and Magento.

---

## ⚡ Quick Start (Cloud Development)

**No local setup required!** Start developing instantly in your browser:

### Option 1: GitHub Codespaces (Recommended)

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://github.com/codespaces/new?hide_repo_select=true&ref=main&repo=veloserve/veloserve)

1. Click the button above (or go to **Code** → **Codespaces** → **Create codespace**)
2. Wait ~2 minutes for setup to complete
3. Run the server:
   ```bash
   make run
   ```
4. Click the **Ports** tab → Click the 🌐 globe icon next to port `8080`
5. Your VeloServe instance is now live!

### Option 2: Ona

[![Open in Ona](https://img.shields.io/badge/Open%20in-Ona-ff6b35?style=for-the-badge&logo=cloud)](https://ona.com/#https://github.com/veloserve/veloserve)

1. Click the button above
2. Wait for the workspace to build
3. The server starts automatically!
4. Click the URL in the terminal or go to **Ports** → **Open Browser**

### Test Your Instance

Once the server is running, try these endpoints:

| Endpoint | Description |
|----------|-------------|
| `/` | Static HTML welcome page |
| `/health` | Health check (returns "OK") |
| `/api/v1/status` | Server status JSON |
| `/api/v1/cache/stats` | Cache statistics |
| `/index.php` | PHP test page |
| `/info.php` | PHP configuration info |

**Example curl commands:**
```bash
# Health check
curl https://your-codespace-url.github.dev/health

# API status
curl https://your-codespace-url.github.dev/api/v1/status

# PHP test
curl https://your-codespace-url.github.dev/index.php
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

## ✨ Key Features

### 1. Integrated PHP Engine

- **Embedded PHP interpreter** (libphp or custom PHP SAPI)
- No PHP-FPM required - direct process integration
- Thread-safe or process-pool architecture
- Support for PHP 7.4, 8.0, 8.1, 8.2, 8.3+
- Configurable worker pools per virtual host
- Automatic PHP version detection per site

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
- [x] Integrated PHP support (single version)
- [ ] Basic page caching
- [ ] WordPress detection and caching
- [ ] Configuration system
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

