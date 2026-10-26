#!/bin/bash
set -u
cd /Users/Shared/work/dir/data/codes/vibe-window
mkdir -p target/rust-slot-locks
while :; do
  for slot in {1..10}; do
    if mkdir "target/rust-slot-locks/slot-$slot.lock" 2>/dev/null; then
      lock="$PWD/target/rust-slot-locks/slot-$slot.lock"
      printf 'plan=统一上下文压缩\nthread=01a17654\npid=%s\n' "$$" > "$lock/owner"
      export CARGO_TARGET_DIR="$PWD/target/rust-slot-$slot"
      export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR"
      export RUST_MIN_STACK=16777216
      trap 'rm -f "$lock/owner"; rmdir "$lock"' EXIT
      printf 'slot=%s CARGO_TARGET_DIR=%s\n' "$slot" "$CARGO_TARGET_DIR"
      "$@"
      exit $?
    fi
  done
  sleep 2
done
