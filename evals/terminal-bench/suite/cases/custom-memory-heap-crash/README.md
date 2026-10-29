# custom-memory-heap-crash diagnostics

## Paused preparation

Task 27 was cancelled by the maintainer on 2026-10-29. The existing assets are
retained as drafts, not as ready suite cases. Historical focused tests and
offline fixture controls below do not establish full parent or Harbor child
calibration. No `preparation.json` is present. Resume only on an explicit new
request and obtain fresh readiness evidence before inclusion in an experiment.

Four independent synthetic tasks cover release-only allocator failures. No
upstream source, hidden tests, reference answer or model trace is included in
public fixtures. The official parent remains unchanged: 1 CPU, 2048MiB RAM,
10240MiB storage, 1800 seconds each for agent and verifier.

| Child | Required artifact or behavior | Negative controls |
| --- | --- | --- |
| m1 | Report independently reproduced debug exit and release abort | missing release crash, debug crash, wrong signal |
| m2 | Identify allocation/owner destruction/release event IDs | no violation, wrong owner, wrong allocation |
| m3 | Repair only user.cpp; both builds and normal finalization pass Valgrind | late initialization, early exit, unchanged bug |
| m4 | Repair hooks while preserving protected source bytes | protected edit, late initialization, early exit |

The small C++ allocator model is intentionally synthetic. Release cache storage
belongs to an arena, debug storage uses malloc, and a static finalizer rejects
arena storage after shutdown. It does not claim to reproduce the upstream
patched libstdc++ ABI. The checker compiles and runs real binaries with -O0 and
-O2 -DNDEBUG, checks work and finalization output, and independently runs
Valgrind. A link-time guard rejects direct exit/_exit/_Exit/quick_exit calls,
including candidates that forge the expected finalizer output. The event diagnosis is checked as a complete typed JSON artifact,
not a prose or keyword match. Immutable inputs are SHA256-bound; missing,
symlinked, duplicate-key, oversized and structurally incorrect results fail.

Only public fixtures enter diagnostic environments. Tool-only image sources
are in environment/Dockerfile. Hidden checkers and gold outputs enter only the
verifier and oracle phases. The base digest is architecture-specific (arm64);
the official upstream image is amd64 and must pass a separate emulated runtime
probe. Offline diagnostic controls use fresh containers with 1 CPU, 512MiB,
no network, dropped capabilities and a read-only root filesystem. Harbor child
calibration uses the suite's independent fresh environment policy.

The supplied historical runtime.json is an identity receipt rather than an
execution configuration. The per-case ignored runtime uses the verified v2
runtime and authentic cached uv 0.9.5 / pytest 8.4.1 / pytest-json-ctrf 0.3.5,
installed only after agent execution. environment/verifier-bootstrap.sh is not
copied into the diagnostic tool image. No official scorer or expected result
is changed. Storage is declared, not an enforced writable-layer quota.

Commands from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/custom-memory-heap-crash/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/custom-memory-heap-crash/case.json --run-fixtures
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/custom-memory-heap-crash/runtime.json --parent-id custom-memory-heap-crash --output evals/promptfoo/results/qwen38-suite/preflight/custom-memory-heap-crash-valid-runtime
```

Raw build, network, identity, isolation, resource and calibration evidence stays
under the ignored diagnostics/custom-memory-heap-crash directory. Readiness
requires parent oracle=1/nop=0 plus four independently calibrated children;
compilation or transport completion alone does not establish readiness.

Deterministic verification: 4 focused tests passed, including 20 gold/nop/negative
controls in fresh offline containers. An always-accepting checker produced 17
failures; an additional runtime header-mutation control failed before the
post-execution integrity check was added. Tool image: `vw-memory-diagnostics-native@sha256:b3550c423859c800161647c91b9749156a1bfbb1553f8980936387536c427dfd`.
Clippy and Promptfoo configuration validation exited 0. Cargo dynamically
claimed slot 1, with CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR both
the absolute repository-local `target/rust-slot-1` path and
CARGO_BUILD_JOBS=1; its owned lock was released and cache retained. No Rust
files changed and no Rust behavior tests were run.

A stronger forged-finalizer early-exit fixture exposed two further failures
before the independent link-time termination probes were added; the final
checker passes all four tests and the same 20 distinct fixture controls.
