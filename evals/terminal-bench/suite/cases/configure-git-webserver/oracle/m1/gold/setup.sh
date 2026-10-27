#!/bin/sh
set -eu
root=$1
mkdir -p "$root"
git init --bare --initial-branch=master "$root/server.git"
