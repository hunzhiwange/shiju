#!/bin/sh
set -eu
export GCOV_PREFIX="$PWD/discarded"
export GCOV_PREFIX_STRIP=2
/opt/sqlite/sqlite3 "$1" < "$2"
