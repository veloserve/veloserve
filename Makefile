# VeloServe Makefile
# Common development tasks

.PHONY: all build release run test clean fmt lint doc help install

# Default target
all: build

# ============================================
# Building
# ============================================

## Build development binary
build:
	cargo build

## Build optimized release binary
release:
	cargo build --release

## Quick syntax check without building
check:
	cargo check

# ============================================
# Running
# ============================================

## Run with development config
run:
	RUST_LOG=info cargo run -- --config veloserve.toml

## Run with debug logging
run-debug:
	RUST_LOG=debug cargo run -- --config veloserve.toml

## Run with trace logging (very verbose)
run-trace:
	RUST_LOG=trace cargo run -- --config veloserve.toml

## Run release build
run-release: release
	RUST_LOG=info ./target/release/veloserve --config veloserve.toml

# ============================================
# Testing
# ============================================

## Run all tests
test:
	cargo test

## Run tests with output visible
test-verbose:
	cargo test -- --nocapture

## Run tests for specific module
test-server:
	cargo test server::

test-cache:
	cargo test cache::

test-config:
	cargo test config::

test-php:
	cargo test php::

## Quick HTTP endpoint tests (requires server running)
test-http:
	@echo "Testing health endpoint..."
	@curl -s http://localhost:8080/health && echo " ✓"
	@echo "Testing API status..."
	@curl -s http://localhost:8080/api/v1/status | head -1 && echo " ✓"
	@echo "Testing static files..."
	@curl -sI http://localhost:8080/ | head -1 && echo " ✓"
	@echo "All HTTP tests passed!"

# ============================================
# Code Quality
# ============================================

## Format code
fmt:
	cargo fmt

## Check formatting
fmt-check:
	cargo fmt -- --check

## Run linter
lint:
	cargo clippy

## Run strict linter (warnings as errors)
lint-strict:
	cargo clippy -- -D warnings

## Fix linter warnings automatically
lint-fix:
	cargo clippy --fix --allow-dirty

# ============================================
# Documentation
# ============================================

## Generate documentation
doc:
	cargo doc

## Generate and open documentation
doc-open:
	cargo doc --open

# ============================================
# Cleaning
# ============================================

## Remove build artifacts
clean:
	cargo clean

## Remove only debug build
clean-debug:
	rm -rf target/debug

# ============================================
# Installation
# ============================================

## Install to /usr/local/bin
install: release
	sudo cp target/release/veloserve /usr/local/bin/
	@echo "Installed veloserve to /usr/local/bin/"

## Install to ~/.cargo/bin
install-user: release
	cp target/release/veloserve ~/.cargo/bin/
	@echo "Installed veloserve to ~/.cargo/bin/"

## Uninstall from /usr/local/bin
uninstall:
	sudo rm -f /usr/local/bin/veloserve
	@echo "Removed veloserve from /usr/local/bin/"

# ============================================
# Development Helpers
# ============================================

## Watch for changes and rebuild (requires cargo-watch)
watch:
	cargo watch -x build

## Watch and run tests on changes
watch-test:
	cargo watch -x test

## Start server in background for testing
start-bg:
	@pkill -f "target/debug/veloserve" 2>/dev/null || true
	@sleep 1
	RUST_LOG=info cargo run -- --config veloserve.toml &
	@sleep 2
	@echo "Server started in background"

## Stop background server
stop-bg:
	@pkill -f "target/debug/veloserve" 2>/dev/null || true
	@echo "Server stopped"

## Restart background server
restart-bg: stop-bg start-bg

# ============================================
# Benchmarking
# ============================================

## Run cargo benchmarks
bench:
	cargo bench

## Quick HTTP benchmark (requires wrk)
bench-http:
	@echo "Running benchmark against http://localhost:8080/"
	wrk -t4 -c100 -d10s http://localhost:8080/ || echo "Install wrk: apt install wrk"

# ============================================
# Release
# ============================================

## Create release tarball
dist: release
	mkdir -p dist
	tar -czvf dist/veloserve-$(shell cargo pkgid | cut -d'#' -f2)-linux-amd64.tar.gz \
		-C target/release veloserve
	@echo "Created dist/veloserve-*.tar.gz"

# ============================================
# CI/CD Helpers
# ============================================

## Run all CI checks
ci: fmt-check lint-strict test
	@echo "All CI checks passed!"

# ============================================
# Help
# ============================================

## Show this help
help:
	@echo "VeloServe Development Commands"
	@echo "=============================="
	@echo ""
	@echo "Building:"
	@echo "  make build        - Build development binary"
	@echo "  make release      - Build optimized release binary"
	@echo "  make check        - Quick syntax check"
	@echo ""
	@echo "Running:"
	@echo "  make run          - Run with development config"
	@echo "  make run-debug    - Run with debug logging"
	@echo "  make run-release  - Run optimized release build"
	@echo ""
	@echo "Testing:"
	@echo "  make test         - Run all tests"
	@echo "  make test-verbose - Run tests with output"
	@echo "  make test-http    - Quick HTTP endpoint tests"
	@echo ""
	@echo "Code Quality:"
	@echo "  make fmt          - Format code"
	@echo "  make lint         - Run linter"
	@echo "  make lint-strict  - Strict linting (CI mode)"
	@echo ""
	@echo "Documentation:"
	@echo "  make doc          - Generate documentation"
	@echo "  make doc-open     - Generate and open docs"
	@echo ""
	@echo "Other:"
	@echo "  make clean        - Remove build artifacts"
	@echo "  make install      - Install to /usr/local/bin"
	@echo "  make ci           - Run all CI checks"
	@echo "  make help         - Show this help"

