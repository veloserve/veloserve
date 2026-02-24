#!/bin/bash
# VeloServe cPanel Standardized Hook Handler
#
# cPanel calls hooks with:
#   --describe  → return JSON descriptor
#   (no args)   → event data on stdin as JSON with context.event

VELOSERVE_CONFIG="/etc/veloserve/veloserve.toml"
LOGFILE="/var/log/veloserve/hooks.log"

log() { echo "$(date '+%Y-%m-%d %H:%M:%S') [hook] $*" >> "$LOGFILE" 2>/dev/null; }

# Handle --describe (cPanel discovery)
if [ "$1" = "--describe" ]; then
    cat << 'DESC'
[
    { "category": "Whostmgr", "event": "Accounts::Create", "stage": "post", "hook": "/usr/local/veloserve/hooks/veloserve-hook.sh", "exectype": "script" },
    { "category": "Whostmgr", "event": "Accounts::Remove", "stage": "post", "hook": "/usr/local/veloserve/hooks/veloserve-hook.sh", "exectype": "script" },
    { "category": "Cpanel",   "event": "AddonDomain::addaddondomain", "stage": "post", "hook": "/usr/local/veloserve/hooks/veloserve-hook.sh", "exectype": "script" },
    { "category": "Cpanel",   "event": "AddonDomain::deladdondomain", "stage": "post", "hook": "/usr/local/veloserve/hooks/veloserve-hook.sh", "exectype": "script" },
    { "category": "Cpanel",   "event": "SubDomain::addsubdomain",    "stage": "post", "hook": "/usr/local/veloserve/hooks/veloserve-hook.sh", "exectype": "script" },
    { "category": "Cpanel",   "event": "SubDomain::delsubdomain",    "stage": "post", "hook": "/usr/local/veloserve/hooks/veloserve-hook.sh", "exectype": "script" },
    { "category": "Cpanel",   "event": "Park::park",   "stage": "post", "hook": "/usr/local/veloserve/hooks/veloserve-hook.sh", "exectype": "script" },
    { "category": "Cpanel",   "event": "Park::unpark",  "stage": "post", "hook": "/usr/local/veloserve/hooks/veloserve-hook.sh", "exectype": "script" },
    { "category": "Whostmgr", "event": "SSLStorage::add_ssl",    "stage": "post", "hook": "/usr/local/veloserve/hooks/veloserve-hook.sh", "exectype": "script" },
    { "category": "Whostmgr", "event": "SSLStorage::delete_ssl", "stage": "post", "hook": "/usr/local/veloserve/hooks/veloserve-hook.sh", "exectype": "script" }
]
DESC
    exit 0
fi

# Read event JSON from stdin
EVENT_DATA=$(cat)
EVENT=$(echo "$EVENT_DATA" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('context',{}).get('event',''))" 2>/dev/null)
log "Event: $EVENT"

reload_veloserve() {
    if systemctl is-active --quiet veloserve 2>/dev/null; then
        systemctl reload veloserve 2>/dev/null || systemctl restart veloserve 2>/dev/null
        log "Reloaded"
    fi
}

add_vhost() {
    local domain="$1" docroot="$2"
    [ -z "$domain" ] || [ -z "$docroot" ] && return 1
    grep -q "domain = \"${domain}\"" "$VELOSERVE_CONFIG" 2>/dev/null && return 0
    {
        echo ""; echo "[[virtualhost]]"
        echo "domain = \"${domain}\""
        echo "root = \"${docroot}\""
        echo "platform = \"generic\""
    } >> "$VELOSERVE_CONFIG"
    log "Added vhost: $domain -> $docroot"
}

remove_vhost() {
    local domain="$1"
    [ -z "$domain" ] && return 1
    python3 - "$VELOSERVE_CONFIG" "$domain" << 'PYEOF'
import sys, re
cfg_file, target = sys.argv[1], sys.argv[2]
with open(cfg_file) as f: content = f.read()
# Split into blocks, remove the one matching the target domain
blocks = re.split(r'(?=\[\[virtualhost\]\])', content)
out = []
for b in blocks:
    if 'domain = "' + target + '"' in b:
        continue
    out.append(b)
with open(cfg_file, 'w') as f: f.write(''.join(out))
PYEOF
    log "Removed vhost: $domain"
}

get() { echo "$EVENT_DATA" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d$1)" 2>/dev/null; }

case "$EVENT" in
    Accounts::Create)
        D=$(get ".get('data',{}).get('domain','')")
        H=$(get ".get('data',{}).get('homedir','')")
        [ -z "$H" ] && U=$(get ".get('data',{}).get('user','')") && H="/home/$U"
        add_vhost "$D" "${H}/public_html"; reload_veloserve ;;
    Accounts::Remove)
        U=$(get ".get('data',{}).get('user','')")
        if [ -n "$U" ]; then
            # Remove all vhosts whose root is under /home/$U/
            python3 -c "
import re
with open('$VELOSERVE_CONFIG') as f: c = f.read()
# Remove any [[virtualhost]] block referencing /home/$U/
c = re.sub(r'\n*\[\[virtualhost\]\][^\[]*?/home/$U/[^\[]*', '', c)
with open('$VELOSERVE_CONFIG', 'w') as f: f.write(c)
" 2>/dev/null
            log "Removed all vhosts for user: $U"
        fi
        reload_veloserve ;;
    AddonDomain::addaddondomain|addaddondomain)
        D=$(get ".get('data',{}).get('args',{}).get('newdomain','')")
        R=$(get ".get('data',{}).get('args',{}).get('dir','')")
        add_vhost "$D" "$R"; reload_veloserve ;;
    AddonDomain::deladdondomain|deladdondomain)
        D=$(get ".get('data',{}).get('args',{}).get('domain','')")
        remove_vhost "$D"; reload_veloserve ;;
    SubDomain::addsubdomain|addsubdomain)
        D=$(get ".get('data',{}).get('args',{}).get('domain','')")
        R=$(get ".get('data',{}).get('args',{}).get('rootdomain_or_dir','')")
        add_vhost "$D" "$R"; reload_veloserve ;;
    SubDomain::delsubdomain|delsubdomain)
        D=$(get ".get('data',{}).get('args',{}).get('domain','')")
        remove_vhost "$D"; reload_veloserve ;;
    Park::park|park)
        D=$(get ".get('data',{}).get('args',{}).get('domain','')")
        U=$(get ".get('data',{}).get('user','')")
        add_vhost "$D" "/home/${U}/public_html"; reload_veloserve ;;
    Park::unpark|unpark)
        D=$(get ".get('data',{}).get('args',{}).get('domain','')")
        remove_vhost "$D"; reload_veloserve ;;
    SSLStorage::add_ssl|add_ssl)
        D=$(get ".get('data',{}).get('domain','')")
        C=$(get ".get('data',{}).get('cert_file','')")
        K=$(get ".get('data',{}).get('key_file','')")
        if [ -n "$D" ] && grep -q "domain = \"${D}\"" "$VELOSERVE_CONFIG" 2>/dev/null; then
            python3 -c "
with open('$VELOSERVE_CONFIG') as f: lines=f.readlines()
out=[]; hit=False
for l in lines:
    if l.strip()=='[[virtualhost]]': hit=False
    if hit and l.strip().startswith('ssl_certificate_key'): out.append('ssl_certificate_key = \"$K\"\n'); continue
    if hit and l.strip().startswith('ssl_certificate'): out.append('ssl_certificate = \"$C\"\n'); continue
    out.append(l)
    if 'domain = \"$D\"' in l: hit=True
with open('$VELOSERVE_CONFIG','w') as f: f.writelines(out)
" 2>/dev/null
            log "Updated SSL: $D"
        fi
        reload_veloserve ;;
    *) log "Unhandled: $EVENT" ;;
esac
exit 0
