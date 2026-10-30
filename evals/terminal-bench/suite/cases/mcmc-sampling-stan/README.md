# Hierarchical sampling diagnostics and compatibility variants

Status: **blocked pending fresh controls**. On 2026-10-30 the user separately
authorized an independent variant which only adds `R_REMOTES_UPGRADE=never`
to the verified seven-package image. It preserves the original installer and
model. The new image has been built by the formal constrained helper and its
complete native OCI archive verified. Original filesystem layers are identical;
the only image Config change is that environment variable. Parent controls have
not started: the shared 4 CPU / 8 GiB window is occupied by other tasks. No parent
preparation receipt, ready subset, paid call or original-official success is claimed.

New variant: `mcmc-sampling-stan-compat-remotes-never`.
Image: `vw-mcmc-stan-remotes-never@sha256:dd7e3d1d90ed3b7b9e0510f5c2390bdf5fac692e4d306436f5847f4230cf4470`.
Config: `f31b29de166b99fe20325bc25ce6f590a9dd1412974f650eb5d287061afd8cd7`.
Exact compressed/uncompressed layer totals: 272028078 / 666092032 bytes.
`stan-variant/` binds its exact recipe, immutable base variant and new identity.
The metadata-only recipe has no RUN/COPY, and its base has no ONBUILD, so the
formal builder used the remaining 2 CPU / 4 GiB allocation; no executable build
container is required. Parent controls retain the full original 4 CPU / 8 GiB.

The old source log proves StanHeaders 2.32.10 installed before the original
installer automatically upgraded it to 2.39.1. The new policy prevents remotes
from upgrading installed dependencies, while still installing missing ones.
This is not a complete dependency lock and does not prove Stan compilation or
sampling succeeds. Four offline checks exercised the real archived remotes
policy functions with a locally mocked environment lookup; no R package was
preinstalled in either image.

The current diagnostic wrapper changed independently of this task. Its old
validation receipt was retained in ignored evidence, and all 20 fixtures were
rerun successfully under the new wrapper hash. A first formal child attempt
failed before Agent startup on the shared resource lock; it is retained as an
unscored infrastructure result, not a capability failure. Fresh current controls
must wait for the exclusive resource window, under a separate trial path. The
idle queue was stopped at this checkpoint; verify saved state before resuming.
Evidence: ignored `diagnostics/mcmc-sampling-stan/authorized-remotes-never/`,
including `variant-provenance.json`, `fixture-validation.log`, protocol and
integration test logs, the initial infrastructure trial, and queue state/owner.

The following historical seven-package-only failure remains unchanged.

The historical original-image failure remains separate: RcppParallel 6.2.1
required cmake >=3.5, missing from the original image/installer. Its oracle
exited 1; all six verifier tests ran (two passed, four failed). The raw reward
0 is retained, while calibration is infrastructure/invalid/unscored with null
capability score. The original task, data, oracle, verifier and 1800/1800-second
limits remain unchanged.

The user subsequently authorized only CMake 3.28.3-1build7, matching cmake-data
and five missing mandatory libraries. The real bodies of all seven packages
were downloaded from their locked Ubuntu URLs and SHA-256 read back. Metadata
was authenticated against Ubuntu's InRelease with the original image keyring.
`variant/` contains the five formal protocol files: contract, Dockerfile,
deb-lock, SHA256SUMS and complete original base package inventory. No original
installed package was upgraded in the image, and no R/RStan dependency was
preinstalled or source-cache material injected before Agent.

- Immutable original base: `alexgshaw/mcmc-sampling-stan@sha256:073fc36a3347b4f6b43bd6e98339542c41ed531c771af8bee40ba10f00cfabaf`.
- Authorized variant: `vw-mcmc-stan-cmake-compat@sha256:a6c991f2a0f6ba19abdccbd006bef08b40b9c380549d59aa29113c8a9df95559`.
- Manifest is genuinely single AMD64; config SHA is `379d89d5964c77d223c7f15b50efdd945470757c245019f98c6ba19e684be8cf`.
- Exact compressed/uncompressed layer totals: 272027750 / 666092032 bytes,
  fully stream-verified against compressed hashes and diff_ids. Original five
  base compressed hashes/sizes and RootFS prefix are preserved.
- Seven-package data tar total: 52746240 bytes, below the 512 MiB hard cap.
- Current execution identity: `b35499b54cd173f716f856c17ecb9baf971ef1326c5b4ebc4656adf9d95fb5df`.

The formal SuiteDockerEnvironment helper constrained build containers from the
first RUN to 4 CPU / 8192 MiB. Both parent controls used its standard fixed
before-Agent proof: CMake 3.28.3, the seven version/architecture pairs, and the
unchanged base inventory hash all matched. No private calibration hook or
platform/FROM/Trial/Compose/Builder override supplied that proof. Frozen x86
ELF `--help` ran offline in both the variant and fixed diagnostic image.

The fresh variant oracle successfully installed RStan 2.32.7, but the original
installation flow selected StanHeaders 2.39.1. Runtime reported Stan 2.39.0,
whose parser rejected the original model's old `int n[num_obs]` array syntax.
Oracle exit 1 and all six official tests (three passed, three failed) are
preserved. Nop ran all six (two passed, four failed), reward 0 as expected.
The variant raw oracle reward 0 is not a model capability score: its normalized
calibration status is infrastructure/invalid/unscored with null capability.
The seven-package permission did not cover R dependency or model changes.
The new permission described above covers only the remotes upgrade policy;
readiness still requires successful fresh controls.

Each diagnostic is independent and uses synthetic inputs with a Python standard
library JSON executable at `/app/solution.py`:

- m1: binomial CSV counts, column order, zero counts and invalid data.
- m2: normalized hierarchical Beta/Binomial log joint and an executable
  Metropolis transition under controlled proposal/uniform inputs.
- m3: actual four-chain seeded Metropolis sampling on a finite posterior grid,
  warmup removal and conventional unsplit Gelman–Rubin Rhat.
- m4: posterior means and linear-interpolated 5%/95% quantiles after warmup,
  including shape, finite-value and positive-domain validation.

These narrow kernels do not certify Stan compilation or full RStan posterior
sampling. The checker executes unseen synthetic probes in candidate processes;
it does not accept self-reports, keywords or a parent-solution prefix. M2 uses
independent direct density products; m3 reconstructs RNG transitions and sample
variances; m4 independently pools and interpolates quantiles.

The unchanged `validation.json` binds 20 checker-only controls: four gold,
four nop and twelve meaningful negative fixtures. All declared bytes, checker
implementation, image and policy were audited unchanged for reuse. Eight
**fresh current-identity** standard child controls passed (oracle=1/nop=0 each),
using the unchanged materialized FROM and no controller overrides. Main limits
were 1 CPU / 512 MiB, plus the formal 0.25 CPU /64 MiB egress sidecar.

Run from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/mcmc-sampling-stan/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/mcmc-sampling-stan/case.json --run-fixtures
```

Current raw evidence is ignored under
`evals/promptfoo/results/qwen38-suite/diagnostics/mcmc-sampling-stan/authorized-cmake/`:
package body/provenance receipts, native OCI archive, formal build observations,
current execution config, fresh child and parent controls, fixed before-Agent
proofs, complete official logs/CTRF, independent S10/B2/A1 capacity traces and
current Clippy receipt. The earlier original failure, unique original OCI
layers and 47 SHA-bound R source archives remain in the parent evidence folder.
Source archives were not injected into the Agent. Parent, diagnostic and Cargo
stages run serially; task SizeRw belongs to S10 GiB, extra R caches to B2 GiB,
receipts to A1 GiB, with 5 GiB safety margin. No scope or budget was increased.

The retained numbered task records the current blocker and exact current
Clippy outcome. Do not create a parent preparation receipt or count this
compatibility image as original-official success while gold remains invalid.


The required Clippy invocation exited 0 in dynamic slot 1 with identical absolute target/build
paths and JOBS1. Rust/Cargo inputs were equal before and after this invocation;
the owned lock was released and cache retained. This is separate from the older
Clippy receipt whose inputs drifted. Protocol checks (23), integration checks
(40), case unit checks (20) and fresh checker fixtures (20) passed; Promptfoo
configuration validation exited 0. None is a parent Stan sampling result.

An optional later Clippy refresh selected cold slot 3 and was stopped (exit -15);
its owned lock was released and cache retained. It is not a pass. The earlier
required pass is retained separately; no Rust code was changed by this task.
