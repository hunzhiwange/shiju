# extract-elf diagnostics

Four independent synthetic artifact tasks isolate ELF header offsets (m1),
half-open virtual-address/file-offset mapping (m2), endian word decoding and
region tails (m3), and unsigned decimal-address JSON memory output (m4).
Each child reads only its declared synthetic inputs and writes `/app/result.json`.
No parent solution, hidden test, model trace, or parent input is copied into a
child image. Gold outputs are independently checked from the fixture format in
`tests/test_cases.py`; the checker verifies actual JSON artifacts, exact types,
complete coverage, unchanged fixture hashes and duplicate/symlink rejection.
Each child has three distinct error artifacts plus nop.

The official parent remains unchanged at source tree
`5005c9867da9cb9a346ef48b5cafe5b0b2082197`. Its actual official checker remains
authoritative, including its coverage/accuracy semantics; diagnostic strictness
does not alter the parent score. Official agent/verifier limits remain 900/900
seconds, 1 CPU and 2048 MiB; diagnostics use the suite's fixed policy and fresh
512 MiB offline calibration containers. No paid model is invoked.

`validation.json` binds the fixture/checker bytes and twenty fresh-container
calibrations. Full-parent and child Harbor receipts must additionally be present
before this case is considered prepared. Raw local evidence and runtime settings
live under the ignored `qwen38-suite/diagnostics/extract-elf` and
`qwen38-suite/preflight/extract-elf` result directories.

The verifier bootstrap is invoked only after the agent exits, using hash-pinned
existing uv 0.9.5, Python 3.13.7 and pytest 8.4.1 dependency caches. It preserves
the official test script and test assertions and cannot provide parent answers
to the agent. It changes apt transport to HTTPS and bounds dependency requests;
all dependency failures remain visible in the archived official verifier log.

Validation from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/extract-elf/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/extract-elf/case.json --run-fixtures
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/extract-elf/runtime.json --parent-id extract-elf --output evals/promptfoo/results/qwen38-suite/preflight/extract-elf
```

The runtime settings point to the verified generation-2 historical runtime;
the top-level old `runtime.json` is an identity report, not executable settings.

Verified preparation (2026-10-27): 27 unit tests passed; 20 fixture checks and
10 fresh Harbor trials passed. The official parent oracle executed two tests
and passed both (reward 1); nop executed two tests and failed both (reward 0).
All four diagnostic oracle/nop pairs returned 1/0 without recovery or agent
exceptions. Full preparation receipts were revalidated against current source,
adapter, runtime and settings with `verify_preparation`.

Original image: `alexgshaw/extract-elf@sha256:6932e4cb318464307eacd497ef8dc617eaf551b6a90231f815ec0b911895cfed`,
linux/amd64; compressed layers 347385449 bytes, uncompressed 1033833472 bytes.
All compressed SHA-256 values, unpacked diff_ids and local RootFS chain matched.
The actual restricted probe confirmed 1 CPU and 2147483648-byte memory limit;
storage_mb remains the official 10240, without claiming a writable-layer quota.

Clippy and `score.sh test` exited 0. Cargo dynamically claimed shared slot 1:
`CARGO_TARGET_DIR` and `CARGO_BUILD_BUILD_DIR` were both
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`, with one build
job. The owned lock was released; caches remain. No Rust behavior tests were
run. The existing `block 0.1.6` future-compatibility notice remains.

Ignored `diagnostics/extract-elf/acceptance.json` records exact commands, exit
codes, image/resource provenance, evidence hashes and test counts. `red.log`
retains the intentionally incomplete checker rejecting no negatives (23 tests,
28 failing subcases); the final checker passes all 27 tests. No official source
or shared evaluation configuration was edited and no paid model was called.
