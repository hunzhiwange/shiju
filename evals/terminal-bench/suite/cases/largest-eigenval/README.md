# Largest eigenvalue diagnostic preparation

Four independent synthetic JSON CLI tasks, each implemented at `/app/solution.py`:

- m1: real square matrix row/column convention and vector norm.
- m2: largest-magnitude eigenvalue of a real 2×2 matrix, including complex pairs.
- m3: zero, repeated, and tied diagonal eigenvalues with deterministic unit vectors.
- m4: finite complex eigenpair residual and Euclidean normalization.

These diagnostics do not replace the original performance task. The original
NumPy/eigenpy entrypoint, random matrices, timing checks, 1 CPU / 2 GiB and
900 second budgets remain unchanged. No paid model calls are authorized.

The checker executes candidate programs in subprocesses and compares numerical
results against independent synthetic probes. JSON booleans cannot stand in for
numbers. Each child has a gold program, empty nop, and three distinct wrong
programs. The red phase deliberately omitted m4 output checking: all three m4
negative tests failed; restoring the check yielded 20 passing fixture tests.

Cached diagnostic tool image (amd64):
`vw-kv-grpc-diagnostics@sha256:5235d587637d5065c1724a0a307c9d0d2fc150cdbd541e0c3174c8845ab40bcc`.
It contains Python plus previously pinned gRPC tools; these tasks use only the
standard library. A fresh probe confirmed `/app` empty and `/tests`, `/solution`,
and `/oracle` absent. Fixtures run offline in fresh containers with 1 CPU,
512 MiB, dropped capabilities, no-new-privileges and read-only root filesystem.

Raw red/green fixture logs, image probe, registry identity, capacity probe,
Promptfoo validation and Clippy receipts are in the ignored directory
`evals/promptfoo/results/qwen38-suite/diagnostics/largest-eigenval/`.
`validation.json` binds all declared fixture/checker bytes and local controls.
`preparation.json` binds ten fresh Harbor controls under the current adapter,
scheduler, runtime, source and fixture identities. Four child oracles scored 1,
four child nops scored 0. The original parent oracle passed all 27 pytest
parameterizations; nop passed 24 and failed three speed checks, scoring 0.
Both controls completed without exceptions or recovery. pytest-json-ctrf 0.3.5
aggregates the 27 parameterizations into three test groups (`retries=8` per
group); the audit independently verified all 27 named pytest stdout outcomes.
No zero-test or infrastructure result was accepted.

Original amd64 parent manifest:
`sha256:cb9e3e3b09d0783f3bbcfeec8d77bc32d3791204fb2be6650b19be4fbc082b2e`.
Config: `sha256:c46388bf47c6faec1324b39da95fd7bb7d1eee9e3a3a61e46f59a34ee1850805`.
After explicit scheduler release, same-digest bounded registry range downloads
filled three missing layers in task-local ignored storage. Five shared cached
layers were read-only reused. Every full compressed SHA256, gzip CRC/EOF and
uncompressed diff_id was checked: C=84,686,418 and exact U=220,840,960 bytes.
Unknown U was reserved at 512 MiB before download; final admission retained
2C + 512 MiB + original 10 GiB storage + 2 GiB tools + 1 GiB evidence + 5 GiB
margin = 20,033,596,580 bytes, below fresh host and VM availability. OCI bytes
streamed into docker load without an on-disk archive. Original config, amd64
architecture and all RootFS layers matched after import.

The original image contains Python 3.13.7 and NumPy 2.3.0. In-container PyPI
probes returned 200 for eigenpy 3.10.3, pytest 8.4.1 and pytest-json-ctrf 0.3.5.
Unmodified solution and verifier installed their own pinned dependencies;
no verifier repair, dependency substitution or source change was needed.
Parent resource receipts prove 1 CPU / 2 GiB; fresh children used 1 CPU /
512 MiB with 0.25 CPU / 64 MiB egress sidecars and no network. Capacity
monitors recorded no floor breach; no task containers remain.

The task-local runtime-v2 launcher was used for the ten controls. Their exact
original scoring bytes and execution results were combined without rerunning
controls, then accepted by production `inputs` and `verify_preparation`.
The combined receipt also lives at the ignored
`preflight/largest-eigenval/preparation.json`; `final-gate.json` records actual
pytest counts, original evidence hashes, resources and the acceptance gate.

Focused commands:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/largest-eigenval/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/largest-eigenval/case.json --run-fixtures
evals/promptfoo/score.sh test
cargo clippy --all-targets --all-features -- -D warnings
```

Clippy exited 0 using dynamically acquired shared slot 1:
`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`.
`CARGO_BUILD_BUILD_DIR` matched, `CARGO_BUILD_JOBS=1`; the lock was released and
cache retained. Clippy is a lint gate with zero behavior tests. No Rust changed.
The historical runtime JSON lacks launcher paths (`historical_root`); use the
ignored task-local runtime-v2 launcher for current calibration, preserving the
historical identity report unchanged.
