#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/switch_version.sh <version>
  scripts/switch_version.sh --current

Run from the repository root to print the workspace version.
To switch versions, pass 0.7.0 or v0.7.0. This updates the workspace
version in Cargo.toml and refreshes Cargo.lock through Cargo.
USAGE
}

die() {
  echo "[switch_version.sh] ERROR: $*" >&2
  exit 1
}

workspace_version() {
  local manifest="$1"
  local new_version="${2:-}"
  awk -v new_version="$new_version" '
    /^[[:space:]]*\[/ {
      in_workspace_package = ($0 ~ /^[[:space:]]*\[workspace\.package\][[:space:]]*(#.*)?$/)
    }
    in_workspace_package && /^[[:space:]]*version[[:space:]]*=[[:space:]]*"[^"]+"/ {
      found++
      if (new_version == "") {
        split($0, fields, "\"")
        version = fields[2]
      } else {
        sub(/"[^"]+"/, "\"" new_version "\"")
      }
    }
    new_version != "" { print }
    END {
      if (found != 1) { exit 1 }
      if (new_version == "") { print version }
    }
  ' "$manifest" || die "Could not read the workspace package version in $manifest"
}

if [[ $# -ne 1 ]]; then
  usage
  exit 1
fi

case "$1" in
  -h|--help)
    usage
    exit 0
    ;;
  --current)
    workspace_version Cargo.toml
    exit 0
    ;;
esac

VERSION="${1#v}"
SEMVER_PATTERN='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'
[[ "$VERSION" =~ $SEMVER_PATTERN ]] || die "Invalid version: $1"

command -v cargo >/dev/null 2>&1 || die "Required command not found: cargo"
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

BACKUP="$(git rev-parse --git-path shiju-switch-version.lock)"
mkdir -m 700 "$BACKUP" || die "Could not acquire the version update lock at $BACKUP"
RESTORE=0
cleanup() {
  local status=$?
  if [[ "$status" -ne 0 && "$RESTORE" -eq 1 ]]; then
    if ! cp -p "$BACKUP/Cargo.toml" Cargo.toml || ! cp -p "$BACKUP/Cargo.lock" Cargo.lock; then
      echo "[switch_version.sh] ERROR: Restore Cargo.toml and Cargo.lock from $BACKUP" >&2
      return
    fi
  fi
  rm -rf "$BACKUP"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

cp -p Cargo.toml Cargo.lock "$BACKUP/"
workspace_version Cargo.toml "$VERSION" > "$BACKUP/updated.toml"
RESTORE=1
cat "$BACKUP/updated.toml" > Cargo.toml
cargo update --workspace

echo "[switch_version.sh] Updated workspace version to $VERSION"
