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

# Format: "category event"
HOOKS=(
    "Whostmgr Accounts::Create"
    "Whostmgr Accounts::Remove"
    "Cpanel AddonDomain::addaddondomain"
    "Cpanel AddonDomain::deladdondomain"
    "Cpanel SubDomain::addsubdomain"
    "Cpanel SubDomain::delsubdomain"
    "Cpanel Park::park"
    "Cpanel Park::unpark"
    "Whostmgr SSLStorage::add_ssl"
    "Whostmgr SSLStorage::delete_ssl"
)

for h in "${HOOKS[@]}"; do
    cat=$( echo "$h" | awk '{print $1}')
    evt=$(echo "$h" | awk '{print $2}')
    $MANAGE_HOOKS add script "$HOOK_SCRIPT" --manual --category "$cat" --event "$evt" --stage post 2>/dev/null || true
    echo "Registered: $cat::$evt"
done

echo "All hooks registered. VeloServe will auto-update config on cPanel changes."
