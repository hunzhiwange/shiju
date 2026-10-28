#!/bin/sh
set -eu
mkdir -p /app/install/bin
ln -sf /usr/bin/gcc /app/install/bin/ccomp
