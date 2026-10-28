#!/bin/sh
set -eu
mkdir -p source
cp /opt/sqlite/sqlite3.c /opt/sqlite/sqlite3.h /opt/sqlite/shell.c source/
gcc -Isource probe.c -o probe
