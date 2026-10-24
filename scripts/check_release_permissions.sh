#!/usr/bin/env bash
# Exercise release creation and asset upload without compiling or publishing.
set -euo pipefail

: "${GH_REPO:?Set GH_REPO to owner/repository}"
tag="${1:?Usage: bash scripts/check_release_permissions.sh <existing-tag>}"
work_dir="$(mktemp -d)"
release_id=""
cleanup() {
  result=$?
  trap - EXIT
  if [[ -n "$release_id" ]]; then
    if ! gh api --method DELETE "repos/$GH_REPO/releases/$release_id"; then
      echo "Failed to remove probe draft release $release_id; remove it manually." >&2
      result=1
    fi
  fi
  rm -rf "$work_dir"
  exit "$result"
}
trap cleanup EXIT

# An existing release must never be reused or deleted by this probe.
# Creation fails if this tag already has a release; cleanup is armed only after
# creation succeeds and the ID has been captured.
gh api --paginate "repos/$GH_REPO/releases?per_page=100" --jq '.[].tag_name' > "$work_dir/releases.txt"
if grep -Fxq -- "$tag" "$work_dir/releases.txt"; then
  echo "Tag $tag already has a release; choose an existing tag without a release." >&2
  exit 1
fi
printf '%s\n' 'Release permission probe; safe to delete this draft.' > "$work_dir/notes.txt"
# Verify the tag rather than allowing the API to create one implicitly.
gh api "repos/$GH_REPO/git/ref/tags/$tag" --silent
# Use the same create-release endpoint and omit target_commitish, as in the
# production gh command. Capture the ID directly for precise cleanup.
release_id="$(gh api --method POST "repos/$GH_REPO/releases" \
  -f "tag_name=$tag" -F draft=true -f "name=Release permission probe: $tag" \
  -f "body=$(cat "$work_dir/notes.txt")" --jq '.id')"
if [[ ! "$release_id" =~ ^[0-9]+$ ]]; then
  echo "Invalid release ID; inspect and remove the probe draft manually." >&2
  release_id=""
  exit 1
fi
printf '%s\n' 'release upload probe' > "$work_dir/release-probe.txt"
gh release upload "$tag" "$work_dir/release-probe.txt" --repo "$GH_REPO"
echo 'Release creation and asset upload succeeded; removing probe draft (tag preserved).'
