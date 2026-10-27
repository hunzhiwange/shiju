#!/bin/sh
set -eu
root=$1
mkdir -p "$root"
git init --initial-branch=master "$root/server.git"
