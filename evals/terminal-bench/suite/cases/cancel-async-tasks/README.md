# cancel-async-tasks diagnostics

Four independent synthetic Python tasks cover input validation, bounded
concurrent execution, exception propagation with cancellation of siblings,
and cancellation cleanup/return timing. Each task has a complete public
contract and examples.json; the candidate writes only /app/run.py. No official
solution, hidden tests, old model traces or credentials are copied here.

The checker runs submitted functions in a fresh interpreter with an eight
second process timeout. Event barriers prove concurrency without relying on
throughput timing. Counters verify exactly-once execution and peak active
jobs. Distinctive exceptions must survive directly or inside exception groups.
External task cancellation must propagate only after asynchronous finally
blocks finish, with queued factories untouched. Successful completion must
also wait for cleanup. Candidate stdout and self-authored tests are not proof.

Each child has gold, nop and three behavioral mutations. The initial
file-existence checker accepted all 12 mutations; the test failed 12 subcases
before the executable checker replaced it. The exception exercise deliberately
rejects a naive semaphore plus TaskGroup implementation that releases a slot
and invokes a queued factory before TaskGroup notices the sibling failure.
Its gold schedules only after checking completed tasks for failure.

Diagnostics use the pinned clean native Python 3.13 image in case.json. Its
recipe contains no task materials. Every fixture control runs in a fresh
offline container with 1 CPU, 512MiB, dropped capabilities, read-only rootfs,
and separate read-only checker/oracle mounts. Harbor injects gold only in the
solution phase. The official parent package and scorer remain unchanged.

The documented task command selects historical runtime.json, an identity
receipt lacking execution paths; its KeyError is archived. Preparation uses
an ignored case-local runtime configuration selecting the verified generation
2 runtime, frozen source and original parent timeouts. The verifier-only
bootstrap extracts hash-verified Python 3.13.7 and uv 0.9.5 plus pinned official
pytest dependencies after the agent exits. It does not modify /app, official
checks or expected values. It provides offline dependency resolution after
an initial bounded container probe of astral.sh returned 403. Original test.sh
still executes, including its installation and scoring commands. Dependency
file hashes and exact paths are bound by the preparation receipt.

Run from repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/cancel-async-tasks/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/cancel-async-tasks/case.json --run-fixtures
```

Raw commands, red/green logs, resource observations, immutable image manifest,
dependency receipts and calibration evidence are in ignored results under
qwen38-suite/diagnostics/cancel-async-tasks and preflight/cancel-async-tasks.
The initial child setup attempt failed before agent execution because Docker
required an explicit memory-swap limit when capping its egress sidecar. That
infrastructure failure is retained; fresh child controls use 0.25 CPU,
128MiB memory and 256MiB memory-swap for sidecars, without changing network
policy or main-container limits. Completed parent controls are retained and
only children resume in new containers.

No paid model is called; policy stays Qwen3.8-Flash, 32000 output tokens,
200000 input tokens and one attempt.

Final calibration on 2026-10-27: the parent oracle passed all six unchanged
official tests, nop received zero. Four independent fresh child oracle/nop
pairs passed (10 Harbor controls total). All 20 fresh fixture controls passed,
including 12 mutations; one unittest with 20 behavioral subchecks passed.
Promptfoo configuration validation and Clippy exited 0. Clippy used dynamically
claimed slot 1, absolute target/build directory at the repository's
target/rust-slot-1 and one build job; its lock was released and cache retained.
The block 0.1.6 future compatibility notice remains unrelated. No Rust behavior
tests were executed. The ignored acceptance.json binds exact commands, exit
codes, image/layer/dependency/checker hashes and resource/isolation evidence.
