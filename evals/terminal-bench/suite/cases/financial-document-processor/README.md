# financial-document-processor diagnostics

Four standalone synthetic UTF-8 transcription tasks isolate classification
(m1), gross/tax extraction (m2), Total/Amount Due conflicts and absent VAT
(m3), and conserved file moves with exact CSV reconciliation (m4). OCR is
outside these narrow diagnostics and remains part of the complete parent.
Every child implements solution.py; the independent checker executes the
actual program and checks its artifacts, exact document coverage and hashes.
It rejects symlinks, duplicate JSON keys, nonfinite amounts, incorrect invoice
rows, copy-instead-of-move, incorrect destinations and aggregate mismatches.
Each child includes a gold program, nop and three distinct incorrect programs.
Only declared synthetic inputs enter a fresh child image. No parent solution,
hidden test, source document or model trace is included in child fixtures.

Original parent tree: f461579cc917b5e02fb466d90ad8a63b208c4b01.
The unchanged parent retains all 17 JPG/PDF files, official OCR solution and
seven official tests, 1 CPU, 4096 MiB memory, 10240 MiB storage, and
1200-second agent/verifier limits. Diagnostic policy remains one attempt,
token-plan/qwen3.8-flash, 32000 output and 200000 input tokens. No paid model
is invoked. Readiness requires parent and all child oracle/nop calibration.

Local evidence lives in ignored qwen38-suite/diagnostics/financial-document-processor
and preflight/financial-document-processor-recovery-20261028-02 directories. validation.json binds
all declared fixture/checker bytes and twenty fresh offline Docker checks.
The task-local runtime uses verified generation-2 runtime-v2, not the obsolete
identity-report runtime.json. Verifier bootstrap provisions authentic pinned
cached dependencies only after the agent; it preserves the original tests
and leaves dependency errors visible. It supplies no agent-side tools/answers.

Validation from repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/financial-document-processor/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/financial-document-processor/case.json --run-fixtures
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/financial-document-processor/runtime.json --parent-id financial-document-processor --output evals/promptfoo/results/qwen38-suite/preflight/financial-document-processor-recovery-20261028-02
```

Verified restoration (2026-10-28): 30 unit tests, 20 fresh offline fixture
controls and all 10 fresh Harbor oracle/nop trials passed. The original parent
oracle executed seven official tests and passed all seven; nop executed the
same seven and failed all seven. No tests were skipped, no Harbor exceptions
occurred, and no paid model was invoked. These are calibration controls, not
model capability scores. preparation.json was revalidated against current
source, runtime, adapter, scheduler, bootstrap and fixture hashes.

Original image: alexgshaw/financial-document-processor at
sha256:ef6c9cfaaf14cdd200163008a188d5baa9f626f3bfb8d3e7d6f30c08518e6251,
linux/amd64. Four compressed layers total 51790066 bytes; unpacked tar bytes
sum to 130614272. All compressed hashes, diff_ids and the loaded RootFS chain
matched. All 17 image document hashes matched the frozen original source.
Restricted startup confirmed 1 CPU and 4294967296-byte memory limit. Official
storage_mb remains 10240, without claiming writable-layer quota enforcement.

The original agent installed Tesseract, its system certificates, Python 3.13.7
and its exact Python dependencies itself, within the unchanged 1200 seconds.
No dependency cache, audit tool, bootstrap or altered reference script entered
the agent phase. Before calibration, a throwaway original-image audit completed
the entire 35.1 MB apt dependency download and the 30.8 MiB Python download;
its caches were discarded. Host and VM capacity exceeded the conservative
20.6 GB peak admission estimate.

A subsequent verifier-only failure exposed the minimal image's missing
libsqlite3.so.0. The bootstrap now copies the existing hash-pinned runtime
SQLite library after extracting the verifier interpreter, before validating
imports. The same bootstrap passed independently in a fresh original image,
including pandas/pytest/SQLite imports, then passed complete parent oracle/nop
calibration. The official checker and expected values remain unchanged.
The historical agent timeout and failed verifier bootstrap remain archived
separately as unscored/infrastructure; neither is an accepted calibration.

Clippy and score.sh test exited 0. Cargo dynamically acquired slot 1, with
CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR both set to
/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1 and
CARGO_BUILD_JOBS=1. No Rust behavior tests were run. The owned lock and all
owned audit/calibration containers and networks were released; caches remain.

Current command/exit-code, dependency, test-count, capacity and cleanup
receipts are in diagnostics/financial-document-processor/recovery-20261028;
acceptance.json binds the final asset and evidence hashes. Future reruns must
select a fresh preflight output directory. Include this parent and its four
independently scored diagnostic cases in the later suite freeze.
