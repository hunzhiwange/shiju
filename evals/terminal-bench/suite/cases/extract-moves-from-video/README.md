# extract-moves-from-video diagnostic preparation

Four independent synthetic tasks isolate terminal-frame transcription (m1),
submitted input reconstruction from changing buffers (m2), repeated viewport
capture and wrapped-row merging (m3), and chronological LF command-file output
(m4). Each has a complete public instruction, independent expected output,
three semantic negative artifacts and nop. Identical commands from different
input actions must survive reconstruction. No parent video, answer, solution
prefix or model trace enters a diagnostic environment.

m1 uses a locally rendered PNG, visually reviewed against its gold transcript.
m2 and m3 provide structured synthetic frame observations to isolate temporal
reconstruction from OCR. m4 receives unsorted transcribed events, including a
timestamp tie. JSON outputs permit formatting variation; text output requires
exact UTF-8/LF bytes. The checker independently reads the output, verifies input
hashes and rejects symlinks, malformed JSON, omissions, additions, wrong order,
false deduplication and incorrect formatting. The unit suite independently
reconstructs gold from the public contracts. An existence-only checker first
failed 16 of 26 tests, then the strict checker passed all 26.

The official parent remains unchanged at source tree
`c415bb6b41b2b2aa544907a5c482134ac1086fc5`, including its video, reference
solution, two tests, 90% character-level similarity threshold, 1 CPU, 2048 MiB,
10240 MiB storage, and 1800-second agent/verifier limits. Its image is
`alexgshaw/extract-moves-from-video@sha256:74ef87375ce2b6332ca9fe0180eb541b64cf4ef4188edd5b1be22c272e5a5b4b`.
The diagnostic image is digest-pinned in case.json. Fixture calibration uses
fresh offline containers with 1 CPU, 512 MiB, read-only root, no capabilities,
no-new-privileges and 128 processes. Only declared synthetic inputs enter each
image; hidden checkers, oracles and gold are injected in their proper phases.

`environment/verifier-bootstrap.sh` provisions authentic checksum-pinned uv
0.9.5, Python 3.13.7, its SQLite library, curl and pytest dependencies only
after the agent exits.
It preserves official test.sh and test_outputs.py, uses HTTPS for the same
signed apt repositories and bounds redundant dependency network requests.
Dependencies and local executable settings are stored in ignored per-case
results. The old root runtime.json is an identity report; the executable
settings use the verified generation-2 historical runtime.

Run from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/extract-moves-from-video/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/extract-moves-from-video/case.json --run-fixtures
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/extract-moves-from-video/runtime.json --parent-id extract-moves-from-video --output evals/promptfoo/results/qwen38-suite/preflight/extract-moves-from-video
```

Preflight requires a fresh output directory. Full-parent and four-child
oracle/nop Harbor evidence is required in addition to the 20 fixture checks
bound by validation.json. Calibration evidence, Registry manifests, compressed
layer hashes, diff IDs, dependency hashes, isolation/resource observations,
network probes and command results live in ignored per-case diagnostics and
preflight result directories. Docker retains the official storage declaration
but does not enforce a writable-layer disk quota. No paid model is invoked;
model, token limits and one-attempt policy stay frozen.

Final evidence: the original parent oracle passed both official tests; nop
failed both. All ten fresh Harbor controls matched their expected rewards,
without trial exceptions. The preparation receipt was independently verified.
All 26 unit tests (52 checker subprocesses), 20 fresh fixture controls, and
Promptfoo configuration validation passed. Required Clippy exited 0 in dynamic
slot 1, with CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR set to
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1` and one build
job; the owned lock was released and cache retained. The existing block 0.1.6
future-compatibility notice remains. Exact commands, exits, hashes and resource
receipts are in ignored `diagnostics/extract-moves-from-video/acceptance.json`.
Next: include the prepared case in the suite freeze before separately
requested paid evaluation.
