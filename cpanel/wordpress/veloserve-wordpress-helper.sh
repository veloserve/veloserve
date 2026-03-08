#!/usr/bin/env bash
set -euo pipefail

PLUGIN_SLUG="veloserve-cache"
DEFAULT_PLUGIN_PACKAGE="/usr/local/src/veloserve/wordpress-plugin/veloserve-cache.zip"

HELPER_VERSION="1.0.0"

usage() {
  cat <<EOF
Usage:
  $0 discover [--home-root /home]
  $0 install --site-path <wp-root> [--plugin-zip <path>] [--force]
  $0 version

Commands:
  discover    Scan cPanel home directories and print JSON with discovered WordPress installs
  install     Install VeloServe plugin zip into a discovered WordPress site
  version     Print helper version
EOF
}

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

discover_sites() {
  local home_root="${1:-/home}"
  local first=1

  printf '{"generated_at":"%s","sites":[' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  if [[ -d "$home_root" ]]; then
    while IFS= read -r -d '' config; do
      local site_root
      local user
      local relative_root
      site_root="$(dirname "$config")"
      relative_root="${site_root#"$home_root"/}"
      user="${relative_root%%/*}"

      if [[ $first -eq 0 ]]; then
        printf ','
      fi
      first=0

      printf '{"user":"%s","path":"%s","wp_config":"%s","status":"discovered"}' \
        "$(json_escape "$user")" \
        "$(json_escape "$site_root")" \
        "$(json_escape "$config")"
    done < <(find "$home_root" -mindepth 3 -maxdepth 4 -type f -name wp-config.php -print0 2>/dev/null)
  fi

  printf ']}'
}

install_plugin() {
  local site_path=""
  local plugin_zip="$DEFAULT_PLUGIN_PACKAGE"
  local force=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --site-path)
        site_path="$2"
        shift 2
        ;;
      --plugin-zip)
        plugin_zip="$2"
        shift 2
        ;;
      --force)
        force=1
        shift
        ;;
      *)
        echo "Unknown option: $1" >&2
        usage >&2
        exit 1
        ;;
    esac
  done

  if [[ -z "$site_path" ]]; then
    echo "--site-path is required" >&2
    exit 1
  fi

  if [[ ! -f "$site_path/wp-config.php" ]]; then
    echo "Not a WordPress root: $site_path" >&2
    exit 1
  fi

  if [[ ! -f "$plugin_zip" ]]; then
    echo "Plugin package not found: $plugin_zip" >&2
    exit 1
  fi

  local plugins_dir="$site_path/wp-content/plugins"
  local plugin_dir="$plugins_dir/$PLUGIN_SLUG"

  mkdir -p "$plugins_dir"

  if [[ -d "$plugin_dir" && $force -ne 1 ]]; then
    echo "Plugin already installed at $plugin_dir (use --force to overwrite)" >&2
    exit 1
  fi

  rm -rf "$plugin_dir"
  unzip -q "$plugin_zip" -d "$plugins_dir"

  if [[ ! -f "$plugin_dir/veloserve-cache.php" ]]; then
    echo "Plugin zip structure invalid; expected $plugin_dir/veloserve-cache.php" >&2
    exit 1
  fi

  local site_owner
  site_owner="$(stat -c '%U:%G' "$site_path" 2>/dev/null || stat -f '%Su:%Sg' "$site_path" 2>/dev/null)"
  if [[ -n "$site_owner" ]]; then
    chown -R "$site_owner" "$plugin_dir" 2>/dev/null || true
  fi

  printf '{"status":"installed","site_path":"%s","plugin_dir":"%s"}\n' "$site_path" "$plugin_dir"
}

main() {
  local command="${1:-}"
  shift || true

  case "$command" in
    discover)
      local home_root="/home"
      while [[ $# -gt 0 ]]; do
        case "$1" in
          --home-root)
            home_root="$2"
            shift 2
            ;;
          *)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
        esac
      done
      discover_sites "$home_root"
      ;;
    install)
      install_plugin "$@"
      ;;
    version)
      printf '{"version":"%s"}\n' "$HELPER_VERSION"
      ;;
    *)
      usage
      exit 1
      ;;
  esac
}

main "$@"
