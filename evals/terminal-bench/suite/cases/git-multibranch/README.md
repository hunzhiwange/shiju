# git-multibranch independent diagnostics

Preparation batch 24; synthetic standalone tasks, no model calls.

| Child | Submitted program | Independent black-box acceptance | Negative implementations |
|---|---|---|---|
| m1 | `python3 /app/serve.py ROOT PORT` | Authenticated real Git HTTP clone, exact bytes and commit identity; missing/wrong credentials rejected | No authentication; wrong password; wrong root |
| m2 | `sh /app/setup.sh ROOT` | Empty bare main repository; real main/dev pushes deploy exact nested binary bytes to separate directories within 3 seconds | Swapped branches; main only; disabled hook |
| m3 | `sh /app/setup.sh ROOT` | Preview isolation, repeated main/dev updates, and one push updating both refs | Preview overwrites main; shared destination; stale main revision |
| m4 | `python3 /app/serve.py ROOT PORT CERT KEY` | Live self-signed HTTPS, independent main/dev routes, nested binary bytes, missing 404 and changed content | Swapped routes; plaintext service; fixed response path |

Each child has its own complete instruction and fresh environment. m1 uses a
read-only HTTP authentication sandbox; it does not claim SSH coverage. m2/m3
invoke actual executable post-receive hooks through Git, and m4 runs an actual
TLS server. No prerecorded service output or parent solution prefix is used.
The official parent alone checks SSH password access and Nginx integration.
Parent resources remain 1 CPU, 2048MiB, 10240MiB requested storage, and separate
900-second agent and verifier timeouts; upstream materials are read-only.

Diagnostic tools image is the existing cached
`vw-configure-git-webserver-diagnostics@sha256:681f5a56bbcab388a9711d1db08a5ebf9175ecdde29946135f6750ced77280d9`.
The committed Dockerfile documents its Python/Git tools-only build. It contains
no task fixtures, reference programs or checker. Reuse avoids a shared builder
or image download. Local images need to be provisioned on a new host.
Fixtures use fresh offline containers with 1 CPU, 512MiB RAM, 128 PIDs,
dropped capabilities, read-only system filesystem, and a 64MiB tmpfs.
Scratch Git repositories live under writable /app because hooks require an
executable filesystem. Started services are terminated on success or failure.
Harbor child controls use fresh no-network environments, 1 CPU/512MiB and
1024MiB requested storage. Storage quotas are not claimed to be enforced.

Policy stays token-plan/qwen3.8-flash, 32000 output tokens, 200000 input tokens,
24 requests/32 tools, one attempt. Preparation uses no paid model.

Run from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/git-multibranch/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/git-multibranch/case.json --run-fixtures
```

Raw logs, resource inspections and task-local runtime-v2 configuration live in
the ignored `evals/promptfoo/results/qwen38-suite/diagnostics/git-multibranch/`.
`validation.json` binds fixture, checker and policy bytes. Full readiness
requires `preparation.json` with fresh parent oracle/nop and all child controls.
Do not substitute legacy runtime identity or earlier scores.

TDD evidence reproduces the initial branch isolation miss: a checker examining
only first main/dev deployment accepts a preview hook that overwrites main.
A real preview push and later independent updates reject that implementation.

The original image is `alexgshaw/git-multibranch:20251031`, manifest
`sha256:5bc449921c138295f6ddd8dca2977319fee2ce523811dbe56cec4ca663385fb5`,
config `sha256:ec6eeb02cecf35faf5badd75646ab0dd2e02cab484638b413ccd4606a2e2ef7d`
(linux/amd64). Every compressed layer and decompressed diff_id was verified
against retained official OCI bytes before importing through a tar stream.
Exact C=222129979 and U=633505792; the unchanged admission formula
2C+U+18GiB required 20405118582 bytes. Host free=20500905984 and VM
free=87754059776 at admission. No new layer download or disk archive was needed.
Only coordinator-authorized duplicate archive/parts were deleted after exact
byte verification against retained complete blobs; the ignored per-path receipt
records hashes, offsets, sizes and reconstruction instructions.

The first raw parent control never collected a test: the official uv download
failed with an SSL connection error, leaving uvx missing. This is recorded as
verifier infrastructure failure, separately from calibration. The task-local
runtime injects authentic cached uv 0.9.5, Python 3.13.7 and pinned verifier
wheels only after the agent exits. `environment/verifier-bootstrap.sh` installs
and verifies these dependencies; its input archive hashes and environment
are bound in the preparation receipt. Official test.sh/test_outputs.py,
reference solution, agent environment, resources and 900-second limits are
unchanged. The reference solution reports a missing tmux at its final keepalive
command; this was not patched, and the original functional test determines
the calibration score. No service artifact replay is used.

Validation on 2026-10-28: 20 fixture-container unittests passed; 20 fixture
calibrations accepted four gold implementations and rejected four nop plus
12 negative implementations. Eight fresh Harbor child controls returned
oracle=1/nop=0. Two fresh original-parent controls with verifier-only cached
dependencies ran exactly one official pytest each: oracle 1 passed/reward=1,
nop 1 failed/reward=0 due to SSH connection refused, with no skipped tests or
trial exception. All ten controls passed `verify_preparation`; the earlier
zero-test uv download failure remains separately archived. Original agent and
verifier timeouts remain 900 seconds, with no timeout multiplier or replay.
The parent nop container inspection confirms 1 CPU, 2147483648 bytes RAM and
the exact original image. Harbor has removed its completed containers.

Clippy exited 0 using dynamically acquired slot 1:
`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`,
with matching `CARGO_BUILD_BUILD_DIR` and one build job. Its owned lock was
released and cache retained; Clippy runs zero behavior tests. Promptfoo
configuration checks exited 0. Raw commands, exit codes, test counts, capacity,
source hashes and final artifact hashes are recorded in the ignored
`completed-evidence.json`. No Rust files or shared evaluation defaults changed.
