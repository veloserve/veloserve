# Installation

## Quick Install (Recommended)

```bash
curl -sSL https://veloserve.io/install.sh | bash
```

## Manual Download

Download pre-built binaries from [GitHub Releases](https://github.com/veloserve/veloserve/releases):

| Platform | Architecture | Download |
|----------|-------------|----------|
| Linux | x86_64 (glibc) | `veloserve-vX.X.X-linux-x86_64.tar.gz` |
| Linux | x86_64 (AlmaLinux 9) | `veloserve-vX.X.X-linux-x86_64-almalinux9.tar.gz` |
| Linux | ARM64 | `veloserve-vX.X.X-linux-aarch64.tar.gz` |
| macOS | Intel (x86_64) | `veloserve-vX.X.X-darwin-x86_64.tar.gz` |
| macOS | Apple Silicon (M1/M2/M3/M4) | `veloserve-vX.X.X-darwin-aarch64.tar.gz` |
| Windows | x86_64 | `veloserve-vX.X.X-windows-x86_64.zip` |

### Linux / macOS

```bash
# Download (replace version and arch as needed)
curl -LO https://github.com/veloserve/veloserve/releases/latest/download/veloserve-linux-x86_64.tar.gz

# Extract
tar -xzf veloserve-linux-x86_64.tar.gz

# Install
sudo mv veloserve /usr/local/bin/
sudo chmod +x /usr/local/bin/veloserve

# Verify
veloserve --version
```

### Windows

1. Download the `.zip` file from the releases page
2. Extract to `C:\Program Files\VeloServe\`
3. Add the folder to your PATH, or run directly:

```powershell
.\veloserve.exe --version
```

## Build from Source

### Requirements

- Rust 1.70+ — install via [rustup.rs](https://rustup.rs)
- PHP 8.x (for PHP support)

### CGI Mode (Default)

```bash
git clone https://github.com/veloserve/veloserve.git
cd veloserve
cargo build --release
sudo cp target/release/veloserve /usr/local/bin/
```

### SAPI Mode (Embedded PHP)

=== "Ubuntu / Debian"

    ```bash
    sudo apt install php-dev libphp-embed libxml2-dev libsodium-dev libargon2-dev
    ```

=== "Fedora / RHEL / AlmaLinux"

    ```bash
    sudo dnf install php-devel php-embedded libxml2-devel libsodium-devel
    ```

=== "From Source"

    ```bash
    ./configure --enable-embed --with-openssl --with-curl --with-gd
    make && sudo make install
    ```

Then build VeloServe with the embed feature:

```bash
cargo build --release --features php-embed
sudo cp target/release/veloserve /usr/local/bin/
```

## Docker

```dockerfile
FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y php-cgi && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/veloserve /usr/local/bin/
EXPOSE 8080
CMD ["veloserve", "--config", "/etc/veloserve/veloserve.toml"]
```

## Verify Installation

```bash
# Check version
veloserve --version

# Check help
veloserve --help

# Test configuration
veloserve config test
```

## Uninstall

```bash
# Remove binary
sudo rm /usr/local/bin/veloserve

# Remove config (optional)
sudo rm -rf /etc/veloserve
```

## Next Steps

- **[Quick Start](quick-start.md)** — get serving in 2 minutes
- **[CGI Mode](../standalone/cgi-mode.md)** — simple, portable PHP execution
- **[SAPI Mode](../standalone/sapi-mode.md)** — maximum performance
- **[cPanel Installation](../cpanel/installation.md)** — deploy on cPanel servers
