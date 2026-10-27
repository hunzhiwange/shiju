# Shape-aware batching diagnostics

Four independent synthetic artifact tasks for `llm-inference-batching-scheduler`.
The original instruction, 1,600 input requests, reference solution, six official
pytest tests, analytical model, scoring, 1CPU/2048MiB/10240MiB resources and
1800-second Agent/verifier timeouts are unchanged. No paid model is called.
Diagnostic policy retains token-plan/qwen3.8-flash, 32000 output tokens,
200000 input tokens, one attempt, fresh containers and the suite's 900/120s
Agent/verifier budgets.

| Child | Observable contract | Incorrect controls |
| --- | --- | --- |
| m1 | Exact minimum prompt alignment at 1/63/64/65/127/128/129; preserve IDs/order | Round down, advance exact boundary, include generation |
| m2 | Two exact prompt buckets, feasible consistent shapes and complete coverage | Excess batches, mixed shapes, wrong hidden dimension |
| m3 | Two input buckets sharing at most two shapes; exactly-once coverage | Per-bucket cap only, omission, duplication |
| m4 | Select minimum-cost candidate and emit its actual plan plus exact arithmetic cost | Bad candidate, omitted compilation, sum instead of max decode bound |

m4 explicitly specifies a small synthetic cost model. It does not copy or
replace the official parent's cost model or change any official threshold.
The checker reads submitted artifacts, validates structure and constraints,
and independently computes candidate costs. It never scores claims, keywords,
or Agent-written tests. Synthetic inputs are public; checker metadata and gold
artifacts are injected only during verification/reference execution.

## Current verification

Twenty focused unittest controls passed, as did twenty fresh offline Docker
fixture controls (four gold, four nop, twelve meaningful negatives). The first
checker omitted the joint shape cap: a mutant using two shapes per bucket and
four across both was incorrectly accepted. The failing 20-test run is retained
in `red.log`; enforcing the union cap made all controls pass. The declarative
`validation.json` binds the current checker, fixtures and public suite adapter.
`evals/promptfoo/score.sh test` passed.

Clippy passed with shared slot 1, CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`,
CARGO_BUILD_JOBS=1. Command: `cargo clippy --all-targets --all-features -- -D warnings`.
Exit code 0; zero behavior tests (lint only). The owned lock was released and
cache retained. No Rust files changed and no full cargo test or fmt check ran.

Eight fresh Harbor child controls passed (four oracle reward 1, four nop
reward 0). Every main container used linux/amd64, 1CPU/512MiB; each native
sidecar had explicit 0.25CPU/64MiB limits. The first child attempt failed before
Agent execution because a global amd64 environment conflicted with the native
sidecar. That infrastructure failure remains archived separately; valid child
controls use task-local main platform settings. All own containers were removed.

Fresh original-parent oracle passed all six official tests; nop passed only
input integrity and failed the other five (reward 0). Both retained original
1800-second budgets and 1CPU/2GiB. There were no exceptions, Agent/verifier
timeouts or recovery scores. Actual resource/image receipts, six CTRF test
names, first scores and bootstrap evidence were independently checked.
`preparation.json` binds all ten valid controls and passed production
`verify_preparation` against the final current identity. Child controls were
reused only after verifying unchanged shared implementation, child settings,
fixtures and image identity; adding parent-only dependency recovery did not
change their execution contract.

Raw logs are in the ignored directory
`evals/promptfoo/results/qwen38-suite/diagnostics/llm-inference-batching-scheduler/`.
The combined preflight receipt is under
`evals/promptfoo/results/qwen38-suite/preflight/llm-inference-batching-scheduler/`.
The specified historical runtime preflight failed with missing `historical_root`;
the task-local runtime-v2 configuration retains the original parent timeout.
The final gate combines existing successful fresh controls without repeating
those controls merely to reproduce the historical launcher command.

Original-container probes found Python 3.13.7 and sqlite 3.40.1 working, PyPI
and Debian endpoints returning 200, the official uv installer endpoint returning
403, and the GitHub release request failing TLS. A verifier-only bootstrap
provides authentic cached uv 0.9.5 installer/release and offline wheel closure.
The unmodified official test.sh still runs the original installer and pytest
8.4.1 / pytest-json-ctrf 0.3.5. The cache was independently executed in the
original image before calibration; no Agent-visible dependency injection or
official checker edit occurred. Runtime recovery binds each source SHA256.

| Cache | SHA256 |
| --- | --- |
| Official uv installer | 8402ab80d2ef54d7044a71ea4e4e1e8db3b20c87c7bffbc30bff59f1e80ebbd5 |
| Official uv amd64 release | 2cf10babba653310606f8b49876cfb679928669e7ddaa1fb41fb00ce73e64f66 |
| Six-wheel archive | a2a51e2baa5d5084034bc1e0b68ef772ef13cafa184f10e62b80c7c69b7c1760 |

Registry metadata identifies the original linux/amd64 image manifest as
`sha256:19e79aa49be4e55dccca1dde966515ebe6852314b6010bcb9230afb48b996774`,
config `sha256:0aff6db1f586c1b1e14999750d1cdf8fb94f11bc69c700fcad9b43a7c2ef5686`,
with 44,181,013 compressed bytes. All six full compressed SHA256 and decompressed diff_ids matched; exact U is
125,164,032 bytes. Five cached blobs were reused read-only and the remaining
16,520-byte blob downloaded. Live host/VM free bytes were 20,926,042,112 /
92,034,887,680; the conservative admission requirement was 19,952,585,770
(2C + 512MiB U cap + original 10GiB storage + 2GiB tools + 1GiB evidence +
5GiB margin). Import streamed without an archive file, preserving the original
manifest/config/RootFS and architecture. Runtime host capacity monitoring found
no reservation-floor breach. The already-cached pure-tool diagnostic base is
pinned by digest in `case.json`.

Commands from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/llm-inference-batching-scheduler/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/llm-inference-batching-scheduler/case.json --run-fixtures
```
