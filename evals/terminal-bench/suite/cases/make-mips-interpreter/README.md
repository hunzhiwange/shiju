# MIPS interpreter preparation

Four independent synthetic diagnostics accompany the unchanged official DOOM
MIPS interpreter parent. No paid model was called. Original instructions,
configuration, environment, reference solution and three official tests were
read before design. Hidden upstream assets remain in ignored source storage;
diagnostic programs do not reuse the parent reference implementation.

| Child | Observable contract | Executable negative controls |
| --- | --- | --- |
| m1 | Decode MIPS32 fields, register indices and signed immediates | unsigned immediate, swapped registers, truncated target |
| m2 | Arithmetic, fixed zero register, little-endian byte/word memory | writable zero, big endian, unsigned offsets |
| m3 | Delay slots, jal/jr link, exact byte write and exit | skipped delay slot, wrong link, dropped NUL |
| m4 | Compare executed state with first-difference reporting | ignored stdout, truncated prefix, wrong priority |

Each child specifies a complete stdin/stdout Python API and synthetic input.
The checker executes submitted `run.py` against verifier-only probes and checks
actual typed JSON values. Gold and twelve faulty programs are calibration-only;
the agent sees only its instruction and example. All twelve negatives produce
valid JSON with exit 0 and are rejected for real mismatches, without relying on
candidate exceptions or timeouts. Omitting the m3 stdout comparison in a real
red/green test admitted the dropped-NUL negative, failing one of twenty tests;
restoring exact comparison passed all twenty.

The tools-only amd64 image is the clean cached task52 single-platform manifest:
`vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf`.
Its /app is empty and /tests, /solution and /oracle are absent. Both this image
and the original parent ran frozen x86 ELF `shiju --help` offline, without keys,
using the fixed runtime sidecars; ELF SHA256 is
`847823c5cd99a6828cddfc2e3e5dbb67c5ddcc9f8693a13d539d8a8fa47c9dae`.

Final controls use execution identity
`ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1`
and the formal `SuiteDockerEnvironment`. All eight fresh standard child
oracle/nop controls scored 1/0, with unchanged generated FROM, fixture bytes
and task settings. No private Trial, platform, main-service or Compose override
was used; production code applies resource limits. Main containers retain
1 CPU/512 MiB and the native egress sidecar 0.25 CPU/64 MiB. Diagnostic policy
remains 900/120 seconds, one attempt, qwen3.8-flash, 32000 output and 200000 input
tokens. Historical controls from the earlier execution identity are excluded.

The local controller initially misidentified a bounded classic-builder
CpuQuota/CpuPeriod as an unbounded NanoCpus=0. That admission failure occurred
before the final nop was launched. Seven already scored controls were retained
under the identical identity, and only the unstarted final nop was run after
using production `active_resources`. A resume assertion expected six rows but
found seven; it exited before scoring and is retained as controller failure.
Neither failure is counted as an agent result or repeated successful control.

The original parent manifest is
`sha256:082fc8821b317f30fdfbf8d08d528874ce331c03e8083aef0406d48cdd7132a2`,
config `sha256:818505509c7be8f53b6133b093076cb03527f7539c8ad980435cf556143ddcb8`.
All twelve compressed layers and uncompressed diff_ids were stream verified.
C is 518964876 bytes and exact U is 1577993728 bytes, measured under a positive
2 GiB decompression hard cap. Gzip ISIZE was used only as an initial lower
bound. OCI import streamed the original manifest/config/layers without an
assembled archive; original config and amd64 RootFS were checked after import.
Unique OCI bytes and original scoring receipts are retained.

Every capacity admission and running capacity check reserves a further 40 GiB
for the concurrent task. The original S=10 GiB, B=2 GiB, A=1 GiB and safety=5 GiB
remain intact; this task's conservative import reservation is 23031731108
bytes, below its 26 GiB scheduling envelope. Parent controls sample effective
min(host, VM) free bytes, own writable-layer sizes and evidence allocated bytes,
and stop their own project if a floor or B/A cap is breached. Resources remain
original 1 CPU/2048 MiB, storage 10240 MiB, and 1800-second agent/verifier limits.
Declared storage is not a claim that Docker enforces a writable-layer quota.

The original parent dependency probe found the fixed uv installer URL returned
403; Debian/PyPI and native Python 3.13.7/sqlite were available. The authentic
SHA-bound uv 0.9.5 installer/release and wheel cache from task54 were verified
read-only and probed in the exact original image: pytest 8.4.1, NumPy 2.3.1,
Pillow 11.2.1 and pytest-json-ctrf 0.3.5. `verifier-bootstrap.sh` is verifier-only.
It serves only the exact installer request from verified bytes, delegates other
curl calls and apt to their real executables, and uses real offline uv resolution.
No Agent environment, official checker, source, time limit or expected output
was changed; no scorer or artifact replay substitutes for execution.

Final original oracle passed all 3 official checks with reward 1; fresh nop
failed all 3 with reward 0. The nop's expected missing-frame test failure is
separate from harness exceptions and Agent/verifier deadlines. Both complete
within original limits, with no harness exceptions, recovery scores or orphan
task containers. All 20 focused unit tests and 20 fresh fixture controls pass.
`inputs` and `verify_preparation` bind the ten valid official/child controls,
source/runtime/adapter/Harbor identities, image, settings and diagnostic hashes.

Evidence lives in ignored
`evals/promptfoo/results/qwen38-suite/diagnostics/make-mips-interpreter/`:
`final-gate-current.json`, `parents-calibration-current.json`,
`children-calibration-current.json`, real CTRF/scoring logs, capacity and resource
receipts, exact layer hashes and dependency probes. The requested historical
preflight runtime report lacked historical_root and exited before scoring;
local runtime.json supplies explicit frozen paths and the unchanged policy.
The original disk shortage, earlier controller failures and old controls are
preserved separately, excluded from the final ten-control preparation receipt.

Repository gates `evals/promptfoo/score.sh test` and
`cargo clippy --all-targets --all-features -- -D warnings` pass; they execute
zero behavior tests. Clippy dynamically acquires one shared repository slot,
sets matching absolute CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR and JOBS=1,
then releases its lock while retaining caches. The actual slot and paths are in
clippy-receipt.json. No Rust was changed; no full Cargo test or Makefile ran.

Focused reproduction commands:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/make-mips-interpreter/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/make-mips-interpreter/case.json --run-fixtures
```
