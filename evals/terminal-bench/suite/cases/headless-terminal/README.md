# headless-terminal diagnostics

Four independent synthetic exercises isolate real PTY creation and byte
exchange (`m1`), control keys and persistent shell state (`m2`), live terminal
geometry and UTF-8 boundaries (`m3`), and multiple independent sessions with
consuming reads and bounded idle waits (`m4`). Each public instruction defines
its complete Python API and observable behavior. The synthetic API deliberately
exposes read/resize/close operations; it is not a prefix of the official
BaseTerminal solution. No official solution, tests, hidden material or model
trace enters public fixtures. The checker imports candidate implementations
in a bounded subprocess and drives actual interactive bash sessions.

Each child has an independent gold, missing-program nop and three meaningful
faulty implementations. Faults cover pipes in place of PTYs, implicit Enter,
truncated output, ignored Ctrl-C/Ctrl-U, lost variables, fixed geometry,
stateless UTF-8 decoding, dropped Unicode input, replayed output, shared
sessions and ignored read timeouts. Expected shell output comes from live
commands with fresh nonce values. No source keyword or agent self-test is
accepted as correctness evidence.

The pinned public Python/compiler diagnostic image is reused from
build-cython-ext. Its empty `/app`, history, tool versions and offline
one-CPU/512MiB cgroup probe are archived. Fresh child environments receive only
their own public synthetic fixture. Shell, process and terminal behavior are
executed inside Linux containers. No paid model is called.

The frozen official parent uses linux/amd64 image
`sha256:eb7e209672bf6cef2785fafd9e13509b10626c327bcc2b37f5bf40ca83eaf3aa`,
configuration `sha256:41e1eca1ec72a2873c91d2e1b988b0565c0b1e9a0e65ddc6df95faa786336838`.
All compressed layer SHA256 and decompressed diff_id values match the
Registry configuration. Compressed layers total 62,277,092 bytes; exact
uncompressed tar size is 152,801,792 bytes. Before downloading, a 256MiB
hard ceiling reserved unknown U; exceeding it would prevent import.
Exact-U admission then checked host and VM capacity against three compressed
copies, original 10GiB storage, 2GiB tool cache, 1GiB archive, 5GiB reserve
and 16MiB metadata overhead. Original one CPU, 2048MiB RAM and separate
900-second agent/verifier limits are retained. Writable-layer quota
enforcement is not claimed. Local tag lookup was repaired with `docker tag`
from the verified digest, without re-resolving the upstream floating tag.

`environment/verifier-bootstrap.sh` runs after the agent exits. It makes
SHA-bound authentic wheels available to the original system Python 3.13.7:
pytest 8.4.1, requests 2.32.5 and pytest-json-ctrf 0.3.5 with pinned transitive
wheels. A verifier-only pip configuration uses that offline cache. Real
apt-get calls retain their exit status and get a 90-second network bound;
Vim is actually installed by the unchanged official test script. The original
system Python already has its SQLite dependency, so no replacement Python,
uv or extra libsqlite3 closure is needed. Separate probes installed Vim,
imported the real packages, checked pytest version, and reached the Debian
and relevant PyPI endpoints from the original image. The official source,
solution, tests, resource declarations and scoring are unchanged.

Evidence and commands are archived beneath the ignored directory
`evals/promptfoo/results/qwen38-suite/diagnostics/headless-terminal/`.
`validation.json` binds the fixture/checker bytes. `preparation.json` binds
all ten fresh scorer controls, frozen source, runtime-v2 identity and the
verifier-only dependency identity. Original logs, resource observations,
manifest/config, full verified OCI blobs and dependency caches remain there.
Only this task's redundant verified download parts and import tar were removed
after rehashing the retained complete OCI blobs.
Preflight uses that directory's `runtime.json`; the historical suite-level
`runtime.json` is an identity report, not an execution configuration.

Focused commands from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/headless-terminal/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/headless-terminal/case.json --run-fixtures
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/headless-terminal/runtime.json --parent-id headless-terminal --output evals/promptfoo/results/qwen38-suite/preflight/headless-terminal
```

Checker development evidence: a weakened import-only checker produced twelve
failures in the twenty-test suite because it accepted all faulty programs;
restoring live PTY probes passed all twenty tests. The initial and restored
runs both exited 0. The separate fixture calibration is recorded in
`validation.json`.

Repository gates: `evals/promptfoo/score.sh test` and
`cargo clippy --all-targets --all-features -- -D warnings` exited 0.
Clippy dynamically acquired slot 1 with absolute
`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`,
matching `CARGO_BUILD_BUILD_DIR`, and `CARGO_BUILD_JOBS=1`.
The lock was released and cache retained. Clippy is a build/lint gate with
zero behavior tests; it does not replace the terminal controls.

Final fresh calibration passed: the original parent oracle collected seven
tests and passed all seven (reward 1; 14.06 seconds of pytest); fresh nop
collected seven and failed all seven (reward 0; 0.12 seconds). Neither had
an exception, timeout or skipped test. Each child independently produced
oracle reward 1 and nop reward 0 in a fresh container. The production
`verify_preparation` gate accepted all ten scorer controls against the current
bytes, source, runtime-v2 and verifier-only dependencies. Twenty focused
unit controls and twenty separate fixture controls passed. All task containers
and sidecars were removed after their results were saved; no Cargo lock remains.
