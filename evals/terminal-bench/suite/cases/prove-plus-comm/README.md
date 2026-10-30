# Coq proof diagnostics

Four independent synthetic exercises cover right-zero induction (m1), the
right-successor step (m2), addition by one without unproved premises (m3), and
addition commutativity (m4). Every prompt defines a fixed proposition and the
`/app/proof.txt` tactic-body interface. These fixtures contain no parent input,
reference solution, hidden tests or model trajectory.

The checker accepts an explicit closed tactic language, constructs its own
trusted theorem, compiles it with `coqc`, requires `Print Assumptions` to report
no global assumptions and runs `coqchk` in a fresh directory. Candidate goal
changes, fabricated `.vo` files and claimed test reports cannot replace kernel
acceptance. Candidate dependencies, `.coqrc` and `COQPATH` are excluded.
Every child has independent gold, nop and three rejection controls:
insufficient reflexivity, unfinished induction and an unproved premise.

The tools-only Linux amd64 image is pinned to single OCI manifest
`sha256:e0a94c66fb02d10f09da65092460a31003cdeca721aea8bb47ed4c128faa30f7`,
config `sha256:c3a1a773c60627f35224fab046b98873913444c35617b5168ad9c3d76edb8471`.
It extends the existing clean task52 Python/Debian base and installs Coq/coqchk
8.16.1. Signed APT metadata resolved 21 packages totaling 253,946,744 bytes;
`environment/dependencies.json` records exact versions, HTTPS URLs, sizes and
SHA256 hashes. Fetch those exact files into `environment/packages/`, verify
sizes and hashes, and build `environment/Dockerfile` using the shared bounded
builder. Python system packages are installed in explicit Pre-Depends order.
No dependency download occurs during the offline build. Initial unordered dpkg
and local-APT installation failures are retained separately in ignored logs.

The actual successful RUN was captured with CPU quota/period 100000/100000 and
memory=swap=536870912 from creation. The image contains no task/oracle/tests;
`/app` is empty. The frozen runtime-v2 x86_64 Agent and existing sidecars passed
read-only/offline execution and library checks in both tools and parent images.
The diagnostic runtime retains 1 CPU, 512MiB and 1GiB storage per child.

Tools admission retained original S=10GiB, B=2GiB, A=1GiB, 5GiB margin, three
compressed copies capped at 1GiB each and complete U capped at 4GiB: 25GiB.
The initial 512MiB combined download cap rejected the dependency set before
exceeding it; the larger cap was admitted from a new host/VM snapshot.
Streaming Docker save inspection wrote no archive file. Final compressed layers
total 968,478,819 bytes; complete U, counting every RootFS reference, is
2,159,507,968 bytes. Every compressed digest, config and diff_id matched.
Downloaded package/build-context copies fit the separate B reservation.
Only this task's verified, reconstructible download copies and failed COPY
images were removed afterward; hashes, URLs and cleanup receipts remain.

The original registry manifest is
`sha256:e26741d01681a9da1beeff8b7b7b65fe2b921b8b122a306a3707a81ebd051f73`,
Linux amd64. Its compressed total is 491,449,847 bytes and complete verified U
is 1,298,711,040 bytes. All layers were checked against registry digests and
RootFS diff_ids, then streamed into Docker without a full archive file.
The original Coq 8.18.0 and frozen Agent passed execution/library probes.
All nine source files still match the catalog. The official task/checker and
1 CPU, 2048MiB, 10GiB, 900-second agent/verifier budgets remain unchanged.

Semantic TDD deliberately accepted failed Coq compilations in an isolated
checker copy. Eight actual invalid proofs then escaped rejection, producing
nine unittest failures including the aggregate count. The unchanged real
checker passed five unittest methods with zero skips and all 20 controls.
`diagnostics validate --run-fixtures` independently passed 20 fresh containers.
The unit test uses local Coq when present and otherwise the pinned real Docker
checker; missing infrastructure is an error, never a skipped semantic test.
`validation.json` binds the case/checker/fixture bytes and implementation hash.
Promptfoo configuration validation passed; no paid model was called.

On 2026-10-31, ten fresh standard Harbor controls completed through
`suite.scoring.harbor_trial` and `SuiteDockerEnvironment`: parent and all four
children each received first oracle reward 1 and first nop reward 0. The
unchanged official oracle passed all four tests; official nop passed only file
existence and failed the other three. No verifier recovery, dependency patch or
model invocation was needed. Main/sidecar CPU and memory limits, amd64 layer
closure and disk snapshots were checked for every successful control.

The original capacity blocker is resolved. Parent admission retained S=10GiB,
B=2GiB, A=1GiB and 5GiB margin; its new capacity snapshot recorded 73,905,266,688
host bytes and 56,258,908,160 VM bytes free. Cached image growth was recorded as
zero separately from the verified complete C/U values. Children retain their
own S=1GiB reservation. A transient shared-resource refusal before m4 execution
and a controller inspect/disappearance race remain separate infrastructure
records. Already successful controls were preserved; no historical baseline or
unscored attempt was counted as a success.

`verify_preparation` passed against the same current execution identity
`588690e423e955cab7b2c335d302678cae81598b2ec421946065c88f964ce16c`.
`preparation.json` binds every official/case/runtime/adapter/Harbor hash and
all ten first-score evidence trees. Execution order was normalized to the
standard parent-first receipt format without changing any result. No official
scoring, shared suite implementation or default model policy changed.

Clippy passed (exit 0): `cargo clippy --all-targets --all-features -- -D warnings`,
using dynamically acquired slot 2, identical absolute CARGO_TARGET_DIR and
CARGO_BUILD_BUILD_DIR `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-2`,
and CARGO_BUILD_JOBS=1. The owned lock was released and cache retained.
The first slot-4 attempt was interrupted at the disk protection threshold during
concurrent external build growth (exit 143); that interruption remains separate
from the successful final check. Effective Rust behavior-test count is zero:
this task changes only evaluation assets and does not modify Rust source.
No full Cargo test, fmt check, wasm check or Makefile command was run.

Recheck fixtures from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/prove-plus-comm/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/prove-plus-comm/case.json --run-fixtures
```

For another standard preflight, use the ignored `resume-20261030/runtime.json`
execution config and a fresh output directory. `qwen38-suite/runtime.json` is a
report, not an execution config. Recheck capacity and shared execution identity
before starting; fixture validation alone cannot replace Harbor calibration.

Evidence: `evals/promptfoo/results/qwen38-suite/diagnostics/prove-plus-comm/resume-20261030/`.
