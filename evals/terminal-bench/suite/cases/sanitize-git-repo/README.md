# sanitize-git-repo diagnostics (preparation batch 49)

Four independent standard-library Python programs operate on fresh synthetic Git
working trees. Only explicitly fictional `SYNTH_<KIND>_<8 uppercase/digits>`
credentials are used. No official solution prefix, hidden input or model trace
is included. `/app/solution.py` is executed by the verifier, which checks actual
files and JSON results rather than statements about success.

| Child | Program arguments | Behavioral oracle | Three wrong programs |
|---|---|---|---|
| m1 | ROOT | Exact occurrence paths, line numbers and types; read-only | Empty scan, hidden-file omission, near-match false positives |
| m2 | FILE | All four placeholders, repeated occurrences, byte preservation | First replacement only, wrong placeholder, deleted lines |
| m3 | ROOT | Exact sanitized working tree; normal bytes, binary assets, Git metadata and permissions retained | README corruption, binary deletion, hidden-file omission |
| m4 | ROOT MANIFEST | Read-only audit of leaks, complete file set, normal hashes and sanitized hashes | Always ready, ignored normal changes, ignored placeholder errors |

m1–m3 each receive three freshly generated repositories with changing paths and
fictional values. Fixtures include CRLF, comments, executable files, binaries,
existing placeholders, near matches and multiple tokens on one line. m4 probes
clean, hidden leak, wrong placeholder, normal modification, missing and extra
file states. Expected values derive from constructed inputs, independently of
candidate output. Snapshot comparison includes directories and file modes and
rejects unexpected symlinks. The repository history is intentionally out of the
working-tree sanitization scope and must remain untouched.

## Calibration completed on 2026-10-30

Twenty host tests and twenty fresh Linux fixtures pass: four gold, four nop,
and twelve semantic negatives. The earlier red run demonstrated that a checker
which examined only sanitized files accepted README corruption and binary
deletion. Full working-tree snapshots reject both regressions.

The previous diagnostic image was ARM64. Python-only oracle/nop controls passed,
but the frozen AMD64 agent failed to start (ELF probe exit 133). The diagnostic
base now uses cached, digest-pinned AMD64 Python/Git tools:
`vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf`.
The base has an empty `/app`; it contains no task or oracle assets. The frozen
agent starts in both parent and diagnostic bases. All eight images produced by
the standard child build are AMD64 and retain the tools base's complete layer
prefix. No controller platform, Trial or Compose override is used.

Under the current shared Docker execution identity, all ten fresh standard
`SuiteDockerEnvironment` controls pass: parent and m1–m4 each score oracle=1 and
nop=0. The original parent oracle passes all three official tests; nop fails two
and passes the preservation test. First scorer results are used directly,
without verifier recovery or artifact replay. `inputs` and `verify_preparation`
accept the final receipt. Earlier ARM64 controls, pre-agent resource/lock
failures and superseded execution identities remain separate historical evidence.

The original parent is unchanged:
`alexgshaw/sanitize-git-repo:20251031`, manifest
`sha256:4b5234da5bb0d67f3b0bf8db40a2883c07a5219f62b64c2bf9ff1ac84cd0f672`.
All compressed layer hashes and decompressed diff IDs were verified; compressed
bytes=202928151, uncompressed tar bytes=368109056, hard cap=1073741824. Capacity
receipts include image copies and two parent storage/build/artifact reserves.
Parent resources remain 1 CPU, 2048MiB, 10240MiB and independent 900s timeouts.
Children retain 1 CPU, 512MiB, 1024MiB and 900s/120s timeouts; standard egress
sidecars receive their separate 0.25 CPU/64MiB limits. All ten admission receipts
fit 4 CPU/8192MiB. Final Linux fixtures held the shared resource lock throughout.

Required Clippy exited 0 with stable Rust input hashes, dynamically acquired
slot 1, both target/build directories set to the repository's
`target/rust-slot-1`, and one build job. Clippy contributes zero behavior tests.
Promptfoo configuration validation passed. No Rust implementation changed here.
Model calls=0; policy remains token-plan/qwen3.8-flash, 32000 output, 200000 input,
24 turns, 32 tools and one model attempt.

`calibration.json` summarizes results, image identities, resources and checksums;
`validation.json` and `preparation.json` bind the exact inputs and raw scorer
evidence. The historical global `runtime.json` lacks `historical_root`; actual
calibration uses the task-local `resume-20261030/runtime.json` bound to verified
runtime-v2. Original official files, default Promptfoo configuration and accuracy
baselines remain unchanged.

Focused checks (run container checks in an available preparation window):

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/sanitize-git-repo/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/sanitize-git-repo/case.json --run-fixtures
```

Ignored evidence: `evals/promptfoo/results/qwen38-suite/diagnostics/sanitize-git-repo/resume-20261030/`
and `evals/promptfoo/results/qwen38-suite/preflight/sanitize-git-repo-20261030-current-identity/`.
