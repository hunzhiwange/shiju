# feal-linear-cryptanalysis diagnostics

Four independent synthetic Python program tasks isolate unsigned 64-bit pair
parsing and half extraction (m1), mask parity and signed linear bias (m2),
finite candidate ranking with deterministic ties (m3), and candidate-key
Feistel decryption with both matching and mismatching known plaintexts (m4).
The byte cipher in m3/m4 is explicitly synthetic; it is not a parent answer
or a claim to reproduce the full FEAL attack.

Each task reads its declared input.json on stdin through /app/solution.py and
returns exact typed JSON on stdout. The checker runs the actual program,
rejects missing/malformed/duplicate-key output, booleans replacing integers,
symlinks and modified input. Independent specification tests check expected
values, including forward encryption of all m4 decryptions. Every task has
three distinct meaningful error programs and nop. No parent data, reference
solution, hidden tests, or model trace enters a diagnostic image.

The parent source remains read-only at tree
8f6b44ea1c65fc24042e0208292a7e9bcd72dd48. Its complete original task, tests,
resources (1 CPU, 2048MiB, 10240MiB storage) and 1800-second agent/verifier
limits remain authoritative. Diagnostics retain the suite policy: one attempt,
fresh environment, token-plan/qwen3.8-flash, output 32000 and input window
200000. No paid model is called during preparation.

validation.json binds all declared bytes and 20 fresh offline resource-limited
fixture runs. Full readiness additionally requires 10 fresh Harbor oracle/nop
trials with actual verifier test counts. Raw logs, runtime configuration, image
identity, resource and dependency receipts are kept under ignored
qwen38-suite/diagnostics/feal-linear-cryptanalysis and
qwen38-suite/preflight/feal-linear-cryptanalysis.

The task-local verifier bootstrap installs authentic hash-pinned cached uv,
Python and pytest dependencies only after the agent exits. Debian repositories
use HTTPS, with bounded dependency requests. It preserves the upstream test
script and assertions and leaves dependency failures visible. It supplies no
answers or tools to the agent phase.

Validation from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/feal-linear-cryptanalysis/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/feal-linear-cryptanalysis/case.json --run-fixtures
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/feal-linear-cryptanalysis/runtime.json --parent-id feal-linear-cryptanalysis --output evals/promptfoo/results/qwen38-suite/preflight/feal-linear-cryptanalysis/harbor
```

Verified preparation (2026-10-27): 26 unit tests, 20 fresh offline fixture
checks and 10 fresh Harbor trials passed. The complete parent oracle executed
one official test and passed; nop executed the same test and failed. All four
diagnostic oracle/nop pairs returned 1/0 without exceptions or recovery.
`verify_preparation` revalidated the complete receipt against current source,
runtime, adapter, scheduler, fixture and checker hashes.

Original image: `alexgshaw/feal-linear-cryptanalysis@sha256:ea78749a8422c6228d26c8753f45906a6a2df089906764b7e3c3e27e8f718ee7`,
linux/amd64. All 13 compressed layer hashes, unpacked diff_ids and local
RootFS chain matched. Compressed layers total
150821673 bytes; unpacked layer tar total
403315712 bytes. The restricted probe confirmed
1 CPU and 2147483648-byte memory limit. Official storage_mb remains 10240;
this does not claim a writable-layer disk quota. Parent image source hashes
match the frozen original and agent-visible /tests and /solution are absent.

The container reached Debian and PyPI over HTTPS. The online uv installer
endpoint failed; its result is retained and the verifier uses authentic pinned
cached tools instead. The original scoring script still executes unchanged.
An initial preflight's parent oracle/nop passed but diagnostic startup failed
because a task-driver-wide amd64 default conflicted with the native sidecar.
That failed attempt is retained in ignored harbor-platform-failure; removing
the driver override allowed all ten fresh final trials to pass. No shared
Harbor configuration, parent environment, scoring, timeout or budget changed.

Clippy and score.sh test exited 0. Cargo dynamically claimed shared slot 1,
with CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR both set to the repository's
absolute target/rust-slot-1 path and CARGO_BUILD_JOBS=1. The owned lock was
released and caches retained. No Rust behavior tests were run; the existing
block 0.1.6 future-compatibility notice remains. Ignored acceptance.json records
the absolute Cargo path, exact preflight command, exit codes, counts, image
and resource evidence. checker-red.log records a real failing regression for
input replacement by symlink; the final checker rejects it.
