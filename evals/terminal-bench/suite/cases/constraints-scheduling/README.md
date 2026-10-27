# constraints-scheduling diagnostic preparation

Four independent, synthetic personal-assistant tasks; no model calls or original
hidden calendars/solutions are copied into these fixtures. Each child exposes
only its own `input.json` and instruction. Only the named output is writable;
inputs are read-only and networking is forbidden.

| Child | Observable artifact | Independent expected behavior | Negative controls |
| --- | --- | --- | --- |
| m1 | result.json | Leap-day date, AM/PM, minute-preserving UTC intervals | afternoon conversion, truncated minutes, wrong date |
| m2 | result.json | Two maximal half-open intersections across three people | two-person intersection, union, missing interval |
| m3 | result.json | Named candidates surviving work/lunch/busy filters | ignores lunch, rejects endpoint contact, ignores work hours |
| m4 | meeting.ics | Earliest 45-minute slot at 10:25 UTC, title and exact attendees | rounds start, wrong duration, missing attendee |

Expected answers were derived from these small synthetic specifications before
any model execution. The checker reads actual candidate files, compares typed
JSON or parsed ICS event fields, and checks input integrity. ICS UID and PRODID
are not frozen strings; CRLF and attendee order are accepted. This is a narrow
UTC ICS contract, not a general RFC 5545 parser.

The initial intentionally permissive checker failed 16 negative/nop controls;
the corrected checker passes 23 focused tests and all 20 offline container
calibrations (four gold, four nop, twelve negative artifacts). Validation binds
fixture/checker bytes and the digest-pinned Python image in `validation.json`.
Fresh fixture containers have no network, 1 CPU, 512MiB, 128 PIDs, dropped
capabilities, read-only rootfs, and only explicit candidate/hidden-test mounts.

The parent remains the exact frozen upstream task, including official tests,
resources (1 CPU, 2048MiB, 10240MiB), and 1200-second agent/verifier limits.
Parent image: `alexgshaw/constraints-scheduling@sha256:567ce5a189f8d11ac461790876e934cc7af38391baf89a78f95f4dafc1fec3b0`.
The direct offline reference-solution probe passed all three original test
functions. Full Harbor oracle/nop evidence is required separately before ready.

Run from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/constraints-scheduling/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/constraints-scheduling/case.json --run-fixtures
```

Raw local evidence, execution configuration, and red/green logs live under the
ignored `evals/promptfoo/results/qwen38-suite/diagnostics/constraints-scheduling/`.
The task's suggested `qwen38-suite/runtime.json` is a historical identity
receipt lacking `historical_root`; its failure is preserved. The per-case
execution config instead uses the established, verified `runtime-v2` generation
and unchanged source/case roots. No model, scoring, or budget changes.

Final acceptance (2026-10-27): all ten fresh Harbor controls matched their
expected rewards (parent and m1–m4: oracle=1, nop=0); the unchanged parent
pytest collected three tests. `verify_preparation` accepted the receipt and
all source/checker/runtime/scoring hashes. No verifier recovery or dependency
bootstrap was needed. Actual parent runtime: Python 3.13.9, pytest 8.4.1,
pytest-json-ctrf 0.3.5; uv 0.9.5. Container uv/PyPI probes returned HTTP 200.
The initial network probe ran before curl installation and exited 127; this
is retained separately from the successful probe and official scoring.

Preflight command (exit 0):

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/constraints-scheduling/runtime.json --parent-id constraints-scheduling --output evals/promptfoo/results/qwen38-suite/preflight/constraints-scheduling-valid-runtime
```

`evals/promptfoo/score.sh test` and
`cargo clippy --all-targets --all-features -- -D warnings` both exited 0.
Clippy dynamically claimed slot 1, with `CARGO_TARGET_DIR` and
`CARGO_BUILD_BUILD_DIR` both set to
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`, and
`CARGO_BUILD_JOBS=1`. No Rust behavior tests were used for this Python-only
change. The owned lock was released and the compilation cache retained.

Image compressed layers total 49,849,705 bytes; all compressed hashes and
uncompressed diff IDs match the registry manifest and local RootFS chain.
The conservative disk peak is 19,570,925,778 bytes, below both measured host
and VM free space. Parent CPU/memory limits were observed as 100000/100000
and 2147483648 bytes. Official storage is preserved as declared; the Docker
backend does not enforce writable-layer quotas. Detailed image/layer,
resource/isolation, timestamps and evidence hashes are archived in
`diagnostics/constraints-scheduling/acceptance.json` and adjacent receipts.

Next: include these five independently scored entries in the full-suite
freeze; paid execution still requires the experiment's separate authorization.
