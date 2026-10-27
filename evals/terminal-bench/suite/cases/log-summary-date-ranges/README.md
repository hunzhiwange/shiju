# Log summary date ranges diagnostics

Four independent synthetic artifact tasks write `/app/summary.csv`:

- m1 selects filenames across inclusive calendar ranges, including leap day,
  multiple sources, future files and the unconditional total.
- m2 aggregates the bracketed severity field, preserving duplicate records and
  ignoring message keywords and unrelated severity levels.
- m3 counts repeated records across leap-day, month and rolling-date boundaries.
- m4 normalizes precomputed counts into the required columns, ordering and zeros.

Each child has a frozen independent CSV oracle, an empty nop and three distinct
wrong artifacts. The checker parses the actual CSV and compares every cell,
including header, row order, zero counts and integer representation. Four
additional tests independently recompute expectations from public synthetic
inputs. No parent data, reference solution or scoring expectations enter child
fixtures. No paid model calls are authorized.

The red phase intentionally used an existence-only checker: 12 negative tests
failed among 20 controls. Exact CSV checking then passed all 24 tests. Original
raw logs and infrastructure failures are retained in the ignored directory
`evals/promptfoo/results/qwen38-suite/diagnostics/log-summary-date-ranges/`.

The diagnostic tool image is
`vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf`.
Its config digest is
`sha256:a6c16930afacbb61ef2e9f1e5f994d6c560b787fa4c5f849b5c15b3f017d111c`,
identical to the cached kv-store tool image. All RootFS layers, Config and amd64
architecture are equal. This manifest removes the OCI-index platform ambiguity;
no package or filesystem content changed. The image probe found `/app` empty
and `/tests`, `/solution`, `/oracle` absent. Standard materialization uses the
case's declared image. For this local image, reproducible builds use process
variables `DOCKER_BUILDKIT=0 COMPOSE_DOCKER_CLI_BUILD=1`, with
`DOCKER_DEFAULT_PLATFORM` unset. Do not inject private platform or CMD overrides.

The original parent remains the unchanged official task: reference date
2025-08-12, 164 generated log files, 1 CPU, 2 GiB, 10 GiB storage and original
900 second agent/verifier timeouts. Registry manifest:
`sha256:cbeb6ba905c2fec294f16cd5e16e3ea7f2e04d38ac2484d51a11de262aa7dc51`;
config:
`sha256:748e8485201e72133a9400dc8f6f524eb20e9812655eef41b32a1cc21796bdef`.
All original compressed SHA256 and uncompressed diff_id values were verified
from existing task25 cache: C=44,857,647, U=131,185,152 bytes. No duplicate
parent download or import was needed. Capacity evidence records host and VM
availability, original storage/tool/evidence reservations and runtime monitoring.

Inside the original image, apt and PyPI returned 200, while the fixed uv 0.9.5
installer URL returned 403. A verifier-only bootstrap supplies the SHA-bound
original installer, official uv release and real offline wheels after agent
completion. A separate original-image probe executed uv 0.9.5 and pytest 8.4.1.
Official test.sh, test_outputs.py, solution and reward semantics are unchanged;
no artifact replay or replacement checker is used. Dependency hashes and the
bootstrap policy are in the preparation receipt. The bootstrap shell is stored
in `environment/` for audit; diagnostic materialization never copies it to agents.

Focused checks:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/log-summary-date-ranges/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/log-summary-date-ranges/case.json --run-fixtures
evals/promptfoo/score.sh test
cargo clippy --all-targets --all-features -- -D warnings
```

The specified historical runtime preflight exits 1 (`historical_root` missing).
Current calibration uses the existing verified runtime-v2 through a task-local
runtime JSON, preserving historical evidence. Clippy and Promptfoo validation
exited 0; Clippy used dynamically acquired shared slot 1 with
`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`,
matching `CARGO_BUILD_BUILD_DIR` and `CARGO_BUILD_JOBS=1`. The lock was released
and cache retained. Clippy runs no behavior tests; no Rust code changed.

Final evidence: `validation.json` binds 20 fresh fixture controls under the new
single-platform image. All 24 focused tests passed. `preparation.json` binds
eight fresh child controls built from the exact standard materialized
Dockerfile (no added platform or ref substitution), plus the two original
parent controls. Child oracles scored 1 and nops scored 0. Original parent
oracle passed both official tests; nop failed both, scoring 0. All ten controls
had no exceptions, scoring replay or timeout. Children ran offline at 1 CPU /
512 MiB with ARM egress sidecars limited to 0.25 CPU / 64 MiB; parent receipts
prove 1 CPU / 2 GiB and actual amd64 layers. Only sidecar resource limits were
added to calibration Compose, with no main-service platform/CMD overrides.
Production `inputs` and `verify_preparation` accepted the combined receipt.
`final-gate.json` and the preflight directory retain the acceptance evidence;
no task containers remain. Original source hashes are unchanged.

Earlier exploratory controls with the old OCI index and modified calibration
FROM lines are excluded from the final receipt. The standard raw old image
failed both BuildKit and legacy default-platform probes; the final declared
single-platform image passed the standard legacy path. These infrastructure
failures are preserved separately and never counted as capability results.
