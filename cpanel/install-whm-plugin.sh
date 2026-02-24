#!/bin/bash
# VeloServe WHM Plugin Installation Script
# 
# This script installs the VeloServe WHM plugin on cPanel servers

set -e

VELOSERVE_VERSION="1.0.0"
PLUGIN_DIR="/usr/local/cpanel/whostmgr/docroot/cgi/veloserve"
REGISTRY_DIR="/var/cpanel/apps"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}VeloServe WHM Plugin Installer${NC}"
echo "================================"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Error: Please run as root${NC}"
    exit 1
fi

# Check if cPanel is installed
if [ ! -d "/usr/local/cpanel" ]; then
    echo -e "${RED}Error: cPanel not detected${NC}"
    exit 1
fi

echo "Installing VeloServe WHM Plugin v${VELOSERVE_VERSION}..."

# Create plugin directory
mkdir -p ${PLUGIN_DIR}

# Install CGI script
cp whm/veloserve.cgi ${PLUGIN_DIR}/
chmod 755 ${PLUGIN_DIR}/veloserve.cgi

# Install static assets (CSS, JS, images)
mkdir -p ${PLUGIN_DIR}/assets
cp -r whm/assets/* ${PLUGIN_DIR}/assets/ 2>/dev/null || true

# Create cPanel App Registry entry
cat > ${REGISTRY_DIR}/veloserve.conf << 'EOF'
name=veloserve
service=whostmgr
user=root
url=/cgi/veloserve/veloserve.cgi
EOF

# Register with WHM
/usr/local/cpanel/bin/register_appconfig ${REGISTRY_DIR}/veloserve.conf

# Create required directories
mkdir -p /etc/veloserve
mkdir -p /etc/veloserve/vhosts
mkdir -p /var/log/veloserve
mkdir -p /run/veloserve

# Set permissions
chmod 755 /etc/veloserve
chmod 755 /var/log/veloserve
chmod 755 /run/veloserve

# Install systemd service (if systemd is present)
if [ -d "/etc/systemd/system" ]; then
    echo "Installing systemd service..."
    
    cat > /etc/systemd/system/veloserve.service << 'EOF'
[Unit]
Description=VeloServe Web Server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/veloserve --config /etc/veloserve/veloserve.toml start --foreground
ExecStop=/bin/kill -TERM $MAINPID
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    echo -e "${GREEN}Systemd service installed${NC}"
fi

# Install init.d script (fallback)
if [ -d "/etc/init.d" ]; then
    echo "Installing init.d script..."
    
    cat > /etc/init.d/veloserve << 'EOF'
#!/bin/bash
# chkconfig: 2345 99 01
# description: VeloServe Web Server

VELOSERVE_BIN=/usr/local/bin/veloserve
VELOSERVE_CONFIG=/etc/veloserve/veloserve.toml
PIDFILE=/run/veloserve.pid

case "$1" in
    start)
        echo "Starting VeloServe..."
        $VELOSERVE_BIN --config $VELOSERVE_CONFIG start --foreground &
        ;;
    stop)
        echo "Stopping VeloServe..."
        $VELOSERVE_BIN stop
        ;;
    restart)
        echo "Restarting VeloServe..."
        $VELOSERVE_BIN restart
        ;;
    status)
        $VELOSERVE_BIN status
        ;;
    reload)
        echo "Reloading VeloServe configuration..."
        $VELOSERVE_BIN reload
        ;;
    *)
        echo "Usage: $0 {start|stop|restart|status|reload}"
        exit 1
        ;;
esac
EOF

    chmod +x /etc/init.d/veloserve
    chkconfig --add veloserve 2>/dev/null || true
    echo -e "${GREEN}Init.d script installed${NC}"
fi

# Create default configuration
if [ ! -f "/etc/veloserve/veloserve.toml" ]; then
    echo "Creating default configuration..."
    
    # Detect EA-PHP version (newest available)
    EA_PHP_CGI=""
    for ver in 84 83 82 81 80; do
        if [ -x "/opt/cpanel/ea-php${ver}/root/usr/bin/php-cgi" ]; then
            EA_PHP_CGI="/opt/cpanel/ea-php${ver}/root/usr/bin/php-cgi"
            echo -e "${GREEN}Detected EA-PHP: ${EA_PHP_CGI}${NC}"
            break
        fi
    done

    cat > /etc/veloserve/veloserve.toml << CONFEOF
[server]
listen = "0.0.0.0:8080"
# listen_ssl = "0.0.0.0:443"
workers = "auto"
max_connections = 10000

[php]
enable = true
mode = "cgi"
version = "8.3"
${EA_PHP_CGI:+binary_path = "${EA_PHP_CGI}"}
workers = 16
memory_limit = "512M"
max_execution_time = 60
error_log = "/var/log/veloserve/php-error.log"

ini_settings = [
    "opcache.enable=1",
    "opcache.memory_consumption=256",
    "opcache.max_accelerated_files=20000",
    "opcache.revalidate_freq=2",
]

[cache]
enable = true
storage = "memory"
memory_limit = "1G"
default_ttl = 3600
disk_path = "/var/cache/veloserve"
CONFEOF

fi

# Install cPanel hooks (auto-sync config on account/domain/SSL changes)
if [ -f "hooks/install-hooks.sh" ]; then
    echo "Installing cPanel hooks..."
    bash hooks/install-hooks.sh 2>/dev/null && echo -e "${GREEN}cPanel hooks installed${NC}" || echo -e "${YELLOW}Hook registration skipped (non-fatal)${NC}"
fi

# Register VeloServe in WHM Service Manager (so WHM can restart it)
if [ -d "/var/cpanel/service_autorestart" ]; then
    echo "veloserve" > /var/cpanel/service_autorestart/veloserve
    echo -e "${GREEN}Registered in WHM Service Manager${NC}"
fi

echo ""
echo -e "${GREEN}Installation complete!${NC}"
echo ""
echo "Next steps:"
echo "1. Import Apache vhosts and swap:  ./import-apache-and-swap.sh --swap"
echo "2. Access WHM: Plugins > VeloServe"
echo ""
echo "Commands:"
echo "  systemctl start veloserve"
echo "  systemctl enable veloserve  # Auto-start on boot"
echo ""
echo "cPanel hooks are active: accounts, domains, and SSL changes"
echo "will automatically update VeloServe config."
