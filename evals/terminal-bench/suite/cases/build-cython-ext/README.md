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

Production uses the standard suite entry points with an explicit
`environment_variant` setting on the `build-cython-ext` runtime task. Its
minimal descriptor is `{ "id": "build-cython-ext-compat-planarity06",
"contract_sha256": "<SHA256 of variant/contract.json>" }`. No other parent
is authorized. The descriptor resolves to the pinned image and hashes of all
three variant definition files. Standard preflight creates a separate copied
parent package and retains all original task bytes except task.name/image.
SuiteDockerEnvironment selects that image and checks dependencies before
returning from start, before any agent setup/execution. No private calibration
hook, platform override or verifier-time dependency substitution is required.

After capacity admission, use a fresh output directory:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/build-cython-ext/production-wiring-20261028/runtime.json --parent-id build-cython-ext --output evals/promptfoo/results/qwen38-suite/preflight/build-cython-ext/standard-variant-next
```

Standard preparation now binds the pre-agent observation and requires
SuiteDockerEnvironment evidence. Freeze carries the descriptor and source
hashes into a separate parent package; the worker forwards that same descriptor.
The parent retains its logical case ID for grouping but has
`kind=compatibility_variant` and explicit variant identity in formal records.
Reporting keeps this layer separate and does not satisfy the original 89-parent
coverage gate with variant scores. Missing/drifting definitions, evidence,
labels or official source bytes fail closed. The historical standalone
`variant.py` and its receipts below are archival evidence, not production
readiness or a substitute for the shared standard calibration.

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

2026-10-28 production wiring validation: 44 mock/file-only focused tests passed,
including standard preflight/freeze/manifest routing, model-worker forwarding,
shared backend startup checks, result labeling/report separation and negative
controls. Required Clippy passed in shared slot 1 with matching target/build
paths and jobs=1; its lock was released. No paid model was run.

Fresh standard calibration completed after a new two-sided capacity admission:
host free 21717889024 bytes, Docker VM free 94007705600 bytes; the sequential
parent peak budget was 20178719232 bytes and diagnostic budget 10111803904.
20 fresh fixture controls passed. Standard `suite.run preflight` then passed
both parent controls (oracle=1 with 11/11 official tests, nop=0) and eight
fresh diagnostic controls. Every trial used SuiteDockerEnvironment; the two
parent receipts bind its pre-agent image/dependency proof. Frozen generation-2
ELF `shiju --help` also passed in the compatibility image. The standard
preparation receipt passes the current validator. No paid worker or global
89-parent freeze/final regression was executed.

The mock/file-only checks comprise 44 focused tests plus 12 record-contract
tests; a 21-test routing/report subset was repeated after the final report
check. All passed. One earlier broad test invocation unintentionally included
real tiny-container tests; it was canceled with exit 130 and is not counted
as passing evidence. All three exact container IDs were confirmed destroyed;
raw interruption and event records are retained. No unrelated resources were
removed. Local acceptance, capacity, raw standard trials and identity impact
are under `results/qwen38-suite/diagnostics/build-cython-ext/production-wiring-20261028/`.
Old successful standalone receipts were preserved byte-for-byte and were not
reused as standard evidence.

Changing shared scheduler/record/backend bytes invalidates old frozen
identities; the current snapshot identifies two newly stale preparations
(openssl-selfsigned-cert and path-tracing-reverse) and 34 already-stale ones.
They must be revalidated under scheduling authority, never hash-relabelled.
