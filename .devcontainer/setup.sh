#!/bin/bash
# VeloServe Development Environment Setup
# This script runs automatically in GitHub Codespaces and DevContainers

set -e

echo "🚀 Setting up VeloServe development environment..."

# Install PHP extensions for WordPress/Magento testing
echo "📦 Installing PHP extensions..."
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
    php-mysql \
    php-curl \
    php-gd \
    php-mbstring \
    php-xml \
    php-zip \
    php-intl \
    php-bcmath \
    php-soap \
    php-opcache \
    php-iconv \
    2>/dev/null || true

# Build VeloServe
echo "🔨 Building VeloServe..."
cargo build

# Create test directory structure
echo "📁 Setting up test files..."
mkdir -p /var/www/html
cp -r examples/www/* /var/www/html/ 2>/dev/null || true

# Update config for container environment
echo "⚙️ Configuring VeloServe..."
cat > veloserve.toml << 'EOF'
# VeloServe Development Configuration (Codespaces/Gitpod)

[server]
listen = "0.0.0.0:8080"
workers = "auto"
max_connections = 1000

[php]
enable = true
version = "8.3"
binary_path = "/usr/bin/php"
workers = 4
memory_limit = "256M"
max_execution_time = 30
ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=128"
]

[cache]
enable = true
storage = "memory"
memory_limit = "256M"
default_ttl = 60

[[virtualhost]]
domain = "*"
root = "/var/www/html"
index = ["index.php", "index.html"]

[virtualhost.cache]
enable = false
EOF

echo ""
echo "✅ VeloServe development environment is ready!"
echo ""
echo "📋 Quick Start Commands:"
echo "   make run          - Start the server"
echo "   make test         - Run tests"
echo "   make test-http    - Test HTTP endpoints"
echo ""
echo "🌐 Access URLs (after starting server):"
echo "   Health:    http://localhost:8080/health"
echo "   Status:    http://localhost:8080/api/v1/status"
echo "   PHP Test:  http://localhost:8080/index.php"
echo "   PHP Info:  http://localhost:8080/info.php"
echo ""

