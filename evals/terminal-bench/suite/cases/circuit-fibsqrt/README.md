# circuit-fibsqrt diagnostic preparation

The frozen official parent is unchanged: its original gate simulator, reference
solution, 28 input probes in three pytest tests, 32,000-step simulation, line
limit, 1 CPU / 2048 MiB, 10 GiB storage declaration, and 3600-second agent and
verifier limits are preserved. No model calls or model scores are produced.

Four synthetic, independently understandable diagnostics use new data:

- m1 implements a gate parser with constants, all binary operations, NOT,
  self-copy registers and forward references. Real candidate programs run on
  valid and invalid input. Negative controls confuse NOT with copy, reverse
  parsed operands, and accept nonconsecutive output indices.
- m2 constructs an eight-bit increment circuit. The checker interprets the
  submitted gate artifact on all 256 inputs, including carry chains and wrap.
  Negative circuits omit the initial carry, use OR instead of XOR for sum,
  or reverse output bit order.
- m3 implements a small ordered gate simulator. Sixty independent subprocess
  probes cover little-endian input, discarded high input bits, zero storage,
  unary/binary gates, feedback, delayed propagation and unsigned output.
  Negatives reverse input bits, initialize internal storage to one or print
  signed output. This diagnostic explicitly defines immediate ordered sweeps
  and small step counts; it does not claim to reproduce every parent event
  queue behavior.
- m4 compares outputs from a supplied fixed eight-wire NOT/copy simulator with
  unsigned XOR expectations. The checker runs the simulator itself and checks
  every result, expectation and boolean. Negatives use a wrong formula, claim
  equality for an incorrect observation, or omit a boundary input.

The initial existence-only checker incorrectly accepted all 12 negative
controls. The fixed checker rejects those controls and missing/malformed
submissions. Three unit tests execute 26 artifact controls, including wrong
boolean types and duplicate JSON keys. The separate container calibration
runs gold, nop and three negatives for each child in 20 fresh offline
containers (1 CPU, 512 MiB, read-only root, no capabilities, 128-process limit).
Only listed public synthetic fixtures enter diagnostic images. No upstream
solution prefix, hidden tests or old model trace is copied into them. Gold and
checker material enter only the corresponding Harbor phases. The diagnostic
Python image is pinned by digest and installs no dependencies.

The parent image is
`alexgshaw/circuit-fibsqrt@sha256:29783439f529eaed2145592f15af7a2281161528860392a382c825489030ae3a`
(linux/amd64). Registry compressed layer hashes, decompressed diff IDs, local
RootFS chain and original source identity are independently verified. The
capacity reservation includes three compressed copies, uncompressed layers,
10 GiB task storage, 2 GiB build cache, 1 GiB archive and 5 GiB headroom;
host and VM free space each exceeded that reservation. The exact downloaded
image was started with 1 CPU and 2048 MiB. Its /app contained only the public
simulator and example gates, with source hashes checked. Docker preserves the
storage declaration but does not enforce a writable-layer disk quota.

Container probes reached PyPI and Debian bookworm; the uv script initially
returned HTTP 403 through Python's HTTP client. The verifier-only bootstrap
supplies checksum-pinned uv 0.9.5 and official pytest 8.4.1 /
pytest-json-ctrf 0.3.5 dependency wheels after the agent exits. It changes no
parent task or official scorer bytes. The unchanged official test.sh can
still run its installer; its uv archive and wheel resolution use the local
verified cache. This setup is archived in the case-local ignored runtime.
The root results/runtime.json is historical identity evidence, not scheduler
configuration; use the runtime path below for calibrated generation 2.

Run from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/circuit-fibsqrt/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/circuit-fibsqrt/case.json --run-fixtures
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/circuit-fibsqrt/runtime.json --parent-id circuit-fibsqrt --output evals/promptfoo/results/qwen38-suite/preflight/circuit-fibsqrt
```

Preflight requires a new output directory. Raw commands, timestamps, image and
dependency hashes, TDD failures, container isolation/resource observations and
calibration results live under ignored `diagnostics/circuit-fibsqrt/` and
`preflight/circuit-fibsqrt/`. Model, output/input limits, tool/request budgets
and one-attempt policy remain frozen in case.json. Completion requires all ten
fresh Harbor oracle/nop controls and complete preparation receipt validation;
a successful build alone cannot establish readiness.

Final evidence: all 20 fixture controls and ten Harbor controls matched their
expected outcomes. The original parent oracle passed all three official tests;
parent nop failed the functional test. `verify_preparation` accepted the full
receipt. The three unit tests passed all 26 artifact controls, and Promptfoo
configuration validation exited 0. Required Clippy exited 0 using dynamically
claimed slot 1, with `CARGO_TARGET_DIR` and `CARGO_BUILD_BUILD_DIR` both set to
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1` and one build
job. Its lock was released and cache retained. Detailed command exits, image
identity, resources and evidence hashes are in ignored
`diagnostics/circuit-fibsqrt/acceptance.json`. Next: include the prepared case
in the full-suite freeze; paid evaluation still requires its own authorization.
