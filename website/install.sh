#!/bin/bash
# VeloServe Installer
# Usage: curl -sSL https://veloserve.io/install.sh | bash
# Or: curl -sSL https://raw.githubusercontent.com/veloserve/veloserve/main/scripts/install.sh | bash

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# VeloServe version
VERSION="${VELOSERVE_VERSION:-latest}"
INSTALL_DIR="${VELOSERVE_INSTALL_DIR:-/usr/local/bin}"
GITHUB_REPO="veloserve/veloserve"

echo -e "${BLUE}"
echo "  ⚡ VeloServe Installer"
echo "  High-performance web server with embedded PHP"
echo -e "${NC}"

# Detect OS and architecture
detect_platform() {
    local os arch

    # Detect OS
    case "$(uname -s)" in
        Linux*)     os="linux" ;;
        Darwin*)    os="darwin" ;;
        MINGW*|MSYS*|CYGWIN*) os="windows" ;;
        *)          
            echo -e "${RED}Error: Unsupported operating system$(uname -s)${NC}"
            exit 1
            ;;
    esac

    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)   arch="x86_64" ;;
        aarch64|arm64)  arch="aarch64" ;;
        armv7l)         arch="armv7" ;;
        *)
            echo -e "${RED}Error: Unsupported architecture $(uname -m)${NC}"
            exit 1
            ;;
    esac

    echo "${os}-${arch}"
}

# Get the latest release version from GitHub
get_latest_version() {
    curl -sSL "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | \
        grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
}

# Download and install VeloServe
install_veloserve() {
    local platform="$1"
    local version="$2"
    local tmp_dir

    echo -e "${BLUE}→ Detected platform: ${platform}${NC}"
    echo -e "${BLUE}→ Installing version: ${version}${NC}"

    # Create temp directory
    tmp_dir=$(mktemp -d)
    trap "rm -rf ${tmp_dir}" EXIT

    # Construct download URL
    local os arch ext
    os=$(echo "$platform" | cut -d'-' -f1)
    arch=$(echo "$platform" | cut -d'-' -f2)
    
    if [ "$os" = "windows" ]; then
        ext=".exe"
        archive_ext=".zip"
    else
        ext=""
        archive_ext=".tar.gz"
    fi

    local filename="veloserve-${version}-${platform}${archive_ext}"
    local download_url="https://github.com/${GITHUB_REPO}/releases/download/${version}/${filename}"

    echo -e "${BLUE}→ Downloading from: ${download_url}${NC}"

    # Download
    if command -v curl &> /dev/null; then
        curl -sSL -o "${tmp_dir}/${filename}" "${download_url}" || {
            echo -e "${YELLOW}⚠ Pre-built binary not found. Trying to build from source...${NC}"
            install_from_source
            return
        }
    elif command -v wget &> /dev/null; then
        wget -q -O "${tmp_dir}/${filename}" "${download_url}" || {
            echo -e "${YELLOW}⚠ Pre-built binary not found. Trying to build from source...${NC}"
            install_from_source
            return
        }
    else
        echo -e "${RED}Error: curl or wget is required${NC}"
        exit 1
    fi

    # Extract
    echo -e "${BLUE}→ Extracting...${NC}"
    cd "${tmp_dir}"
    if [ "$archive_ext" = ".zip" ]; then
        unzip -q "${filename}"
    else
        tar -xzf "${filename}"
    fi

    # Install binary
    echo -e "${BLUE}→ Installing to ${INSTALL_DIR}...${NC}"
    
    # Check if we need sudo
    if [ -w "${INSTALL_DIR}" ]; then
        mv "veloserve${ext}" "${INSTALL_DIR}/veloserve${ext}"
        chmod +x "${INSTALL_DIR}/veloserve${ext}"
    else
        echo -e "${YELLOW}→ Requesting sudo access to install to ${INSTALL_DIR}${NC}"
        sudo mv "veloserve${ext}" "${INSTALL_DIR}/veloserve${ext}"
        sudo chmod +x "${INSTALL_DIR}/veloserve${ext}"
    fi

    echo -e "${GREEN}✓ VeloServe installed successfully!${NC}"
}

# Fallback: Install from source using cargo
install_from_source() {
    echo -e "${BLUE}→ Installing from source...${NC}"
    
    # Check if cargo is installed
    if ! command -v cargo &> /dev/null; then
        echo -e "${YELLOW}→ Rust not found. Installing Rust...${NC}"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi

    # Clone and build
    local tmp_dir=$(mktemp -d)
    trap "rm -rf ${tmp_dir}" EXIT
    
    echo -e "${BLUE}→ Cloning repository...${NC}"
    git clone --depth 1 "https://github.com/${GITHUB_REPO}.git" "${tmp_dir}/veloserve"
    
    echo -e "${BLUE}→ Building (this may take a few minutes)...${NC}"
    cd "${tmp_dir}/veloserve"
    cargo build --release
    
    # Install
    if [ -w "${INSTALL_DIR}" ]; then
        cp "target/release/veloserve" "${INSTALL_DIR}/veloserve"
        chmod +x "${INSTALL_DIR}/veloserve"
    else
        sudo cp "target/release/veloserve" "${INSTALL_DIR}/veloserve"
        sudo chmod +x "${INSTALL_DIR}/veloserve"
    fi
    
    echo -e "${GREEN}✓ VeloServe built and installed successfully!${NC}"
}

# Create default config if it doesn't exist
create_default_config() {
    local config_dir="/etc/veloserve"
    local config_file="${config_dir}/veloserve.toml"
    
    if [ ! -f "${config_file}" ]; then
        echo -e "${BLUE}→ Creating default configuration...${NC}"
        
        if [ -w "/etc" ]; then
            mkdir -p "${config_dir}"
        else
            sudo mkdir -p "${config_dir}"
        fi
        
        cat > /tmp/veloserve.toml << 'EOF'
# VeloServe Configuration
# Documentation: https://veloserve.io/docs

[server]
listen = "0.0.0.0:8080"
workers = "auto"

[php]
enable = true
binary_path = "/usr/bin/php-cgi"
workers = 4
memory_limit = "256M"

[cache]
enable = true
storage = "memory"
default_ttl = 3600

[[virtualhost]]
domain = "*"
root = "/var/www/html"
index = ["index.php", "index.html"]
EOF

        if [ -w "${config_dir}" ]; then
            mv /tmp/veloserve.toml "${config_file}"
        else
            sudo mv /tmp/veloserve.toml "${config_file}"
        fi
    fi
}

# Main installation
main() {
    local platform version

    # Detect platform
    platform=$(detect_platform)

    # Get version
    if [ "${VERSION}" = "latest" ]; then
        echo -e "${BLUE}→ Fetching latest version...${NC}"
        version=$(get_latest_version)
        if [ -z "${version}" ]; then
            version="v1.0.0"  # Fallback
        fi
    else
        version="${VERSION}"
    fi

    # Install
    install_veloserve "${platform}" "${version}"
    
    # Create config
    create_default_config

    # Verify installation
    echo ""
    if command -v veloserve &> /dev/null; then
        echo -e "${GREEN}✓ Installation complete!${NC}"
        echo ""
        veloserve --version
        echo ""
        echo -e "${BLUE}Quick Start:${NC}"
        echo "  veloserve --config /etc/veloserve/veloserve.toml"
        echo ""
        echo -e "${BLUE}Or create a simple test:${NC}"
        echo "  mkdir -p /tmp/www && echo '<?php phpinfo();' > /tmp/www/index.php"
        echo "  veloserve start --root /tmp/www --listen 0.0.0.0:8080"
        echo ""
        echo -e "${BLUE}Documentation:${NC} https://veloserve.io"
        echo -e "${BLUE}GitHub:${NC} https://github.com/veloserve/veloserve"
    else
        echo -e "${RED}Installation may have failed. Please check the output above.${NC}"
        exit 1
    fi
}

main "$@"

