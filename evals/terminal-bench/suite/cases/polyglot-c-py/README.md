# polyglot-c-py diagnostics

Four independent synthetic tasks measure shared C/Python parser syntax (m1:
a literal marker), triangular sums (m2), argument validation and 64-bit factorial
results (m3), and polynomial/residue output (m4). Each instruction defines a
complete contract and the single `/app/shared.py.c` output. No parent Fibonacci
solution, upstream task bytes or hidden tests enter public fixtures.

The checker freshly compiles each submitted source and executes the same source
with Python and the compiled binary. Both exit status and exact stdout must
match independently computed results. Agreement alone cannot pass. Missing or
symlink sources and files over 64KiB are rejected. Executable scratch directories
are inside the writable candidate app, preserving the runner's non-executable
`/tmp`. Each child has gold, nop and three semantic negative controls.

The tool-only AMD64 Docker v2 manifest is pinned in case.json and
`environment/Dockerfile`:
`vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf`.
Its config digest is
`sha256:a6c16930afacbb61ef2e9f1e5f994d6c560b787fa4c5f849b5c15b3f017d111c`.
History inspection found only tool/package installation and wheel copies; the
initial `/app` is empty. Frozen Agent offline startup passed with the same
hash-bound binary/sidecars. This is ABI evidence, not model performance.

2026-10-30 preparation evidence is under ignored
`evals/promptfoo/results/qwen38-suite/diagnostics/polyglot-c-py/resume-20261030/`:

- Host unittest: 1 method covering 20 real gold/nop/negative programs passes.
  Fresh container fixture validation passes all 20 controls. The original TDD
  records remain in the parent evidence directory: a narrow-integer fixture
  defect was corrected, and disabling stdout comparison caused 10 failures.
- Shared diagnostics changed during this resume. Fresh fixture validation was
  rerun under implementation
  `2365b523747f771c31c18689f42b99cae85ed73b9b77f762138e4a33b3f776c4`.
- Ten fresh standard Harbor controls pass: original parent and m1..m4 each have
  first rewards oracle=1/nop=0. The original parent runs one official pytest
  test per control. `preparation.json` passes `inputs`/`verify_preparation` under
  scheduler `94dec6f0c90690b5c0ea348c8142a22b0cec1d62fa174df762f6dc93cb7653df`.
  Old scheduler controls and partial calibration groups interrupted by shared
  implementation changes are retained as historical evidence and excluded from
  current readiness. No old hash was rewritten to a new identity.
- Parent manifest is
  `sha256:0f1c3b7816d70cf5551573fd6aeef76893f2ae3000be2419997b6871b5d987ed`.
  Full streams verified compressed SHA256, all diff_ids and exact decompressed
  U=345676288 bytes, C=148540671 bytes, with a positive 352MiB decompression
  hard stop. Three host/VM samples admitted the conservative peak 20793930748
  bytes: four C copies, two U caps, 128MiB metadata, original S10+B2+A1+5GiB.
  Import preserved AMD64, config and RootFS; the frozen Agent parent ABI passed.
- The original parent has gcc 13.3.0 and lacks Python/uv. Dependency injection
  occurs only after the agent, through the shared verifier bootstrap. Authentic
  hash-pinned uv 0.9.5, Python 3.13.7, pytest 8.4.1, CTRF 0.3.5 and libsqlite3
  passed an offline dependency probe. The initial missing-sqlite failure and a
  probe import-name mistake are preserved. No Agent image or official scorer
  files were changed. The real apt wrapper preserves nonzero/timeout failures;
  repositories retain signature checks while using HTTPS. Original test.sh and
  test_outputs.py execute unchanged, within the original 900-second limit.
- Main containers are limited at creation to 1CPU/2048MiB for the parent or
  1CPU/512MiB for diagnostics; diagnostic egress sidecars use 0.25CPU/64MiB.
  The parent retains its original
  internet access and has no egress sidecar. Standard
  `suite.scoring.harbor_trial`/`SuiteDockerEnvironment` supply build/start
  admission, including the shared async-lock fix `ff353cdf3`. A pre-agent shared-lock
  failure is archived separately; the resumed
  controller waits for available resources and archives only pre-agent resource
  races for bounded recovery. Cross-batch task counts do not replace the global
  CPU/memory admission; the two-task rule applies within this batch. No valid score within an unchanged preparation identity
  was retried.
- `score.sh test` passes with the cached fixed Promptfoo version in offline mode;
  the ordinary invocation also exits 0, with a telemetry shutdown timeout notice.
  Required Clippy passes in dynamically acquired slot 2, target/build-dir
  `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-2`, JOBS=1.
  Its lock is released; no Rust changes or full Cargo tests were made.

`acceptance.json` records exact hashes, resources, cleanup, final host/VM samples
and the bound preparation identity. All owned containers, networks and volumes
are removed; source caches and historical failures are retained. No paid model
calls were made. Keep the frozen runtime, dependency caches and ignored receipts
available when validating or freezing this preparation.

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/polyglot-c-py/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/polyglot-c-py/case.json --run-fixtures
```
