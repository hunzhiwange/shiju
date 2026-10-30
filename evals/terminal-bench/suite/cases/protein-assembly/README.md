# Protein assembly diagnostics

Four independent synthetic tasks cover FASTA parsing and exact spectral
constraints (m1), allowed-domain selection by spectral axis and ligand (m2),
GS linker and every sliding GC window validation (m3), and delivery of a
complete coding sequence (m4). The invented peptides do not represent
functional proteins. Official sequences, hidden tests and reference solutions
remain in the ignored upstream package.

Each prompt states its input, output, no-network boundary and tool/time budget.
m1–m3 execute submitted Python programs with `-I` against checker-owned probes.
m4 translates the actual DNA, accepts synonymous codons and any valid GS
linkers, and checks every 50-base window. Submitted claims and self-tests are
not grading evidence. Each child has gold, nop and three semantic negatives.

The generic image is the reviewed cached amd64 single manifest
`vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf`.
Its `/app` is empty and it has no upstream, solution, tests or oracle directory.
The frozen generation-2 Agent, SQLite sidecars and native budget hook execute
in both this image and the original parent image. No platform override is used.
Materialization copies only synthetic public fixtures; solution and checker
files enter Harbor separately.

The initial permissive checker produced 17 failed assertions. Fresh container
calibration later exposed another defect: from `/app`, Python loaded the
candidate `select.py` while importing stdlib `select`. The checker now removes
the candidate working directory from its import search path before loading its
libraries. Candidate programs still run with `-I`. A fresh-process regression
adds a shadow `json.py` so the failure reproduces on macOS, where `select` is
built in. The regression failed before the fix and passes after it.

Current deterministic evidence: 4 unit tests pass, including 20 process controls,
synonymous coding, linker bounds, malformed/frame/domain/terminal rejection,
and import isolation. The exact diagnostics CLI also passes all 20 fresh
bounded offline containers: 4 gold acceptances and 16 nop/semantic rejections.
Gold probes cover 6 FASTA, 8 selection and 13 composition inputs.

All four children additionally pass fresh standard Harbor oracle/nop pairs
through `SuiteDockerEnvironment`: oracle rewards are 1 and nop rewards are 0.
The final serial window receipts and actual runtime inspections are under
`preflight/protein-assembly-children-windowed-20261031/` and the diagnostic
evidence directory. These eight Trials preserve the 1 CPU / 512MiB child
budgets and bounded egress sidecars. The original parent nop also scores 0.

Fresh `cargo clippy --all-targets --all-features -- -D warnings` passes using
dynamically claimed slot 3, matching absolute target/build directories and
`CARGO_BUILD_JOBS=1`; its owned lock was released and cache retained. An earlier
slot-2 run was interrupted by external removal of its temporary directory and
lock record; its original failure log remains archived. No Rust source changed.

Full-parent readiness remains blocked. All 11 official files retain their
catalog hashes. The original amd64 image is
`alexgshaw/protein-assembly:20260403`, manifest
`sha256:602c467ae6b661a73441b960d9fa7130f068f4c6041d37ef8632611733d83f4a`.
Its 44,235,754 compressed bytes and 125,246,464 uncompressed tar bytes were
SHA/diff-ID verified before import. The real standard Harbor oracle reached
the unchanged official verifier and scored 0: the reference solver left `X`
in the selected 5WJ2 and 2H5Q sequences, matching translation failures at
nucleotides 324–327 and 1611–1614 exactly. It produced no `/app/gblock.txt`.
This is an upstream reference-solution failure; no oracle, API response,
sequence, official checker or expected score was replaced to make it pass.

Evidence is ignored under
`evals/promptfoo/results/qwen38-suite/diagnostics/protein-assembly/resume-20261030/`
and `evals/promptfoo/results/qwen38-suite/preflight/protein-assembly-20261031-cached/`.
`parent-oracle-failure.json` records the exact residue mapping. Direct fixture
success does not establish complete-parent readiness. The original parent
keeps 1 CPU, 4096MiB RAM, 10240MiB storage and both 1800-second limits. No paid
model call, model/budget change or shared suite-code change has occurred.

From the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/protein-assembly/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/protein-assembly/case.json --run-fixtures
```

After the upstream reference failure is resolved within the authorized source
contract, rerun standard preflight using a fresh output directory and the
ignored `resume-20261030/runtime.json` execution config. Historical top-level
`runtime.json` is a report without `historical_root`. Follow
`suite/BUILD_RESOURCES.md`, claim the shared Docker window, and remeasure host
and VM free space before execution. Only complete successful parent/child
calibration may create a readiness receipt.
