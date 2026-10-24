#!/usr/bin/env bash
# Commit only release formulas, and succeed if the same version is already present.
set -euo pipefail
source_dir="${1:?Expected formula directory}"
tap_dir="${2:?Expected tap checkout}"
tag="${3:?Expected release tag}"
for name in shiju shiju-acp shiju-desktop shiju-webview; do
  ruby -c "$source_dir/$name.rb"
done
for name in shiju shiju-acp shiju-desktop shiju-webview; do
  cp "$source_dir/$name.rb" "$tap_dir/Formula/$name.rb"
  git -C "$tap_dir" add "Formula/$name.rb"
done
if git -C "$tap_dir" diff --cached --quiet; then
  echo "Homebrew formulas already match $tag"
else
  git -C "$tap_dir" -c user.name='shiju bot' -c user.email='actions@github.com' \
    commit -m "chore: publish shiju formulas for $tag"
  git -C "$tap_dir" push origin HEAD:refs/heads/main
fi
