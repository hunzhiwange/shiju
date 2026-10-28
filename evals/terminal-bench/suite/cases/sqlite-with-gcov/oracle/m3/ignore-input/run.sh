#!/bin/sh
set -eu
export GCOV_PREFIX="$PWD/coverage"
export GCOV_PREFIX_STRIP=2
/opt/sqlite/sqlite3 ":memory:" "SELECT 1;"
