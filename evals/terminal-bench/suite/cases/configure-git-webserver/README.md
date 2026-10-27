# configure-git-webserver independent diagnostics

Suite: terminal-bench independent diagnostics; preparation batch 13.

Four synthetic tasks exercise separate Git deployment behaviors. They do not
copy the parent instruction, reference solution, tests, or model traces.

| Child | Submitted entry point | Independent acceptance | Negative implementations |
|---|---|---|---|
| m1 | `sh /app/setup.sh ROOT` | Bare master repository; real clone/commit/push/reclone and exact bytes/history | Working repo; wrong HEAD; missing repository |
| m2 | `sh /app/setup.sh ROOT` | Real post-receive deployment; updates/deletions; preview branch isolation | Nonexecutable hook; wrong work tree; deploys all refs |
| m3 | `sh /app/setup.sh ROOT` | Separate public checkout; private sentinel/mode preserved; no Git metadata or world-writable paths | World-writable tree; exposed metadata; clobbered private data |
| m4 | `python3 /app/serve.py PUBLIC_ROOT PORT` | Live loopback HTTP; exact and updated bytes, nested paths, missing/private-path 404 | Wrong document root; dotfile leak; escaping symlink leak |

Fixtures contain only public context and the complete standalone instructions.
Gold and negative programs are injected only during their calibration phase;
checkers and oracle routing enter only the verifier phase. Fixture calibration uses
a fresh offline container with 1 CPU, 512MiB, 128 PIDs, dropped capabilities,
read-only system filesystem, and 64MiB writable `/tmp`. The checker starts
programs in fresh unpredictable `/app` scratch directories (Git hooks require
an executable filesystem, while the adapter `/tmp` is noexec); m4 starts a real server
and always terminates it. Harbor oracle/nop controls use fresh materialized
tasks with the same CPU/memory policy and 1024MiB requested storage; their
network isolation uses Harbor no-network mode. No service artifact replay is used.

The original parent remains unchanged, with 1 CPU, 2048MiB, 10240MiB storage,
900s agent and 900s verifier. Storage is the official requested value; Docker
writable-layer quota enforcement is not claimed. Its nginx/SSH integration is scored exclusively
by the official checker. Child m4 uses a Python HTTP API; it is an independent
local HTTP diagnostic and does not replace official nginx/SSH scoring.

Policy remains token-plan/qwen3.8-flash, 32000 output tokens, 200000 input
window, 24 model requests / 32 tools, one attempt. Preparation makes zero model
requests. Calibration, infrastructure failures, and official scores stay
separate. `validation.json` binds declared bytes and the checker implementation;
`preparation.json` binds fresh Harbor oracle/nop scores.

Run from the repository root with the pinned Harbor Python environment:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/configure-git-webserver/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/configure-git-webserver/case.json --run-fixtures
```

Local raw evidence and runtime settings are in the ignored directory
`evals/promptfoo/results/qwen38-suite/diagnostics/configure-git-webserver/`.
The repository `evals/promptfoo/results/qwen38-suite/runtime.json` is a legacy identity record;
preflight needs the per-task runtime settings in this evidence directory.

Original image: `alexgshaw/configure-git-webserver@sha256:9e48389b917fd4650dda1f64406bd7198bbf20dc1c6b0e5c4d81a3c16369f88c` (linux/amd64).
Diagnostic image: `vw-configure-git-webserver-diagnostics@sha256:681f5a56bbcab388a9711d1db08a5ebf9175ecdde29946135f6750ced77280d9` (linux/arm64).
The local OCI archive must be loaded on a new host; this diagnostic image is not
published to a registry. The tools-only image uses the committed Dockerfile;
the final digest and package inventory bind the calibrated dependency bytes.
The temporary proxy build arguments use the existing Docker proxy and are
absent from the image configuration and diagnostic execution environment.

Validation on 2026-10-27: 20 container unittest cases passed; 20 fixture
calibrations accepted four gold programs and rejected four nop plus 12 negative
programs; all ten fresh Harbor controls returned the expected oracle=1/nop=0.
The original official pytest ran one test per control, with no agent timeout or
verifier dependency recovery. `verify_preparation` revalidated the complete
receipt against current source, runtime, checker, fixtures, and raw scoring.
Clippy and `evals/promptfoo/score.sh test` exited 0; no Rust behavior tests and
no model calls were made. Dynamic Cargo slot 1 used repository cache
`target/rust-slot-1` for both target and build directory, with one build job;
its owned lock was released and cache retained.

TDD preserved a real checker miss: the initial m2 checker accepted a hook that
deployed preview commits. A real preview push now detects it. Container red
results also exposed noexec `/tmp`; moving scratch repositories under `/app`
restored executable hooks while preserving container restrictions. Raw red and
green evidence, image/package/resource records, commands and exit codes are
bound in the ignored `completed-evidence.json`. No changes were made to parent
materials, shared execution code, default evaluation configuration, or budgets.
