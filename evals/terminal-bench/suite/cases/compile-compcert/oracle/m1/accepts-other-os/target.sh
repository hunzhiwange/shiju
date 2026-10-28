#!/bin/sh
set -eu
[ "$#" = 2 ] || exit 2
case "$1" in
  x86_64|amd64) echo x86_64-linux ;;
  aarch64|arm64) echo aarch64-linux ;;
  *) exit 2 ;;
esac
