#!/usr/bin/env bash
set -euo pipefail

SCRIPT_NAME="$(basename "$0")"

usage() {
  cat <<'USAGE'
Usage:
  scripts/switch_version.sh <version>

Batch update VibeWindow package versions.

Package manifests are discovered automatically:
  crates/*/Cargo.toml
  crates/libs/*/Cargo.toml

It also updates matching package entries in Cargo.lock.
It also updates local path dependency version constraints between VibeWindow crates.

Examples:
  scripts/switch_version.sh 0.1.10
  scripts/switch_version.sh v0.1.10
USAGE
}

die() {
  echo "[$SCRIPT_NAME] ERROR: $*" >&2
  exit 1
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    die "Required command not found: $1"
  fi
}

normalize_version() {
  local value="$1"
  value="${value#v}"
  printf '%s' "$value"
}

validate_version() {
  local value="$1"
  local semver_pattern='^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$'

  if [[ ! "$value" =~ $semver_pattern ]]; then
    die "Version must look like 0.1.10 or 0.1.10-rc.1 (received: $value)"
  fi
}

write_tmp() {
  local tmp_file="$1"
  local target_file="$2"

  if cmp -s "$tmp_file" "$target_file"; then
    rm -f "$tmp_file"
  else
    mv "$tmp_file" "$target_file"
  fi
}

update_manifest_version() {
  local file="$1"
  local version="$2"
  local tmp_file
  tmp_file="$(mktemp)"

  if awk -v version="$version" '
    BEGIN {
      in_package = 0
      changed = 0
    }
    /^\[package\][[:space:]]*$/ {
      in_package = 1
      print
      next
    }
    /^\[/ && $0 !~ /^\[package\][[:space:]]*$/ {
      in_package = 0
    }
    in_package && changed == 0 && /^version[[:space:]]*=/ {
      print "version = \"" version "\""
      changed = 1
      next
    }
    {
      print
    }
    END {
      if (changed == 0) {
        exit 42
      }
    }
  ' "$file" > "$tmp_file"; then
    write_tmp "$tmp_file" "$file"
  else
    local status=$?
    rm -f "$tmp_file"
    if [[ "$status" -eq 42 ]]; then
      die "Could not find [package] version in $file"
    fi
    die "Failed to update $file"
  fi
}

update_local_path_dependency_versions() {
  local file="$1"
  local version="$2"
  local manifests_csv="$3"
  local tmp_file
  tmp_file="$(mktemp)"

  if awk -v version="$version" -v manifests_csv="$manifests_csv" '
    BEGIN {
      count = split(manifests_csv, manifests, ",")
      for (idx = 1; idx <= count; idx++) {
        targets[manifests[idx]] = 1
      }
    }

    function is_local_path(path,    dir, parts, count, idx, resolved) {
      if (path == "" || path ~ /^\//) {
        return 0
      }
      dir = FILENAME
      sub(/[^\/]+$/, "", dir)
      count = split(dir path, parts, "/")
      for (idx = 1; idx <= count; idx++) {
        if (parts[idx] == "..") {
          sub(/[^\/]+\/$/, "", resolved)
        } else if (parts[idx] != "." && parts[idx] != "") {
          resolved = resolved parts[idx] "/"
        }
      }
      return (resolved "Cargo.toml") in targets
    }

    function flush_dependency_table(    idx) {
      if (version_line && is_local_path(table_path)) {
        sub(/"[^"]*"/, "\"" version "\"", table[version_line])
      }
      for (idx = 1; idx <= table_size; idx++) {
        print table[idx]
        delete table[idx]
      }
      table_size = version_line = 0
      table_path = ""
    }

    /^[[:space:]]*\[/ {
      flush_dependency_table()
      dependency_table = ($0 ~ /^[[:space:]]*\[(target\..*\.)?(dependencies|dev-dependencies|build-dependencies)\./)
      inline_dependencies = ($0 ~ /^[[:space:]]*\[(target\..*\.)?(dependencies|dev-dependencies|build-dependencies)\]/)
    }
    dependency_table {
      table[++table_size] = $0
      if ($0 ~ /^[[:space:]]*path[[:space:]]*=[[:space:]]*"/) {
        table_path = $0
        sub(/^[^"]*"/, "", table_path)
        sub(/".*$/, "", table_path)
      }
      if ($0 ~ /^[[:space:]]*version[[:space:]]*=[[:space:]]*"/) {
        version_line = table_size
      }
      next
    }
    inline_dependencies && /^[[:space:]]*[[:alnum:]_-]+[[:space:]]*=[[:space:]]*\{/ &&
        match($0, /[{,][[:space:]]*path[[:space:]]*=[[:space:]]*"[^"]+"/) {
      path = substr($0, RSTART, RLENGTH)
      sub(/^[^"]*"/, "", path)
      sub(/"$/, "", path)
      if (is_local_path(path) && match($0, /[{,][[:space:]]*version[[:space:]]*=[[:space:]]*"[^"]*"/)) {
        constraint = substr($0, RSTART, RLENGTH)
        sub(/"[^"]*"/, "\"" version "\"", constraint)
        $0 = substr($0, 1, RSTART - 1) constraint substr($0, RSTART + RLENGTH)
      }
    }
    { print }
    END { flush_dependency_table() }
  ' "$file" > "$tmp_file"; then
    write_tmp "$tmp_file" "$file"
  else
    rm -f "$tmp_file"
    die "Failed to update local path dependency versions in $file"
  fi
}

update_lockfile_versions() {
  local version="$1"
  local names_csv="$2"
  local lockfile="Cargo.lock"
  local tmp_file

  [[ -f "$lockfile" ]] || die "Missing $lockfile"
  tmp_file="$(mktemp)"

  if awk -v version="$version" -v names_csv="$names_csv" '
    BEGIN {
      split(names_csv, names, ",")
      for (idx in names) {
        targets[names[idx]] = 1
      }
      in_package = 0
      package_name = ""
      changed = 0
    }
    /^\[\[package\]\][[:space:]]*$/ {
      in_package = 1
      package_name = ""
      print
      next
    }
    /^\[/ && $0 !~ /^\[\[package\]\][[:space:]]*$/ {
      in_package = 0
      package_name = ""
    }
    in_package && /^name[[:space:]]*=/ {
      package_name = $0
      sub(/^name[[:space:]]*=[[:space:]]*"/, "", package_name)
      sub(/".*$/, "", package_name)
      print
      next
    }
    in_package && package_name in targets && /^version[[:space:]]*=/ {
      print "version = \"" version "\""
      changed += 1
      next
    }
    {
      print
    }
    END {
      if (changed == 0) {
        exit 42
      }
    }
  ' "$lockfile" > "$tmp_file"; then
    write_tmp "$tmp_file" "$lockfile"
  else
    local status=$?
    rm -f "$tmp_file"
    if [[ "$status" -eq 42 ]]; then
      die "Could not find matching packages in $lockfile"
    fi
    die "Failed to update $lockfile"
  fi
}

if [[ $# -ne 1 || "$1" == "-h" || "$1" == "--help" ]]; then
  usage
  if [[ $# -eq 1 && ( "$1" == "-h" || "$1" == "--help" ) ]]; then
    exit 0
  fi
  exit 1
fi

require_cmd awk
require_cmd cmp
require_cmd git

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  die "Run this script inside the VibeWindow git repository"
fi

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

VERSION="$(normalize_version "$1")"
validate_version "$VERSION"

shopt -s nullglob
MANIFEST_FILES=(crates/*/Cargo.toml crates/libs/*/Cargo.toml)
shopt -u nullglob
[[ ${#MANIFEST_FILES[@]} -gt 0 ]] || die "No package manifests found under crates"

LOCK_PACKAGE_NAMES=()
for manifest in "${MANIFEST_FILES[@]}"; do
  package_name="$(awk '
    /^\[package\][[:space:]]*$/ { in_package = 1; next }
    /^\[/ { in_package = 0 }
    in_package && /^name[[:space:]]*=/ {
      sub(/^name[[:space:]]*=[[:space:]]*"/, "")
      sub(/".*$/, "")
      print
      exit
    }
  ' "$manifest")"
  [[ -n "$package_name" ]] || die "Missing package name: $manifest"
  LOCK_PACKAGE_NAMES+=("$package_name")
done

MANIFESTS_CSV="$(IFS=,; printf '%s' "${MANIFEST_FILES[*]}")"
for manifest in "${MANIFEST_FILES[@]}"; do
  update_manifest_version "$manifest" "$VERSION"
  update_local_path_dependency_versions "$manifest" "$VERSION" "$MANIFESTS_CSV"
  echo "[$SCRIPT_NAME] Updated $manifest -> $VERSION"
done

LOCK_NAMES_CSV="$(IFS=,; printf '%s' "${LOCK_PACKAGE_NAMES[*]}")"
update_lockfile_versions "$VERSION" "$LOCK_NAMES_CSV"
echo "[$SCRIPT_NAME] Updated Cargo.lock package entries -> $VERSION"

echo "[$SCRIPT_NAME] Done"
