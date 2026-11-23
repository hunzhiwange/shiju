#!/usr/bin/env bash
set -euo pipefail

# Keep the cargo-dist packages and optional crates.io release packages in sync.
RELEASE_PACKAGES=(
  vw-api-types vw-shared vw-config-types vw-gateway-client vw-acp
  vw-provider-resolver vw-agent vw-cli vw-webview vw-desktop
)

usage() {
  cat <<'USAGE'
Usage:
  scripts/switch_version.sh <version>
  scripts/switch_version.sh --current
  scripts/switch_version.sh --check
  scripts/switch_version.sh --packages

Run from the repository root. To switch versions, pass 0.7.0 or v0.7.0.
This updates the product version inherited by the release packages and
refreshes Cargo.lock. Internal package versions stay pinned.
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

check_release_versions() {
  local version
  version="$(workspace_version Cargo.toml)"
  cargo metadata --no-deps --format-version 1 | perl -0MJSON::PP -e '
    my ($version, @release_packages) = @ARGV;
    my %release = map { $_ => 1 } @release_packages;
    my $metadata = decode_json(<STDIN>);
    my $invalid = 0;
    for my $crate (@{$metadata->{packages}}) {
      my ($name, $manifest) = @$crate{qw(name manifest_path)};
      open my $file, "<", $manifest or die "$manifest: $!";
      my $contents = <$file>;
      my ($package) = $contents =~ /^\h*\[package\]\h*(?:\#[^\n]*)?\R(.*?)(?=^\h*\[|\z)/ms;
      die "Missing [package] in $manifest\n" unless defined $package;
      if (delete $release{$name}) {
        unless ($package =~ /^\h*version\.workspace\h*=\h*true\b/m && $crate->{version} eq $version) {
          warn "$name must inherit release version $version\n";
          $invalid = 1;
        }
      } elsif ($package !~ /^\h*version\h*=\h*["\x27]/m) {
        warn "$name must pin its internal crate version in $manifest\n";
        $invalid = 1;
      }
    }
    die "Missing release packages: " . join(", ", sort keys %release) . "\n" if %release;
    exit 1 if $invalid;
  ' "$version" "${RELEASE_PACKAGES[@]}"
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
  --check)
    check_release_versions
    exit 0
    ;;
  --packages)
    printf '%s\n' "${RELEASE_PACKAGES[@]}"
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

check_release_versions
cp -p Cargo.toml Cargo.lock "$BACKUP/"
workspace_version Cargo.toml "$VERSION" > "$BACKUP/updated.toml"
RESTORE=1
cat "$BACKUP/updated.toml" > Cargo.toml
cargo update --workspace

echo "[switch_version.sh] Updated release packages to $VERSION"
