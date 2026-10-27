# db-wal-recovery diagnostics

Four independent synthetic tasks isolate WAL parsing, transaction recovery,
checksum/commit boundaries, and a queryable standalone database. No parent
records, hidden tests, reference solution, or model traces enter public fixtures.
Only declared fixtures enter each fresh environment; oracle and checker files
are injected in the solution/verifier phases. No model requests are made.

| Child | Required artifact | Negative controls |
| --- | --- | --- |
| m1 | Typed WAL header and all frame offsets in result.json | wrong endian, wrong offset, dropped uncommitted frame |
| m2 | Sorted recovered records in result.json | base only, stale update, missing row |
| m3 | Valid checksum prefix and last committed page versions in result.json | accepted checksum corruption, uncommitted version, incorrect commit |
| m4 | Standalone recovered.db, queried immutable/read-only with integrity_check | base only, missing row, stale update |

m1/m3 use hand-constructed 512-byte pages, repeated page versions, and a damaged
frame followed by a valid-looking suffix and incomplete tail. m2/m4 contain real
SQLite databases and XOR-encoded WAL files generated from independent synthetic
transactions with different keys. The independent hidden audit script parses the
binary files and recovers both SQLite fixtures without using candidate outputs.
The checker verifies input SHA256, complete typed JSON (including duplicate-key
and bool/integer distinctions), or actual SQLite query results. It rejects missing,
malformed, oversized, symlinked, and incorrect artifacts. Gold artifacts calibrate
the artifact contract; they do not demonstrate a claimed recovery algorithm.

The original task remains frozen at tree
`2072a10f19356e3cbb522ec00eba9a15f1a2a1e3`, with 1 CPU, 2048MiB memory,
10240MiB storage, and 900-second agent/verifier limits. Its image is
`alexgshaw/db-wal-recovery@sha256:0e33ea5ec823975d1bd6c3778395c9f94251dd88f571146057bff6adb7e4594e`
(linux/amd64). Diagnostic environments use the native digest in case.json,
1 CPU/512MiB/1024MiB and offline networking; fixture controls additionally drop
capabilities and use no-new-privileges, a read-only root and a bounded tmpfs.
Docker preserves storage declarations but does not enforce writable-layer quotas.
Model, output/input budgets and single-attempt policy remain frozen in case.json.

The task's prescribed runtime.json is a historical identity receipt without
execution settings (`historical_root` is missing). The ignored per-case runtime.json
selects the verified runtime-v2 generation and unchanged source/case roots.
Verifier-only bootstrap supplies hash-pinned CPython 3.13.7, uv 0.9.5 and official
pytest 8.4.1 / pytest-json-ctrf 0.3.5 dependency caches after the agent exits.
After a preserved 900-second nop verifier timeout on HTTP apt downloads, the
verifier bootstrap switches the same Ubuntu repositories to HTTPS and limits
connection timeouts/retries. Repository signatures and package selection remain
unchanged. HTTPS oracle passed all seven official tests; its later nop calibration
was interrupted while apt again retried a partial index (unscored, not a capability
failure). The final verifier-only PATH includes authentic cached curl, uv and
Python, plus an explicit apt-get wrapper that executes the real /usr/bin/apt-get
through GNU timeout (30 seconds, five-second kill grace). It preserves failures
and bounds redundant online dependency acquisition; it never produces rewards.
Both controls must still execute all seven unmodified official pytest tests.
It does not change the original image, official test bytes, scoring, or agent
inputs. Cached dependencies and all raw failures remain in the ignored results.

Verification from repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/db-wal-recovery/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/db-wal-recovery/case.json --run-fixtures
evals/promptfoo/results/task25/harbor-venv/bin/python evals/terminal-bench/suite/cases/db-wal-recovery/oracle/verify_fixtures.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/db-wal-recovery/runtime.json --parent-id db-wal-recovery --output evals/promptfoo/results/qwen38-suite/preflight/db-wal-recovery-bounded-deps
```

Raw commands, image and dependency hashes, resources, source integrity, fixture
calibration and Harbor receipts are under the ignored
`evals/promptfoo/results/qwen38-suite/diagnostics/db-wal-recovery/` and the preflight
output. The initial permissive checker failed 24 rejection controls across 22
tests; the corrected checker passed 27 focused tests and 20 fresh offline fixture
controls. Clippy and Promptfoo configuration validation exited 0. Clippy acquired
slot 2 dynamically; CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR were both
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-2`, with
CARGO_BUILD_JOBS=1. The owned lock was released and the cache retained.
The exact Rust gate was `cargo clippy --all-targets --all-features -- -D warnings`
(exit 0); it is a static gate, not a Rust behavior test. No Rust files changed.

Final behavior acceptance: all ten unified preflight controls passed
(parent and four children each oracle=1, nop=0), with complete evidence bindings
verified by verify_preparation. The parent oracle passed all seven official
pytest tests; the nop ran and failed all seven (zero skipped), confirming actual
artifact rejection rather than a dependency failure. Eight standalone fresh
Harbor child controls and twenty offline fixture controls also passed. No paid
model calls were made. The historical HTTP timeout and interrupted HTTPS nop
remain archived separately as unscored infrastructure results.
An earlier Clippy gate also passed in dynamically acquired slot 1 at
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`; its log and
slot receipt are retained separately. The final gate above used slot 2.
