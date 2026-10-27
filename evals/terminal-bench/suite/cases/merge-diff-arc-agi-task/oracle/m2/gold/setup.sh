#!/bin/sh
set -eu
cd "$1"
mkdir repo
cd repo
git init
git config user.name Synthetic
git config user.email fixture@example.invalid
git fetch ../bundle1.bundle HEAD:branch1
git fetch ../bundle2.bundle HEAD:branch2
git checkout branch1
git merge --no-ff branch2 -m merge
