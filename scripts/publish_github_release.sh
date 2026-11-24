#!/usr/bin/env bash
set -euo pipefail

assets_dir="$1"
notes="$2"
endpoint="repos/$GITHUB_REPOSITORY/releases"
marker="<!-- shiju-release-run: $GITHUB_RUN_ID -->"
shopt -s nullglob
assets=("$assets_dir"/*)
if (( ${#assets[@]} == 0 )); then
  echo "Refusing to publish a release without packages" >&2
  exit 1
fi
printf '\n%s\n' "$marker" >> "$notes"

# Persist a draft first so an interrupted asset upload can be resumed.
if release=$(gh release view "$RELEASE_TAG" --json databaseId,body); then
  if ! jq -e --arg marker "$marker" '.body | split("\n") | index($marker) != null' <<< "$release" >/dev/null; then
    echo "Release $RELEASE_TAG belongs to a different run" >&2
    exit 1
  fi
  release_id=$(jq -er '.databaseId' <<< "$release")
else
  gh release create "$RELEASE_TAG" --verify-tag --draft --title "$RELEASE_TITLE" --notes-file "$notes"
  release_id=$(gh release view "$RELEASE_TAG" --json databaseId --jq '.databaseId')
fi

remote_assets=$(gh api --paginate --slurp "$endpoint/$release_id/assets?per_page=100" | jq -c 'add // []')
if [[ "$COMPLETE" != true ]] && jq -e 'any(.[]; .name == "dist-manifest.json")' <<< "$remote_assets" >/dev/null; then
  echo "Release $RELEASE_TAG already has a complete dist manifest" >&2
  exit 1
fi
pending=()
for path in "${assets[@]}"; do
  remote=$(jq -c --arg name "${path##*/}" '.[] | select(.name == $name)' <<< "$remote_assets")
  if [[ -z "$remote" ]]; then
    pending+=("$path")
    continue
  fi
  checksum=$(sha256sum < "$path")
  if ! jq -e --arg digest "sha256:${checksum%% *}" '.state == "uploaded" and .digest == $digest' <<< "$remote" >/dev/null; then
    echo "Existing release asset differs or is incomplete: ${path##*/}" >&2
    exit 1
  fi
done
if (( ${#pending[@]} )); then
  gh release upload "$RELEASE_TAG" "${pending[@]}"
fi

make_latest=false
if [[ "$COMPLETE" == true && "$PRERELEASE" == false ]]; then
  make_latest=legacy
fi
gh api --method PATCH "$endpoint/$release_id" \
  -F draft=false -F "prerelease=$PRERELEASE" \
  -f "name=$RELEASE_TITLE" -F "body=@$notes" -f "make_latest=$make_latest" \
  --jq '.html_url'
