#!/bin/bash
# Register VeloServe hooks with cPanel
set -e

HOOK_SCRIPT="/usr/local/veloserve/hooks/veloserve-hook.sh"
MANAGE_HOOKS="/usr/local/cpanel/bin/manage_hooks"

if [ "$EUID" -ne 0 ]; then echo "Run as root"; exit 1; fi
if [ ! -x "$MANAGE_HOOKS" ]; then echo "cPanel not found"; exit 1; fi

mkdir -p /usr/local/veloserve/hooks /var/log/veloserve
cp "$(dirname "$0")/veloserve-hook.sh" "$HOOK_SCRIPT"
chmod +x "$HOOK_SCRIPT"

EVENTS=(
    "Accounts::Create"
    "Accounts::Remove"
    "AddonDomain::addaddondomain"
    "AddonDomain::deladdondomain"
    "SubDomain::addsubdomain"
    "SubDomain::delsubdomain"
    "Park::park"
    "Park::unpark"
    "SSLStorage::add_ssl"
    "SSLStorage::delete_ssl"
)

for evt in "${EVENTS[@]}"; do
    $MANAGE_HOOKS add script "$HOOK_SCRIPT" --event "$evt" --stage post 2>/dev/null || true
    echo "Registered: $evt"
done

echo "All hooks registered. VeloServe will auto-update config on cPanel changes."
