# fix-git independent diagnostics

Suite: terminal-bench independent diagnostics; preparation batch 22.

These four synthetic program tasks use real Git repositories and do not copy
parent data, solution prefixes, or model traces. Each task is standalone.
The submitted API is `python3 /app/solution.py REPO`; the verifier supplies a
fresh repository with variable object IDs. Public instructions define every
required behavior. Hidden checker/oracle files enter only the verifier phase.

| Child | Observable behavior | Negative programs |
|---|---|---|
| m1 | Exact ref IDs, abandoned commit parent IDs and ancestry; repository unchanged | Wrong ref; missing parent; inverted ancestry |
| m2 | Lost and initial master commits remain ancestors of clean master; exact files | Copy-only commit; recovery on wrong branch; reset losing master history |
| m3 | Clean master merge with ordered parents, resolved bytes and both branch files | Ours-only resolution; copied files without merge; deleted master file |
| m4 | Exact boolean audit across eight fresh valid/broken repositories; no mutation | Always-ready; ignores untracked files; confuses detached HEAD with master |

m4 probes valid state, detached HEAD, another branch, an untracked file, dirty
content, committed wrong content, missing profile, and copied content without
recovered ancestry. Both committed and working profile bytes are checked.
Repository snapshots detect mutations by read-only m1/m4 submissions; the
checker refreshes Git's stat cache before snapshotting. The first container
run exposed that refresh requirement. A separate red calibration demonstrates
that file-only m2 acceptance misses copy-only history; ancestry checks reject it.

Fixture tests use fresh offline containers: 1 CPU, 512MiB, 128 PIDs, dropped
capabilities, read-only system filesystem, 64MiB tmpfs and no repository mount.
Scratch repositories live under the isolated writable `/app`. Harbor controls
use fresh containers and the same CPU/memory limits, 1024MiB requested storage,
900s agent / 120s verifier, and no-network mode. Storage is requested, not a
claim that Docker enforces writable-layer quotas. The diagnostic tools-only
image is the existing audited image built by `environment/Dockerfile`, pinned
as `vw-configure-git-webserver-diagnostics@sha256:681f5a56bbcab388a9711d1db08a5ebf9175ecdde29946135f6750ced77280d9`
(linux/arm64); `/app` is empty and no tests, oracle, solutions or task data are
present. A new host must restore the exact local image or rebuild and recalibrate.

Original image: `alexgshaw/fix-git@sha256:389b9c8247610c2c5be080b1ac00429007c2c69bf57f7f26c79f0f75ba2d5c74`
(linux/amd64). Registry config and local RootFS diff IDs match. Original task
bytes remain frozen; official resources remain 1 CPU, 2048MiB, 10240MiB and
900s agent/verifier. Only the official two-file pytest determines parent reward.

The first parent verifier stalled during APT and was archived as infrastructure
failure with no reward. `environment/verifier-bootstrap.sh` supplies authentic,
hash-pinned curl, uv 0.9.5 and pytest 8.4.1/pytest-json-ctrf 0.3.5 wheel caches
only after the agent exits. The official test script and checker are unchanged.
Real apt-get calls retain their failure codes and are bounded to 30s each;
Git/Agent environment, scoring, expected bytes and 900s total remain unchanged.
An offline dependency probe verifies the installer checksums and pytest version.
The parent verifier runs in the original container, with no artifact replay.

Policy: token-plan/qwen3.8-flash, 32000 output tokens, 200000 input window,
24 requests / 32 tools, one attempt. Preparation makes zero model requests.
Calibration and infrastructure failures are separate from model scores.

Run from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/fix-git/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/fix-git/case.json --run-fixtures
```

Task-local runtime, raw logs, image/dependency hashes and resource observations
are under ignored `evals/promptfoo/results/qwen38-suite/diagnostics/fix-git/`.
Preflight uses `runtime-bootstrap.json` there, retaining the calibrated runtime-v2
identity. The old global `runtime.json` is an identity report and fails with
`KeyError: historical_root`; the requested command failure is archived without
modifying shared configuration. `validation.json` binds all declared diagnostic
bytes and the checker implementation. `preparation.json`, when present, binds
all ten fresh Harbor controls and their raw scoring evidence.

Validation on 2026-10-28: 20 isolated unittest cases and 20 fixture calibrations
passed (four gold accepted, four nop and twelve negatives rejected). All ten
fresh Harbor controls returned oracle=1 / nop=0 without exceptions or timeouts;
the original pytest collected two tests per parent control (2 passed for oracle,
2 failed for nop). `verify_preparation` revalidated current source, runtime,
checker, fixtures and raw first scores. No recovered-score substitution occurred.
Clippy and `evals/promptfoo/score.sh test` exited 0. Cargo dynamically acquired
slot 1 with absolute target/build directory
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1` and one build
job; its owned lock was released and cache retained. No Rust behavior tests or
model requests ran. Ignored `completed-evidence.json` binds exact commands,
exit codes, test counts, image/config hashes and raw resources/scoring evidence.
