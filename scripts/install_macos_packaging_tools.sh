#!/usr/bin/env bash
set -euo pipefail
: "${CARGO_TARGET_DIR:?Cargo shared slot must be acquired before installation}"
root="${1:?absolute packaging tool root required}"
[[ "$root" == /* ]] || { echo 'Packaging tool root must be absolute' >&2; exit 1; }
export PATH="$root/bin:$PATH"
expected='cargo-bundle v0.12.0'
if [[ ! -x "$root/bin/cargo-bundle" ]] || [[ "$("$root/bin/cargo-bundle" --version)" != "$expected" ]]; then
  cargo install cargo-bundle --version 0.12.0 --locked --force --root "$root"
fi
actual="$("$root/bin/cargo-bundle" --version)"
[[ "$actual" == "$expected" ]] || { echo "Unexpected cargo-bundle version: $actual" >&2; exit 1; }
printf '%s\n' "$actual"
if [[ -n "${GITHUB_PATH:-}" ]]; then
  printf '%s\n' "$root/bin" >> "$GITHUB_PATH"
fi
