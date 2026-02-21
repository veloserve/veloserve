#!/bin/bash
# VeloServe WHM Plugin Uninstallation Script

set -e

PLUGIN_DIR="/usr/local/cpanel/whostmgr/docroot/cgi/veloserve"
REGISTRY_DIR="/var/cpanel/apps"

echo "VeloServe WHM Plugin Uninstaller"
echo "================================="
echo ""

if [ "$EUID" -ne 0 ]; then 
    echo "Error: Please run as root"
    exit 1
fi

# Stop VeloServe if running
if [ -f "/run/veloserve.pid" ]; then
    echo "Stopping VeloServe..."
    /usr/local/bin/veloserve stop 2>/dev/null || true
fi

# Remove systemd service
if [ -f "/etc/systemd/system/veloserve.service" ]; then
    systemctl stop veloserve 2>/dev/null || true
    systemctl disable veloserve 2>/dev/null || true
    rm -f /etc/systemd/system/veloserve.service
    systemctl daemon-reload
    echo "Systemd service removed"
fi

# Remove init.d script
if [ -f "/etc/init.d/veloserve" ]; then
    chkconfig --del veloserve 2>/dev/null || true
    rm -f /etc/init.d/veloserve
    echo "Init.d script removed"
fi

# Unregister from WHM
if [ -f "${REGISTRY_DIR}/veloserve.conf" ]; then
    /usr/local/cpanel/bin/unregister_appconfig ${REGISTRY_DIR}/veloserve.conf 2>/dev/null || true
    rm -f ${REGISTRY_DIR}/veloserve.conf
    echo "WHM registration removed"
fi

# Remove plugin files
if [ -d "${PLUGIN_DIR}" ]; then
    rm -rf ${PLUGIN_DIR}
    echo "Plugin files removed"
fi

echo ""
echo "Uninstallation complete."
echo ""
echo "Note: Configuration files in /etc/veloserve/ were preserved."
echo "To remove them completely, run: rm -rf /etc/veloserve /var/log/veloserve"
