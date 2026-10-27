# distribution-search diagnostics

Four independent synthetic tasks exercise natural-log KL metrics (m1), a small
three-element target search (m2), zero/tiny probabilities (m3), and persisted
count/probability groups (m4). Each reads `/app/input.json` and writes
`/app/result.json`. No upstream solution, tests, data or model traces are public
fixtures. Hidden oracle files enter only the verifier phase. Inputs are immutable;
network access and privilege expansion are forbidden.

The standard-library checker parses actual artifacts and checks both KL values,
normalization, positivity, group counts and typed finite values. Search tasks
accept any satisfying distribution, including permutations, rather than matching
one frozen answer. m3 treats zero forward terms as zero and represents infinite
backward KL by the string `infinity`; tiny positive probabilities are not clipped.
Each child has gold/nop and three distinct meaningful negative controls.

The diagnostic image is the digest-pinned Python image in case.json, with no
additional dependencies. suite.diagnostics materializes only declared synthetic
fixtures into each fresh environment. Fixture calibration uses 1 CPU/512MiB,
no network, read-only root, all capabilities dropped, no-new-privileges and a
128-PID limit. The parent retains its official image, 1 CPU/2048MiB, storage
10240MiB and agent/verifier timeouts of 3600 seconds. Docker does not enforce
the declared writable-layer storage quota.

Verification from repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/distribution-search/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/distribution-search/case.json --run-fixtures
```

The historical runtime.json is an identity receipt lacking historical_root.
The ignored diagnostics/distribution-search/runtime.json uses the existing
verified generation-2 runtime and unchanged source/case roots, with original
parent timeout. Raw calibration lives under the ignored qwen38-suite/preflight
and diagnostics directories. validation.json binds the diagnostic image,
checker and synthetic fixture hashes; it alone does not establish parent readiness.

A deliberately weakened KL/normalization checker produced three test failures;
restoring the constraints passed all controls. No model calls were made.

Deterministic evidence: 25 focused tests, 20 fresh offline fixture controls,
and eight Harbor child oracle/nop controls passed. Clippy and
`evals/promptfoo/score.sh test` exited 0. Clippy dynamically acquired slot 1,
with both CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR set to
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`, jobs=1;
the owned lock was released and cache retained. No Rust source was changed.
Exact commands, exits and child summaries are archived in
`diagnostics/distribution-search/acceptance.json`.

## Parent image and verifier provenance

The official image is `alexgshaw/distribution-search:20251031`, manifest
`sha256:1448f7aa1c6080488251d00350e8e90477858989b1bec0accc866d7b3220738a`,
config `sha256:0c39e3eb6c2fb13618ed2a075343d52ab35dfef59a30e19c1173e534b9179b06`.
It is Linux amd64. After the original 600-second environment-start timeout,
its exact Registry manifest/config/layers were downloaded in resumable ranges;
all seven compressed SHA256 hashes and config rootfs diff_ids were verified.
Existing same-hash cache layers were checked before reuse. OCI archive metadata
was completed with the original platform before import; no image bytes changed.
The earlier failed startup and interrupted unscored verifier attempt remain
archived separately. Pre-pulling does not count as a successful Harbor startup.

`environment/verifier-bootstrap.sh` applies only after the agent exits. It
installs cached official uv 0.9.5 and a wheelhouse containing pytest 8.4.1,
pytest-json-ctrf 0.3.5 and NumPy 2.3.0. The original installer and release archive
are checksum bound by the execution configuration. NumPy's CPython 3.13
manylinux_2_28_x86_64 wheel has PyPI SHA256
`87717eb24d4a8a64683b7a4e91ace04e2f5c7c77872f823f02a94feee186168f`.
Dependency files, PyPI metadata and hashes are archived under this parent's
ignored `diagnostics/distribution-search/verifier-deps` and provenance files.
The wheelhouse remains outside the agent image and `/app`.

The bootstrap limits redundant apt calls to 30 seconds and curl to 60 seconds,
running the real commands and preserving nonzero/timeout exits. Original
repositories, official test.sh/test_outputs.py and reward computation are
unchanged. Offline uv resolves the authentic cached wheels, using the image's
Python 3.13. No scoring predicate, parent input, reference solution, model,
budget or official timeout is modified. Bootstrap logs distinguish dependency
setup from the four real official pytest tests.

Fresh preflight command:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/distribution-search/runtime-verifier.json --parent-id distribution-search --output evals/promptfoo/results/qwen38-suite/preflight/distribution-search-verifier-cache
```

Acceptance (2026-10-27): the fresh preflight exited 0 with all ten controls:
untouched parent and each child had oracle=1, nop=0. The parent oracle passed
all four official pytest tests; nop failed for the missing distribution.
`verify_preparation` accepted the complete current source/runtime/scoring receipt.
The earlier 25 focused tests, 20 offline fixture controls and eight standalone
child controls were rechecked for byte drift; focused tests and offline controls
were rerun successfully. Clippy reran with slot 1 and exited 0; the owned lock
was released. Promptfoo validation also reran with exit 0. Complete raw evidence
and exact commands/exits live in `diagnostics/distribution-search/acceptance.json`.
No model requests were made. Next: include the parent and its four independently
scored children in the full-suite freeze, using the calibrated execution settings.
