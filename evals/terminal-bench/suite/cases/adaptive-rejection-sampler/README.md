# Adaptive rejection sampler diagnostics

Four independent base-R tasks use synthetic inputs and executable assertions:

| Child | Capability | Incorrect controls |
| --- | --- | --- |
| m1 | Validate arguments, density values and finite-point log concavity | Accept all, reject valid input, omit concavity check |
| m2 | Tangent intersections and lower chord geometry | Wrong intersections, intercept sign, tangent used as chord |
| m3 | Lazy squeeze and stable log-space rejection | Always accept, underflow comparison, eager density evaluation |
| m4 | Bounded rejection sampling API and empirical distribution | Uniform substitute, wrong count, normal substitute |

Each prompt specifies `/app/answer.R`, its R API, input contract and independent
success conditions. m4 isolates proposal acceptance under a supplied constant
bound; it does not ask for the parent's full adaptive sampler. The fixed-seed
CDF/moment checks cover triangular, truncated exponential and shifted targets.
No upstream reference implementation or model trajectory is used as a fixture.

`check.py` executes the candidate R program with hidden assertions and requires
the harness to finish. Missing artifacts, assertion failures, early exits and
timeouts reject the candidate. Self-reported test output is not scored.

The diagnostic image is built by `environment/Dockerfile` from a pinned native
ARM64 Ubuntu manifest with base R and Python. The local image reference
includes a tag as a BuildKit lookup hint and retains the immutable digest; a
wrong-digest control is rejected. It contains no parent task files,
reference solution or encrypted archive. Each fixture control runs in a fresh,
offline 1-CPU/512-MiB container with a read-only root, 128 PIDs, no capabilities
and no-new-privileges. Only declared public fixtures enter `/app`; hidden
assertions are mounted read-only at verification time. Harbor oracle/nop runs
also use fresh containers and retain the no-network egress sidecar.

The original parent package, resources and 900-second agent/verifier timeouts
remain unchanged. Its official verifier requires uv 0.9.5 and Python 3.13.
`environment/verifier-bootstrap.sh` extracts only dependency caches under
`/opt/vw-verifier-deps` after the agent exits. The execution configuration binds
all dependency bytes and this script by SHA-256. The unmodified official uv
installer uses its supported `UV_DOWNLOAD_URL=file://...` to consume the exact
official release archive. Bootstrap executes the unchanged, GitHub-hash-verified
installer before the official test script, using a private real-curl dependency
closure. Installed uv/uvx binaries are SHA-256 checked. This also covers an
`astral.sh` TLS failure during the test script without replacing that script. Python downloads are disabled; the pinned 3.13.7
interpreter cache is used. Hash-pinned native libraries and the complete PyPI
wheel set are extracted into the same verifier-only prefix. `UV_OFFLINE` and
`UV_FIND_LINKS` keep the official uvx dependency resolution offline. The official test script and pytest scorer execute
normally. No installer, reward or task artifact is fabricated or replaced.

Local execution configuration and raw evidence are retained at
`evals/promptfoo/results/qwen38-suite/diagnostics/adaptive-rejection-sampler/`.
The original failed calibration remains at
`evals/promptfoo/results/qwen38-suite/preflight/adaptive-rejection-sampler/initial/`.
The task-4 `runtime.json` identity receipt is preserved; use this case's
`execution-bootstrap.json` for preflight.

From the repository root:

```bash
export PYTHONPATH=evals/terminal-bench
PYTHON=evals/promptfoo/results/task25/harbor-venv/bin/python
"$PYTHON" -m unittest discover -s evals/terminal-bench/suite/cases/adaptive-rejection-sampler/tests -p test_cases.py
"$PYTHON" -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/adaptive-rejection-sampler/case.json --run-fixtures
"$PYTHON" -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/adaptive-rejection-sampler/execution-bootstrap.json --parent-id adaptive-rejection-sampler --output /absolute/new/preflight-output
```

`validation.json` binds 20 fixture controls and their declared bytes.
`preparation.json`, when present, binds all ten real Harbor oracle/nop outcomes,
the unchanged official package, generation-2 runtime and verifier bootstrap.
A zero reward is accepted as a parent nop control only when its CTRF proves
all nine official tests ran and failed; missing pytest/CTRF is infrastructure
failure. Earlier raw failures remain outside this case directory.
These are calibration results, not model benchmark scores. Model requests,
output/input budgets and attempt limits remain those in the frozen suite policy.

## Verified calibration

Twenty executable fixture controls passed (4 gold, 4 nop and 12 mutations).
All ten fresh Harbor controls passed: original oracle reward 1 with 9/9 official
pytest tests passing, original nop reward 0 with 9/9 official tests failing,
and each of m1..m4 oracle=1/nop=0. No model was called.

The final source snapshot and current-only collector are preserved under
`diagnostics/adaptive-rejection-sampler/execution-source-final/` and
`diagnostics/adaptive-rejection-sampler/calibrate-installed.py`. Raw execution
evidence is in `preflight/adaptive-rejection-sampler/installed-controls/`.
The immutable execution receipt remains there. A final blank-line cleanup in
one negative fixture was recalibrated with 20 fresh controls; the actual
executed Harbor package bytes were proven identical. The final fixture receipt
at `diagnostics/adaptive-rejection-sampler/fixture-normalization/preparation.json`
matches this directory's receipt. Original trial results, timestamps and
execution identities remain unchanged; `fixture-normalization/provenance.json`
records the one-file change and both package trees. `acceptance.json` records
source, dependency, fixture, checker and scoring evidence hashes.

Required Clippy exited 0, with 0 Rust behavior tests. The stage dynamically
claimed shared slot 1 at the repository's `target/rust-slot-1`, using the same
absolute directory for Cargo target/build and one build job. The lock was
released and cache retained. Twenty R behavior controls and real official
pytest/Harbor scoring provide the behavior evidence. Earlier network failures,
APT timeout, missing installer/CTRF and failed image-resolution probes remain
archived; none was counted as a valid zero-score control.
