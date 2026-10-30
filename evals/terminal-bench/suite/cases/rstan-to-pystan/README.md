# RStan to PyStan diagnostics

These four independent tasks isolate conversion behavior using synthetic data.
Full-parent readiness is blocked. The unchanged original image failed during
APT HTTP downloads; the separately authorized HTTPS transport variant reached
Launchpad PPA setup and failed on a truncated response. Each calibration retains
the original 4 CPU / 8192 MiB / 10240 MiB and 1800/1800-second limits.
No paid model requests are used for preparation.

- m1 parses named CSV features and targets, constructs design matrices, preserves
  empty test data, and rejects malformed headers, nonfinite cells and invalid
  metadata.
- m2 converts RStan iteration/thinning/control arguments and supplies a complete
  synthetic ARD Gaussian-process model. The evaluator compiles the supplied Stan
  source with real PyStan 3.10.0 and compares log-density differences against
  independent NumPy covariance/linear-algebra calculations at three parameter
  points. It checks parameter dimensions and that covariance matrices stay local.
- m3 validates parameter dimensions, positive finite scales and the explicit
  build seed, then constructs one initial-value object per chain. This narrow
  validator does not implement sampling or assess convergence.
- m4 computes component means from already retained draws and verifies five
  numeric-only CSV artifacts, including predictions, negative coefficients and
  an empty prediction matrix.

Each task uses a JSON stdin/stdout program at `/app/solution.py`. Public examples
are illustrative. Hidden probes execute candidate programs on other synthetic
inputs. Gold, nop and three semantic negative controls per task are separate
from the image and agent-visible fixtures. The extra-target-column regression
first exposed a missing checker probe, then passed after that probe was added.

The user explicitly authorized **2048 MiB** for this parent's four children on
2026-10-30 after genuine GCC and clang OOM failures at 512 MiB. Both fixture
execution and materialized standard tasks apply 1 CPU / 2048 MiB without swap;
agent/verifier timeouts remain 900/120 seconds, output/input budgets remain
32000/200000, and the policy retains one attempt, 24 turns and 32 tools for
`token-plan/qwen3.8-flash`. Other parents retain 512 MiB. The checker verifies
actual cgroup limits before evaluating candidates.

The real 2 GiB minimal normal model produced ten draws with a 1845981184-byte
memory peak. A separate three-observation, two-feature synthetic GP also
compiled and sampled, reaching 2147483648 bytes without an OOM kill. These
feasibility probes do not establish full-parent posterior accuracy or performance.

The final tools image is
`vw-rstan-pystan-diagnostics@sha256:ebaf0987df9c994c99dbe2b0980cb2f2bacc4ac34bc343cd227d683ba6de0ed6`.
It contains Python 3.12.12, PyStan 3.10.0/httpstan 4.13.0, NumPy 2.2.6,
setuptools 70.3.0 and Debian clang 14.0.6. `environment/dependencies.json`
locks the public interpreter archive, 23 wheels and nine Debian packages.
Debian metadata was authenticated by apt; package bodies were SHA-checked again
against that metadata. The image's manifest, config, all layer hashes/diff_ids
and architecture were independently read back. Its `/app` is empty, and it has
no parent inputs, tests, solutions, runtime, diagnostic gold or compiled model
cache. It runs offline. The Dockerfile documents the reconstruction context;
public dependency archives and OCI exports remain ignored, without a registry push.

The frozen x86 runtime's `--help`, Python/PyStan imports and clang executable
were checked in the final image with the runtime's own `LD_LIBRARY_PATH`.
Clang libraries are registered with the system loader, so the runtime's path
setting does not prevent tools from starting. The image build used the normal
constrained classic-builder command with 0.5 CPU / 2048 MiB and no network.

Standard trial startup now waits asynchronously for the shared Docker lock.
The wait is cancellable by Harbor's existing timeout, releases after errors,
and preserves the 4 CPU / 8 GiB admission limit. Tests cover event-loop progress,
cancellation, lock release and unchanged capacity/build policies. Both shared
implementation hashes changed; older validation/preparation receipts must be
refreshed before a later freeze. No other parent's assets or budgets were edited.

Run from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/rstan-to-pystan/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/rstan-to-pystan/case.json --run-fixtures
```

The historical root `qwen38-suite/runtime.json` is an evidence report and lacks
scheduler paths; the original supplied preflight command fails with
`KeyError: historical_root`. The ignored task-local `runtime-20261030.json`
provides absolute source/case paths, the generation-2 frozen runtime and the
unchanged parent timeout. Use it with a fresh output directory for regression.
The original root report remains unchanged.

Raw evidence is retained under the ignored
`evals/promptfoo/results/qwen38-suite/diagnostics/rstan-to-pystan/` directory:
original OOM/installation failures, real dependencies and compiler provenance,
both 2 GiB sampling probes, complete temporary-directory archives, build and
image receipts, failing/passing checker and lock tests, standard controls,
source verification, capacity observations and Clippy receipts. Keep the
numbered task until the original parent has valid calibration and a complete
preparation receipt. Infrastructure failures are unscored with null capability;
raw historical rewards are preserved separately.

Final validation on 2026-10-30: five case tests, twelve diagnostic-contract tests
and twelve build/resource tests passed. All twenty fixture controls and eight
fresh standard child controls matched their expected outcomes (oracle=1/nop=0
for each child), with unchanged source bytes and current execution identity.
Both required Clippy invocations exited 0; the final dynamic slot was 1, with
equal absolute target/build paths and JOBS=1. Static Clippy checks count as zero
behavior tests.

The first standard controls encountered an infrastructure lock error before the
agent. After the cancellable lock fix, all eight child controls passed at
execution identity `94dec6f0c90690b5c0ea348c8142a22b0cec1d62fa174df762f6dc93cb7653df`.
Original-parent startup initially lacked exclusive capacity. After capacity
became available, the original oracle actually ran: APT HTTP downloads failed,
`add-apt-repository` was missing, and all six official verifier tests ran
(one passed, five missing-output failures). Raw reward=0 is preserved as
invalid/unscored infrastructure calibration with null capability score.

## Authorized HTTPS APT variant

The user authorized `rstan-to-pystan-compat-https-apt` separately on 2026-10-30.
It changes exactly the two Ubuntu archive/security URI schemes from HTTP to
HTTPS. Signed-By, repository suites/components, pre-Agent installed packages,
official instructions/data/oracle/verifier, task budgets and timeouts remain intact.
It has `counts_as_original_official=false` and a separate standard variant
identity; it does not establish original-official success.

Pinned image:
`vw-rstan-https-apt@sha256:0da5996e1c9f25c72dfe7c3091fe285130108f4339ab28fb6eaf9962ca92d0fd`.
`variant/` binds the exact Dockerfile, original/target sources, the original
114-package inventory and contract. All original base layers and runtime
configuration semantics match; the added layer's only regular file is
`etc/apt/sources.list.d/ubuntu.sources`. Empty Rosetta directory metadata is
recorded separately. No dependency, Python interpreter, cached answer or oracle
artifact was preinstalled. The before-Agent proof independently checks the
actual source-file SHA and complete installed-package inventory.

All three HTTPS transport probes succeeded with complete bodies: Ubuntu main
InRelease, security InRelease and the previously failing
software-properties-common deb. Six new protocol regressions first failed on
the unsupported route, then all 43 new/existing protocol, variant, capacity and
contract tests passed on the actual shared implementation. The required Clippy
command exited 0 in dynamic slot 3 with equal absolute target/build paths and
JOBS=1; its owned lock was released with the cache retained.

The real standard variant oracle passed its before-Agent proof and successfully
installed software-properties-common, resolving the original HTTP blocker.
The unchanged `add-apt-repository ppa:deadsnakes/ppa` then failed inside Launchpad
response reading with `http.client.IncompleteRead(15 bytes read)`. The official
verifier again ran six tests (one passed, five missing-output failures).
Raw reward=0 remains preserved; the normalized calibration is
infrastructure/invalid/unscored with null capability score. No Stan sampling
ran. Preflight stopped before nop and the new child controls, so no current
variant preparation/readiness receipt exists. The prior eight child controls
must not be presented as a complete calibration under the new execution
identity `588690e423e955cab7b2c335d302678cae81598b2ec421946065c88f964ce16c`.

A separate read-only completeness probe parsed the full 2706949-byte Launchpad
WADL but timed out reading the team metadata. That is no evidence of network
recovery, so no calibration retry was started. Resume after complete Launchpad
responses are confirmed; use the approved variant runtime and a fresh preflight
output directory, retain the two-URI boundary and rerun all ten standard controls.
The numbered task retains exact paths and the latest failed worker state.

New raw receipts live under the ignored `https-apt-approved-20261030/` evidence
directory. A duplicate-start mistake overwrote the initial aggregate progress
log; the duplicate aborted at the existing-directory guard. The transport,
build, image, layer audit, protocol tests, Clippy and actual calibration logs
remain intact, with an explicit log-integrity note and preserved duplicate
error. The corrected resume log is separate. No original score data was lost.
