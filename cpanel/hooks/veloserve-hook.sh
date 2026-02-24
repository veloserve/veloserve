#!/bin/bash
# VeloServe cPanel Standardized Hook Handler
#
# Handles cPanel events to keep VeloServe config in sync:
#   - Account create/remove
#   - Domain add/remove (addon, subdomain, park)
#   - SSL install/remove (AutoSSL / Let's Encrypt)
#
# Install:
#   cp hooks/veloserve-hook.sh /usr/local/veloserve/hooks/
#   /usr/local/cpanel/bin/manage_hooks add script /usr/local/veloserve/hooks/veloserve-hook.sh

VELOSERVE_CONFIG="/etc/veloserve/veloserve.toml"
LOGFILE="/var/log/veloserve/hooks.log"

log() { echo "$(date '+%Y-%m-%d %H:%M:%S') [hook] $*" >> "$LOGFILE" 2>/dev/null; }

EVENT_DATA=$(cat)
EVENT="$1"
log "Event: $EVENT"

reload_veloserve() {
    if systemctl is-active --quiet veloserve 2>/dev/null; then
        systemctl reload veloserve 2>/dev/null || systemctl restart veloserve 2>/dev/null
        log "Reloaded"
    fi
}

add_vhost() {
    local domain="$1" docroot="$2" cert="$3" key="$4"
    [ -z "$domain" ] || [ -z "$docroot" ] && return 1
    grep -q "domain = \"${domain}\"" "$VELOSERVE_CONFIG" 2>/dev/null && return 0
    {
        echo ""; echo "[[virtualhost]]"
        echo "domain = \"${domain}\""
        echo "root = \"${docroot}\""
        echo "platform = \"generic\""
        [ -n "$cert" ] && [ -f "$cert" ] && echo "ssl_certificate = \"${cert}\""
        [ -n "$key" ] && [ -f "$key" ] && echo "ssl_certificate_key = \"${key}\""
    } >> "$VELOSERVE_CONFIG"
    log "Added vhost: $domain -> $docroot"
}

remove_vhost() {
    local domain="$1"
    [ -z "$domain" ] && return 1
    python3 -c "
import re
with open('$VELOSERVE_CONFIG') as f: c=f.read()
c=re.sub(r'\n*\[\[virtualhost\]\]\s*\n(?:[^\[]*\n)*?domain\s*=\s*\"$domain\"(?:\n(?!\[\[)[^\n]*)*','',c)
with open('$VELOSERVE_CONFIG','w') as f: f.write(c)
" 2>/dev/null
    log "Removed vhost: $domain"
}

update_ssl() {
    local domain="$1" cert="$2" key="$3"
    [ -z "$domain" ] && return 1
    if grep -q "domain = \"${domain}\"" "$VELOSERVE_CONFIG" 2>/dev/null; then
        python3 -c "
with open('$VELOSERVE_CONFIG') as f: lines=f.readlines()
out=[]; hit=False
for l in lines:
    if l.strip()=='[[virtualhost]]': hit=False
    if hit and l.strip().startswith('ssl_certificate_key'): out.append('ssl_certificate_key = \"$key\"\n'); continue
    if hit and l.strip().startswith('ssl_certificate'): out.append('ssl_certificate = \"$cert\"\n'); continue
    out.append(l)
    if 'domain = \"$domain\"' in l: hit=True
with open('$VELOSERVE_CONFIG','w') as f: f.writelines(out)
" 2>/dev/null
        log "Updated SSL: $domain"
    fi
}

get() { echo "$EVENT_DATA" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d$1)" 2>/dev/null; }

case "$EVENT" in
    Accounts::Create)
        D=$(get ".get('data',{}).get('domain','')")
        H=$(get ".get('data',{}).get('homedir','')")
        add_vhost "$D" "${H}/public_html"; reload_veloserve ;;
    Accounts::Remove)
        D=$(get ".get('data',{}).get('domain','')")
        remove_vhost "$D"; reload_veloserve ;;
    AddonDomain::addaddondomain)
        D=$(get ".get('data',{}).get('args',{}).get('newdomain','')")
        R=$(get ".get('data',{}).get('args',{}).get('dir','')")
        add_vhost "$D" "$R"; reload_veloserve ;;
    AddonDomain::deladdondomain)
        D=$(get ".get('data',{}).get('args',{}).get('domain','')")
        remove_vhost "$D"; reload_veloserve ;;
    SubDomain::addsubdomain)
        D=$(get ".get('data',{}).get('args',{}).get('domain','')")
        R=$(get ".get('data',{}).get('args',{}).get('rootdomain_or_dir','')")
        add_vhost "$D" "$R"; reload_veloserve ;;
    SubDomain::delsubdomain)
        D=$(get ".get('data',{}).get('args',{}).get('domain','')")
        remove_vhost "$D"; reload_veloserve ;;
    Park::park)
        D=$(get ".get('data',{}).get('args',{}).get('domain','')")
        U=$(get ".get('data',{}).get('user','')")
        add_vhost "$D" "/home/${U}/public_html"; reload_veloserve ;;
    Park::unpark)
        D=$(get ".get('data',{}).get('args',{}).get('domain','')")
        remove_vhost "$D"; reload_veloserve ;;
    SSLStorage::add_ssl)
        D=$(get ".get('data',{}).get('domain','')")
        C=$(get ".get('data',{}).get('cert_file','')")
        K=$(get ".get('data',{}).get('key_file','')")
        update_ssl "$D" "$C" "$K"; reload_veloserve ;;
    *) log "Unhandled: $EVENT" ;;
esac
exit 0
