# Task 7 offline validation — 2026-10-27

No Qwen requests or official suite scores were produced. The scheduler preserves
`token-plan/qwen3.8-flash`, output 32000, input window 200000, one attempt, and
24 model requests / 32 tool executions per diagnostic.

- Python: 53 tests across `test_run`, `test_preparation`, `test_budget`,
  `test_agent`, `test_runtime`, `test_scoring`, `test_worker`. The 16 scheduler
  tests include real Docker scoring/recovery subcases and real countable
  loopback HTTP worker processes. Preparation tests use synthetic 89-parent
  fixtures to verify all 445 entries and reject missing/drifting evidence.
- Native helper: 1 Rust test; frozen Linux runtime: 3 focused Rust tests covering
  required-hook absence/disablement, trusted model admission and panic denial.
- Frozen ELF: 12 live gateway calibration checks passed. Serial, parallel,
  nested batch and child executions share the tool ledger; the 33rd tool and
  25th model request are denied. Missing trust, corrupted state and hook crashes
  deny execution. Both HTTP 400 and 502 produce one request without replay.
  A separate 48-process helper control grants exactly 32 permits.
- `cargo fmt --all` and required
  `cargo clippy --all-targets --all-features -- -D warnings` exited 0.
  The dependency `block 0.1.6` emits an existing future compatibility notice.

Cargo stages dynamically claimed these shared slots, with both
`CARGO_TARGET_DIR` and `CARGO_BUILD_BUILD_DIR` set to the same absolute path,
and `CARGO_BUILD_JOBS=1`:

- Slot 1: `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`:
  formatting, helper test and Clippy; exit 0, one executed Rust test.
- Slot 2: `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-2`:
  `cargo test -p vw-agent --no-default-features --lib --target x86_64-unknown-linux-gnu --no-run --message-format=json`,
  followed by three individually filtered executions in an offline amd64
  container; exit 0, three executed tests. Then
  `cargo build -p vw-cli --bin shiju --target x86_64-unknown-linux-gnu --locked`
  and native helper compilation; both exit 0.

The macOS panic-catching test process aborted in the host unwind runtime; the
same actual Rust test passed on the Linux target. An additional Linux run of
the existing plugin test group passed 10/11; its unchanged spawn-failure error
text assertion failed. Neither was hidden or counted as a passing test. The
three focused target tests and production gateway calibration above passed.
An uncached duplicate compile was canceled after switching to the available
cached slot; its owned processes and lock were released. All final stages
released their own locks; caches remain.

New frozen identities are recorded in `runtime-v2.json`:

- Source: `9fcb685d29eb49342983457ab75bedc8293dbf3d70cdb35ad7a48a2298bc5bcf`
- ELF: `847823c5cd99a6828cddfc2e3e5dbb67c5ddcc9f8693a13d539d8a8fa47c9dae`
- Gate: `32521d760857047a6f5d40b490803cbc552293048e445f361b7a45e21fdc4346`

The original historical ELF/source were preserved. The new historical source
contains only execution-gate repairs and the provider retry-boundary fix
already present in current source; its exact patch is `runtime-v2.diff`.

Local raw evidence remains under
`evals/promptfoo/results/qwen38-suite/runtime-v2/`:
`python-validation.json`, per-test logs, `final-rust-validation.log`,
`linux-unit-evidence.json`, `build-evidence.json`, and
`calibration/acceptance.json` with raw evidence SHA-256 values.

Next: prepare each parent's diagnostic definitions and oracle/nop receipts,
then freeze the complete manifest before authorizing a paid experiment.
