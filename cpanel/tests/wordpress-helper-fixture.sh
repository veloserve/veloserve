#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HELPER="$ROOT_DIR/cpanel/wordpress/veloserve-wordpress-helper.sh"
TMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

mkdir -p "$TMP_DIR/home/alice/public_html" "$TMP_DIR/home/bob/www"
touch "$TMP_DIR/home/alice/public_html/wp-config.php"
touch "$TMP_DIR/home/bob/www/wp-config.php"

output="$($HELPER discover --home-root "$TMP_DIR/home")"

echo "$output" | grep -q '"user":"alice"'
echo "$output" | grep -q '"user":"bob"'

echo "Discovery fixture test passed."

mkdir -p "$TMP_DIR/package/veloserve-cache"
cat > "$TMP_DIR/package/veloserve-cache/veloserve-cache.php" <<'PHP'
<?php
// fixture plugin entrypoint
PHP

(
  cd "$TMP_DIR/package"
  zip -q -r "$TMP_DIR/veloserve-cache.zip" veloserve-cache
)

install_json="$($HELPER install --site-path "$TMP_DIR/home/alice/public_html" --plugin-zip "$TMP_DIR/veloserve-cache.zip")"

echo "$install_json" | grep -q '"status":"installed"'
[[ -f "$TMP_DIR/home/alice/public_html/wp-content/plugins/veloserve-cache/veloserve-cache.php" ]]

echo "Install fixture test passed."
