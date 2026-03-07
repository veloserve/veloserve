#!/usr/bin/env bash
set -euo pipefail

if ! command -v php >/dev/null 2>&1; then
  echo "php binary not found; cannot run plugin flow tests" >&2
  exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
php "$SCRIPT_DIR/plugin-flows-test.php"
