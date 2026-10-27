# Mailman preparation

This case prepares four independent synthetic SMTP tasks; it does not replace
or rescore the official Postfix/Mailman parent. The complete upstream
instruction, Dockerfile, helper, reference solution and tests were read before
design. The official source tree remains unchanged. No model was called.

| Child | Observable behavior | Incorrect controls |
| --- | --- | --- |
| m1 | SMTP list/join/leave alias routing into separate mboxes | dropped join, swapped leave, wrong list destination |
| m2 | SMTP join/leave confirmation, sender binding, single-use token | immediate membership, other sender confirmation, replay |
| m3 | fixed member delivery, including sender, excluding outsiders | leaked copy, outsider posting, omitted sender |
| m4 | envelope-directed local mbox delivery and MIME content | dropped body, duplicate delivery, header-directed recipient |

Every checker starts the submitted `server.py PORT ROOT` in a fresh sandbox,
sends new unpredictable messages via `smtplib` and inspects actual mboxes.
No generated transcript or candidate self-test counts as evidence. Independent
fixtures contain only a synthetic domain, list name and users. The gold and
three distinct executable negative programs per child enter only calibration;
the checker and oracle metadata enter only the verifier. The tools-only image
is the cached amd64 Python 3.13.7 tools image already used by task 52, pinned
to its single-platform manifest. It contains standard Python tools and gRPC
dependencies, with no parent/diagnostic material: /app is empty and /tests,
/solution and /oracle are absent. Children and the official parent retain linux/amd64,
1 CPU/2048 MiB and 1800-second agent/verifier timeouts.

Fixture calibration uses the production runner's offline, read-only-root,
capability-free 1 CPU/512 MiB containers. Each server listens only on loopback;
external recipient domains are rejected. Final Harbor child controls retain the 900/120-second policy and use the
exact standard generated Dockerfile, with no main-service platform or image
reference overrides. Only the native egress sidecar receives explicit
0.25 CPU/64 MiB limits. Earlier ARM-image exploratory controls, both with and without a main platform
overlay, are archived separately and excluded from the final receipt because
compatibility with the frozen x86_64 Agent was not independently established.
No global platform override or new builder is used.

Development evidence includes a real red/green checker test: omitting the
sender from the m3 delivery assertion allowed `omit-sender` through and failed
one of twenty tests. Restoring the exact recipient-set comparison passed all
20 tests. An earlier mbox context-manager error is retained separately as a
checker error, not a successful control.

Evidence is stored in the ignored directory
`evals/promptfoo/results/qwen38-suite/diagnostics/mailman/`. `validation.json`
binds all 20 fixture controls and every declared byte. The requested preflight
command using the historical `qwen38-suite/runtime.json` exited before scoring
because that file is an identity report without `historical_root`; task-local
`runtime.json` supplies explicit paths and the unchanged official timeouts.

Focused commands:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/mailman/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/mailman/case.json --run-fixtures
```

Final preparation passed production `inputs` and `verify_preparation` against
all ten fresh controls. The original parent oracle passed all 3 official tests
(reward 1); nop failed all 3 (reward 0), with no skipped tests, exceptions,
Agent/verifier timeout or scoring replay. All four amd64 standard-package
child oracle/nop pairs scored 1/0, and the final image passed 20 focused tests
plus 20 independently executed fixture controls. No task containers remain.
`preparation.json` binds the original scorer logs, source/runtime/Harbor and
adapter identities, declared diagnostic bytes and parent-only dependency
policy. Child controls were completed before adding the parent-only bootstrap;
the final gate verifies that every child setting and all diagnostic, source,
runtime binary, adapter, scheduler and Harbor identities remained identical.
The old ARM controls are excluded; no frozen Agent compatibility is inferred
from oracle/nop controls.

Repository gates: `evals/promptfoo/score.sh test` and
`cargo clippy --all-targets --all-features -- -D warnings` both exited 0.
Clippy used dynamically acquired slot 1 with
`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`,
matching `CARGO_BUILD_BUILD_DIR`, and `CARGO_BUILD_JOBS=1`; its lock was released
and cache retained. These gates execute zero behavior tests. No Rust files
changed. The existing `block` future-incompatibility notice is retained in
the log.

Declared tools image:
`vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf`.
Its config digest is
`sha256:a6c16930afacbb61ef2e9f1e5f994d6c560b787fa4c5f849b5c15b3f017d111c`;
the config and RootFS are identical to the cached kv-store tools image. No
build, reindex or package change was needed. Reproduce local builds with
`DOCKER_BUILDKIT=0 COMPOSE_DOCKER_CLI_BUILD=1` and `DOCKER_DEFAULT_PLATFORM`
unset; the standard materialized FROM is used unchanged.

Original parent image manifest:
`sha256:04fac73db7e2b4f49d43f09fbc2266a94d70131f07efa1393d370abc059f4094`;
config:
`sha256:db488c3287e44deaa76c7bdb4b57019a9602b117a1132c99b2c1ae28b21c7ec7`.
Compressed layers total 161,823,791 bytes; exact uncompressed tar bytes total
433,165,312. Every compressed SHA256 and uncompressed diff_id was checked,
then the original manifest/config/RootFS and amd64 architecture were verified
after streaming OCI import without creating an archive or assembled layer
copy. Import admission conservatively reserved three compressed copies,
512 MiB U cap, 10 GiB official storage, 2 GiB tools, 1 GiB evidence and 5 GiB
margin. Before import, host free was 20,679,618,560 bytes and VM free was
92,023,574,528 bytes; the required reservation was 20,349,695,117 bytes.
Parent controls record both host and VM free space during execution and stop
only their own project on a violated floor. Cgroup receipts show original
1 CPU/2 GiB; storage remains the original declared requirement, not a claim
that Docker enforces a writable-layer quota. No original CMD restoration or
allowlist change was necessary: the image CMD is /bin/bash, not a service.

The original-container dependency probe found Ubuntu/PyPI reachable but the
fixed uv installer URL returned 403. `environment/verifier-bootstrap.sh` runs
only after the agent and uses SHA-bound authentic uv 0.9.5 installer/release
bytes plus a 46-wheel dependency cache. The unchanged official verifier still
installs pytest 8.4.1, mailman 3.3.8 and pytest-json-ctrf 0.3.5. Source-only
mailman/authheaders/authres/dkimpy were built as ordinary universal wheels;
no upstream source or dependency API was modified. The original-image cache
probe installed these packages and initialized a real Mailman database/list
successfully before calibration. `wheels.json`, the cache probe and bootstrap
receipts retain exact dependency hashes and versions. The bootstrap neither
replays artifacts nor replaces apt/uv with success stubs: apt invokes its real
binary with bounded network waits, curl serves only the exact fixed installer
request from verified bytes and delegates other requests to real curl, and
uv runs its authentic installer and resolver offline. Service state and mail
remain in the same original environment for official scoring.

The task-local `calibrate-amd64-children.py`, `calibrate-parent.py` and
`finalize-preparation.py` record exact commands, resources and unchanged-byte
checks, and feed the production preparation gate. Evidence categories remain
separate: the requested launcher failure, early checker exception and ARM
explorations are not part of the final ten-control receipt. The paused download
(exit 143 after a coordination update) resumed from its own checkpoints only
after the parent window was formally confirmed. No paid models were called.
