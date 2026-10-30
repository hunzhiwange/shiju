# Scheme-like evaluator diagnostics

m1 parses nested expressions, quote, comments and string boundaries to JSON AST. m2 evaluates bindings, lazy branches and sequencing. m3 checks lexical capture, nearest-frame mutation, simultaneous let and stopping errors. m4 checks path-line consumption, remaining stdin, character IO and silent expression values.

All material is synthetic. The Python reference interpreter is independently written; no upstream solution or hidden tests enter these assets. Each child has gold, absent-artifact nop and three distinct semantic defects. check.py launches candidate processes on private temporary programs, compares exact stdout and exit status, and rejects stderr/timeouts. Host execution is only for trusted fixture controls, not untrusted model candidates.

Host unittest: 1 test, 20 real-process controls passed. A deliberately truncated checker accepted later-probe defects and made the same test fail (exit 1); restored checker passes (exit 0). Receipts and logs are under the ignored diagnostics/schemelike-metacircular-eval results directory.

The 2026-10-30 continuation verified all 20 controls in fresh Linux containers;
validation.json records their exact fixture/checker hashes. The digest-pinned
diagnostic base has an empty /app, and its image history contains package
installation without diagnostic solutions. Fixture runs use no network, a
read-only root, dropped capabilities, 1 CPU and 512 MiB each.

The official parent image is
alexgshaw/schemelike-metacircular-eval@sha256:b485be38bf80d56c1a4cd9da95bbf7ec129dc03f6786b86c0525518a96f6d1db.
Its interpreter and public test tree match the frozen upstream files, and
eval.scm is absent before the oracle runs. Official oracle reward is 1
(63 program checks passed); nop reward is 0 (all 63 rejected). The official
checker, reference solution, resource settings and 2400-second agent/verifier
timeouts are unchanged. Its verifier installed its dependencies normally;
no bootstrap, scoring recovery, platform or Compose override was needed.

Status: ready. All ten standard controls are calibrated: the parent and each
child have oracle reward 1 and nop reward 0. preparation.json passes the
current verify_preparation gate. Preflight and resumed controls use
suite.scoring.harbor_trial and SuiteDockerEnvironment. Lock contention,
capacity rejections and disappearing-container resource queries remain
separately archived as unscored infrastructure failures.

A concurrent runner change for another parent invalidated the implementation
hash during calibration. All 20 Linux fixture controls were refreshed under
the final runner. Before reusing scored controls, the continuation checked
that every other preparation input and every materialized child byte remained
identical. runner-refresh-final.json records this comparison; historical
receipts and failures remain archived.

Evidence lives under the ignored results/qwen38-suite/diagnostics/
schemelike-metacircular-eval/resume-20261030 directory, with original preflight
and resumed control logs under results/qwen38-suite/preflight/. Clippy and
score.sh test exited 0. Cargo acquired slot 1 dynamically, with
CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR set to
/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1 and
CARGO_BUILD_JOBS=1; the owned lock was released. No Rust behavior tests or paid
model calls were run.
acceptance.json binds the final receipts, counts and cleanup; resource-receipt.json
records the standard build/start admissions and main/sidecar resource limits.

From the repository root, deterministic controls can be rerun with:

```sh
export PYTHONPATH=evals/terminal-bench
evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/schemelike-metacircular-eval/tests -p test_cases.py
evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/schemelike-metacircular-eval/case.json --run-fixtures
```

Preflight needs the frozen runtime-v2 root and a runtime task entry retaining
the parent's 2400-second timeout. The continuation's exact runtime is saved
in diagnostics/schemelike-metacircular-eval/resume-20261030/runtime.json.
Use a fresh output directory for future preflight runs, then include this
parent and all four children in the later suite freeze. The unchanged
storage_mb values are declarations; no writable-layer disk quota is claimed.
