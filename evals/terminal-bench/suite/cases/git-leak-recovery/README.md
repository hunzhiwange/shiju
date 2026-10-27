# git-leak-recovery independent diagnostics

Suite: Terminal-Bench independent diagnostics; preparation batch 23.
All inputs are synthetic Git repositories created by the checker with variable
object IDs. No official hidden data, solution prefix or model output is copied.
The agent submits `/app/solution.py`; the verifier executes it on fresh inputs.

| Child | API after `python3 solution.py` | Independent checks | Negative programs |
|---|---|---|---|
| m1 | `REPO` | Exact sorted unreachable object IDs/types, all object kinds, no mutation | Empty result; reachable HEAD; commits only |
| m2 | `REPO OUTPUT` | Exact fictional credential plus newline; repository unchanged | Wrong value; HEAD-only data; destructive recovery |
| m3 | `REPO` | No stored object contains credential marker; refs, commit IDs/messages and tracked bytes preserved | No cleanup; expire reflog without pruning; recreate history |
| m4 | `REPO RECOVERED MANIFEST` | Exact boolean audit across eight valid/broken states; no mutation | Always ready; reachable-only scan; ignores file changes |

m1–m3 each probe loose and packed objects, including an independent orphan blob.
m4 probes valid, leaked, wrong/missing/symlink recovery, dirty content, changed
commit message and untracked file states. Expected values come from fixtures,
not from candidate output. Repository byte snapshots enforce read-only tasks.
Deletion of history is an explicit candidate rejection, not an infrastructure
error. The red checker probe demonstrates that credential-only acceptance
incorrectly accepts destructive recovery; snapshot checks reject that program.

The tools-only base is pinned to the already audited local arm64 image
`vw-configure-git-webserver-diagnostics@sha256:681f5a56bbcab388a9711d1db08a5ebf9175ecdde29946135f6750ced77280d9`.
`environment/Dockerfile` documents its source. It contains Python/Git and no
checker, solution, task data or oracle. A different host must restore the exact
image or rebuild and recalibrate. Fixture containers run offline with 1 CPU,
512MiB RAM, 128 PIDs, dropped capabilities, read-only system filesystem and
64MiB temporary space; no repository or credentials mount is exposed.

Policy remains token-plan/qwen3.8-flash, 32000 output tokens, 200000 input window,
24 requests, 32 tools and one attempt. Preparation makes zero model requests.
Official task files, 1 CPU, 2048MiB RAM, 10240MiB storage and 900s agent/verifier
limits remain unchanged. Local diagnostics do not constitute parent readiness.

Run the focused unittest and `suite.diagnostics validate --run-fixtures` commands
from task 42. `validation.json` binds the declared checker and all fixture bytes.
Raw logs, red evidence, runtime configuration and resource receipts live under
ignored `evals/promptfoo/results/qwen38-suite/diagnostics/git-leak-recovery/`.
Validation on 2026-10-28: 20 focused unittest cases and 20 fixture calibrations
passed. Eight fresh child Harbor controls returned oracle=1 / nop=0 without
exceptions or timeouts. Clippy and Promptfoo configuration checks exited 0.
Cargo dynamically acquired slot 1 with target/build directory
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`; the owned
lock was released and cache retained. Parent oracle/nop calibration also passed: the original five-test pytest
collected 5 tests in each control, oracle 5 passed (reward 1), nop 4 passed /
1 failed (reward 0), with no exceptions, skips or timeouts. All ten original
Harbor control results and raw first scores passed `verify_preparation`.
No recovered-score substitution or artifact replay was used.


The original parent image is
`alexgshaw/git-leak-recovery@sha256:62160e522a4238b8bcd522411ea97b5b200f811e5899d34947e66809bef7e586`
(amd64). Its config digest is
`sha256:ac4c027aef4e069789e75e13a1b03b9cecb5f42faef5c828cac183d25ba08fe2`.
Every compressed layer SHA256 and decompressed diff_id matched; exact compressed
C=95,724,176 bytes and decompressed U=222,528,512 bytes. Admission used the
smaller host/VM capacity and conservatively reserved three compressed copies,
10GiB official storage, 2GiB tooling, 1GiB evidence, 5GiB margin and 16MiB metadata.
Bounded checkpoint downloads refreshed anonymous authorization and enforced
real-time free-space and download ceilings. Parent resource inspection confirmed
1 CPU and 2048MiB; official 10240MiB storage is requested, not a claim of an
enforced writable-layer quota. A separate explicit amd64 probe read matching
cgroup CPU/memory limits. Image digest, config/RootFS and tag identity were
checked continuously during calibration, including before nop startup.

`environment/verifier-bootstrap.sh` only runs after the agent exits. It injects
authentic hash-pinned uv 0.9.5, Python 3.13.7, curl and pytest caches. An initial
probe inherited an irrelevant database-specific SQLite import; that import was
removed and the actual Python SSL/zlib/pytest path passed. No product code,
official checker, expected value, Agent environment or timeout was changed.
Real apt-get operations retain failure codes and have 30-second bounds inside
the original 900-second verifier limit. Ubuntu, uv installer and PyPI endpoints
returned HTTP 200 from the original image during a separate dependency probe.

The specified old `runtime.json` command fails with missing `historical_root`.
The task-local `runtime-bootstrap.json` uses the verified runtime-v2 generation.
Staged preflight ran eight child controls while waiting for disk admission, then
two parent controls; `verify_preparation` binds the complete ordered receipt to
current sources, runtime, checker/fixtures and raw first scores. No child controls
were repeated. `completed-evidence.json` records exact commands, exits, test
counts, dependency/image identities and resource observations in the ignored
evidence directory. Calibration is separate from official model scores.
