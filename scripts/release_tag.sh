#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/release_tag.sh <tag> [--push]

Create an annotated release tag from the current checkout.

Requirements:
- tag must match vX.Y.Z (optional suffix like -rc.1)
- working tree must be clean
- HEAD must match origin's default branch
- tag must not already exist locally or on origin

Options:
  --push   Push the tag to origin after creating it
USAGE
}

if [[ $# -lt 1 || $# -gt 2 ]]; then
  usage
  exit 1
fi

TAG="$1"
PUSH_TAG="false"
if [[ $# -eq 2 ]]; then
  if [[ "$2" != "--push" ]]; then
    usage
    exit 1
  fi
  PUSH_TAG="true"
fi

SEMVER_PATTERN='^v[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$'
if [[ ! "$TAG" =~ $SEMVER_PATTERN ]]; then
  echo "error: tag must match vX.Y.Z or vX.Y.Z-suffix (received: $TAG)" >&2
  exit 1
fi

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "error: run this script inside the git repository" >&2
  exit 1
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "error: working tree is not clean; commit or stash changes first" >&2
  exit 1
fi

DEFAULT_BRANCH="$(git ls-remote --symref origin HEAD | awk '$1 == "ref:" && $3 == "HEAD" { sub(/^refs\/heads\//, "", $2); print $2 }')"
if [[ -z "$DEFAULT_BRANCH" ]]; then
  echo "error: could not determine origin's default branch." >&2
  exit 1
fi
echo "Fetching origin/$DEFAULT_BRANCH and tags..."
git fetch --quiet origin "refs/heads/$DEFAULT_BRANCH:refs/remotes/origin/$DEFAULT_BRANCH" --tags

HEAD_SHA="$(git rev-parse HEAD)"
DEFAULT_SHA="$(git rev-parse "refs/remotes/origin/$DEFAULT_BRANCH")"
if [[ "$HEAD_SHA" != "$DEFAULT_SHA" ]]; then
  echo "error: HEAD ($HEAD_SHA) is not origin/$DEFAULT_BRANCH ($DEFAULT_SHA)." >&2
  echo "hint: checkout/update $DEFAULT_BRANCH before cutting a release tag." >&2
  exit 1
fi

if git show-ref --tags --verify --quiet "refs/tags/$TAG"; then
  echo "error: tag already exists locally: $TAG" >&2
  exit 1
fi

if git ls-remote --exit-code --tags origin "refs/tags/$TAG" >/dev/null 2>&1; then
  echo "error: tag already exists on origin: $TAG" >&2
  exit 1
fi

MESSAGE="vibewindow $TAG"
git tag -a "$TAG" -m "$MESSAGE"
echo "Created annotated tag: $TAG"

if [[ "$PUSH_TAG" == "true" ]]; then
  git push origin "$TAG"
  echo "Pushed tag to origin: $TAG"
else
  echo "Next step: git push origin $TAG"
fi
