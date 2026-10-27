# build-cython-ext diagnostics

Four independent synthetic tasks cover extension/array-interface inspection,
a NumPy 2 typed-array repair, building/importing three native modules, and
small numerical comparisons. They contain no official source, hidden test,
reference solution, old model output, or credentials.

The explicit case maps isolate public source fixtures from checker, oracle,
gold and negative mutations. m1 requires a complete structured interface
report. m2 independently compiles the submitted Cython source and probes
64-bit sums, empty arrays, strided arrays and float rejection. m3 runs the
submitted build program, then requires three real ExtensionFileLoaders and
calls each extension. m4 compiles the submitted source and compares signed,
fractional and empty calculations against independent formulas. A separate
interpreter performs each native probe; stdout and agent-authored tests are
never used as proof. NumPy must remain exactly 2.3.0.

Each child includes three meaningful negative implementations and a nop.
The initial permissive checker demonstrably accepted nop and all three
incorrect m1 reports; the regression test failed four subcases before the
structured artifact check was implemented. All remaining fixtures exercise
real compiler/import/runtime behavior in fresh containers.

The native ARM64 diagnostic image derives from pinned clean Python 3.13.7
and adds gcc, libc6-dev, NumPy 2.3.0, Cython 3.1.3 and setuptools 80.9.0,
before any task fixtures or hidden material exist. Native architecture avoids
changing Harbor egress-sidecar platform settings. Its Dockerfile records the recipe; case.json records its immutable
manifest identity. No repository or parent package enters the image.
Fixture checks use offline fresh containers, 1 CPU, 512MiB, dropped capabilities,
a read-only rootfs and read-only oracle/checker mounts. Harbor injects the
checker and gold only during their respective phases. The official parent
package and scorer are unchanged and retain their original resource limits.

Diagnostic validation commands (repository root):

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/build-cython-ext/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/build-cython-ext/case.json --run-fixtures
```

Raw preparation, negative calibration,
image provenance, network probes and TDD evidence live under the ignored
`evals/promptfoo/results/qwen38-suite/diagnostics/build-cython-ext/` directory.
`validation.json` binds declared files and checker bytes; machine-bound
`preparation.json` remains ignored. No paid model request is authorized here.

## Authorized compatibility variant

The original official environment remains unready: two unchanged oracle trials
returned reward=0 (10/11 official tests passed), because dynamically resolved
planarity 1.0.0 renamed the graph drawing attributes used by pyknotid 0.5.3.
The original nop returned 0. Those failures remain archived and are never
replaced or reported as model scores.

The user explicitly authorized `build-cython-ext-compat-planarity06` as a
separate compatibility variant. `variant/contract.json` marks
`counts_as_original_official=false` and pins its own image manifest, original
base image, source archive, built wheel, constraints and Dockerfile hashes.
`variant/Dockerfile` installs the authentic planarity 0.6 wheel offline while
building the image, before any agent or oracle starts. The original base image
identity is retained as provenance; the derived image has a separate identity.
`PIP_CONSTRAINT=/opt/vw-compat/constraints.txt` keeps ordinary pip operations
on 0.6. Agent-start/end observations require the exact constraint hash,
planarity 0.6 and NumPy 2.3.0; a changed contract fails calibration.

The compatibility package is a byte-for-byte copy of the frozen full official
package except for two explicit task.toml fields: the task name is labeled as
the variant and docker_image selects its pinned pre-agent image. The original
prompt, repository-clone instruction, solution, tests, expected values,
CPU/memory and all timeouts are preserved. No dependency is injected at
verification time, and no reference material enters the image. The four
synthetic diagnostics retain their independent clean native image.

The wheel was built offline from the unmodified official PyPI 0.6 sdist,
SHA256 `9852691d9c0d05e26a2fcbf2fce5a2632e8847ec462e816fa02e72d35013da68`,
using the pinned amd64 Cython 3.1.3/setuptools 80.9.0 build image recorded in
the contract. Wheel SHA256 is
`094b3f49d3226e903055ceb8770065fdf2a55404414c500d9530225057855ff6`.
The public recipe contains no wheel binary, upstream source, credentials,
or hidden grading material. To rebuild, place the verified wheel and the two
recipe files in an isolated build context; build with platform linux/amd64.
An exact new manifest identity must be reviewed if rebuilding changes bytes.

Calibrate this explicitly labeled variant and four fresh diagnostic pairs:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python evals/terminal-bench/suite/cases/build-cython-ext/variant.py --official evals/promptfoo/results/qwen38-suite/source/tasks/build-cython-ext --wheel evals/promptfoo/results/qwen38-suite/diagnostics/build-cython-ext/compat-wheels/planarity-0.6-cp313-cp313-linux_x86_64.whl --output evals/promptfoo/results/qwen38-suite/diagnostics/build-cython-ext/authorized-variant/new-calibration
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/build-cython-ext/tests -p 'test_*.py'
```

This replaces the parent's standard preflight only for the authorized variant.
It writes `variant-calibration.json` in a separate output directory, never the
normal official `preparation.json`. The shared official-suite freeze cannot
mistake this receipt for original-task readiness. Variant results must remain
separate from the 89 original official task scores. No paid model is called.
The earlier verifier-time replacement proposal remains rejected.

Validated on 2026-10-27 under this authorized contract: the compatibility
parent passed all 11 unmodified official tests, with oracle=1/nop=0. Four
fresh diagnostic pairs also returned oracle=1/nop=0 (10 Harbor trials total).
The fresh fixture CLI passed all 20 controls (4 gold, 4 nop, 12 negative
implementations); five unit tests passed. Promptfoo validation and Python
syntax checks passed. Clippy exited 0 using dynamically claimed slot 1,
`target/rust-slot-1`, with matching CARGO_BUILD_BUILD_DIR and jobs=1; its
owned lock was released and cache retained. No Rust behavior tests were
executed by Clippy. The existing block 0.1.6 future-compatibility notice remains.

The final local acceptance audit is
`evals/promptfoo/results/qwen38-suite/diagnostics/build-cython-ext/authorized-variant/acceptance.json`.
It binds runner, image, package, fixture and scorer receipts, four pre/post
parent environment observations and 18 resource records. Main containers kept
original limits (parent 1 CPU/2048MiB; diagnostics 1 CPU/512MiB); the eight
native Harbor egress sidecars were capped at 0.25 CPU/128MiB without changing
network policy. Preparation must apply the shared capacity policy, including
those sidecar caps, when repeating these commands. Original failures and
canceled/rejected experiments remain separate and unchanged. No normal
original-task preparation receipt or paid-model score was produced.
