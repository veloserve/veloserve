# Contributing to VeloServe

Thank you for your interest in contributing to VeloServe! This document provides guidelines and instructions for development.

## Table of Contents

- [Development Setup](#development-setup)
- [Building](#building)
- [Running](#running)
- [Testing](#testing)
- [Project Structure](#project-structure)
- [Code Style](#code-style)
- [Pull Request Process](#pull-request-process)

---

## Development Setup

### Prerequisites

- **Rust** 1.70 or later
- **Cargo** (comes with Rust)
- **Git**

### Optional: PHP Support

For PHP integration testing, install PHP with common extensions:

```bash
# Ubuntu/Debian
sudo apt install php php-cli php-mysql php-curl php-gd php-mbstring \
    php-xml php-zip php-intl php-bcmath php-soap php-opcache

# RHEL/Rocky/AlmaLinux
sudo dnf install php php-cli php-mysqlnd php-curl php-gd php-mbstring \
    php-xml php-zip php-intl php-bcmath php-soap php-opcache

# Verify installation
php -v
php -m  # List installed extensions
```

See **[docs/PHP_EXTENSIONS.md](docs/PHP_EXTENSIONS.md)** for detailed PHP setup instructions.

### Installing Rust

```bash
# Install Rust using rustup (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the prompts, then reload your shell
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### Clone the Repository

```bash
git clone https://github.com/veloserve/veloserve.git
cd veloserve
```

---

## Building

### Development Build

```bash
# Quick build for development (faster, with debug symbols)
cargo build

# The binary will be at: target/debug/veloserve
```

### Release Build

```bash
# Optimized build for production
cargo build --release

# The binary will be at: target/release/veloserve
```

### Build with All Features

```bash
cargo build --all-features
```

### Check for Errors Without Building

```bash
# Fast syntax and type checking
cargo check
```

---

## Running

### Run with Default Configuration

```bash
# Development mode with logging
RUST_LOG=info cargo run

# Or run the built binary directly
./target/debug/veloserve
```

### Run with Custom Configuration

```bash
# Using the local dev config
cargo run -- --config veloserve.toml

# Using the example production config
cargo run -- --config examples/veloserve.toml
```

### Run with Verbose Logging

```bash
# Debug level logging
RUST_LOG=debug cargo run

# Trace level (very verbose)
RUST_LOG=trace cargo run

# Module-specific logging
RUST_LOG=veloserve::server=debug,veloserve::php=trace cargo run
```

### Command Line Options

```bash
# Show help
cargo run -- --help

# Show version
cargo run -- --version

# Start with specific config
cargo run -- --config /path/to/config.toml

# Start with verbose output
cargo run -- --verbose
```

---

## Testing

### Run All Tests

```bash
cargo test
```

### Run Tests with Output

```bash
# Show println! output
cargo test -- --nocapture

# Show test names as they run
cargo test -- --test-threads=1
```

### Run Specific Tests

```bash
# Run tests in a specific module
cargo test config::

# Run a specific test by name
cargo test test_parse_config

# Run tests matching a pattern
cargo test cache
```

### Run Integration Tests

```bash
# Run only integration tests
cargo test --test '*'
```

### Test Coverage (requires cargo-tarpaulin)

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html
```

---

## Manual Testing

### Test the HTTP Server

```bash
# Start the server
cargo run -- --config veloserve.toml &

# Test health endpoint
curl http://localhost:8080/health

# Test static file serving
curl http://localhost:8080/

# Test API endpoints
curl http://localhost:8080/api/v1/status
curl http://localhost:8080/api/v1/cache/stats

# Test with headers
curl -I http://localhost:8080/
curl -H "Host: example.com" http://localhost:8080/

# Stop the server
pkill veloserve
```

### Test PHP Integration

First, ensure PHP is installed:

```bash
# Check PHP installation
php --version

# Update config to point to PHP
# Edit veloserve.toml:
# [php]
# binary_path = "/usr/bin/php"
```

Then test:

```bash
# Start server
cargo run -- --config veloserve.toml &

# Test PHP execution
curl http://localhost:8080/index.php

# Test PATH_INFO (clean URLs)
curl http://localhost:8080/index.php/test/path

# Check PHP info
curl http://localhost:8080/info.php
```

### Load Testing

```bash
# Using wrk (install: apt install wrk)
wrk -t12 -c400 -d30s http://localhost:8080/

# Using Apache Bench
ab -n 10000 -c 100 http://localhost:8080/

# Using hey (install: go install github.com/rakyll/hey@latest)
hey -n 10000 -c 100 http://localhost:8080/
```

---

## Project Structure

```
veloserve/
├── Cargo.toml              # Rust dependencies and project config
├── Cargo.lock              # Locked dependency versions
├── README.md               # Project overview
├── CONTRIBUTING.md         # This file
├── Makefile                # Common development tasks
├── veloserve.toml          # Local development config
├── .gitignore              # Git ignore rules
│
├── src/
│   ├── main.rs             # CLI entry point
│   ├── lib.rs              # Library exports
│   │
│   ├── server/
│   │   ├── mod.rs          # HTTP server (Tokio + Hyper)
│   │   ├── handler.rs      # Request handling (Nginx-style)
│   │   ├── router.rs       # URL routing
│   │   └── static_files.rs # Static file serving
│   │
│   ├── php/
│   │   └── mod.rs          # PHP process pool integration
│   │
│   ├── cache/
│   │   └── mod.rs          # Multi-layer caching system
│   │
│   ├── config/
│   │   └── mod.rs          # TOML configuration parser
│   │
│   └── cli/
│       └── mod.rs          # CLI commands
│
├── examples/
│   ├── veloserve.toml      # Example production config
│   └── www/                # Test document root
│       ├── index.html      # Test HTML page
│       ├── index.php       # Test PHP page
│       └── info.php        # PHP info page
│
└── tests/                  # Integration tests
```

---

## Code Style

### Formatting

```bash
# Format all code
cargo fmt

# Check formatting without changes
cargo fmt -- --check
```

### Linting

```bash
# Run clippy linter
cargo clippy

# Fix warnings automatically
cargo clippy --fix

# Strict mode (treat warnings as errors)
cargo clippy -- -D warnings
```

### Documentation

```bash
# Generate documentation
cargo doc

# Open in browser
cargo doc --open

# Include private items
cargo doc --document-private-items
```

### Style Guidelines

1. **Naming**
   - Use `snake_case` for functions and variables
   - Use `PascalCase` for types and traits
   - Use `SCREAMING_SNAKE_CASE` for constants

2. **Comments**
   - Use `///` for public API documentation
   - Use `//` for implementation comments
   - Document all public functions and types

3. **Error Handling**
   - Use `Result<T, E>` for fallible operations
   - Use `anyhow::Result` for application errors
   - Use `thiserror` for library errors

4. **Async Code**
   - Prefer `async/await` over manual futures
   - Use `tokio::spawn` for background tasks
   - Be mindful of blocking operations

---

## Pull Request Process

### Before Submitting

1. **Create a branch**
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make your changes**
   - Write clean, documented code
   - Add tests for new functionality
   - Update documentation if needed

3. **Run checks**
   ```bash
   # Format code
   cargo fmt
   
   # Run linter
   cargo clippy
   
   # Run tests
   cargo test
   
   # Build release to ensure no errors
   cargo build --release
   ```

4. **Commit with clear messages**
   ```bash
   git commit -m "feat: add new caching strategy"
   git commit -m "fix: resolve memory leak in PHP pool"
   git commit -m "docs: update configuration examples"
   ```

### Commit Message Format

Use conventional commits:

- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `test:` Test additions/changes
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `chore:` Maintenance tasks

### Submitting

1. Push your branch
   ```bash
   git push origin feature/my-feature
   ```

2. Create a Pull Request on GitHub

3. Fill out the PR template with:
   - Description of changes
   - Related issues
   - Testing performed
   - Screenshots (if UI changes)

4. Wait for review and address feedback

---

## Getting Help

- **Issues**: Open a GitHub issue for bugs or feature requests
- **Discussions**: Use GitHub Discussions for questions
- **Discord**: Join our Discord server (link in README)

---

## License

By contributing to VeloServe, you agree that your contributions will be licensed under the MIT License.

