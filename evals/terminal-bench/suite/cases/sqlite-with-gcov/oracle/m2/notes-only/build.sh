#!/bin/sh
set -eu
mkdir -p build
cd build
gcc -O0 -g -ftest-coverage -c /opt/sqlite/sqlite3.c -o sqlite3.o
gcc -O0 -g -ftest-coverage -c /opt/sqlite/shell.c -o shell.o
gcc --coverage shell.o sqlite3.o -ldl -lpthread -lm -o sqlite3
