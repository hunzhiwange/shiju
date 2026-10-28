# Scientific stack migration diagnostics

Four independent synthetic exercises: m1 ports a legacy entrypoint and preserves
integer/fractional semantics and Unicode; m2 computes centered numerical values;
m3 preserves CSV IDs, missing-data behavior and duplicates; m4 compares numerical
results with explicit relative/absolute tolerances. All use Python's standard
library so migration behavior can be isolated from package installation.

The checker runs submitted bridge.py programs on unseen inputs. It uses explicit
expected outputs, never imports the gold implementation, and rejects missing
artifacts, invalid JSON, nonfinite numbers, incorrect types and numeric errors.
Each child has gold, empty submission and three distinct semantic negative controls.
The digest-pinned clean Python image and public synthetic fixtures are separate
from verifier-only check.py/oracle files. No upstream hidden inputs are committed.

`validation.json` binds twenty fresh offline fixture controls with 1 CPU/512MiB,
a read-only root filesystem and no repository mount. TDD first exposed twelve
wrong implementations accepted by an existence-only checker; the independent
behavioral checker rejects all twelve, and the focused unittest runs 20 tests.

The complete original parent passed both official tests (oracle reward 1), and
a fresh empty submission failed both (nop reward 0). All eight current standard
Harbor child trials also match oracle=1/nop=0. They use SuiteDockerEnvironment,
exact materialized FROM references and unchanged policy, without Compose,
platform or Trial overrides. Both parent and diagnostic images pass the frozen
x86 ELF's offline --help probe. No Agent environment repair, verifier bootstrap,
artifact replay, scoring change or model call was used. A Python urllib probe
received 403 from the uv installer endpoint, but the official curl path actually
installed uv 0.9.5 and every pinned scientific dependency successfully in both
fresh parent trials; the original network observations and logs remain distinct.

Parent manifest is
`sha256:64e69cee13bf6b0b9016e735b51891bce996a60f1e2d1a005bf12c949e221d71`,
config `sha256:725e815168e64c91e9b45a71f3c989d3672b510647b98e7392c0bf9e7dc203c6`,
linux/amd64. Compressed layers total 346662397 bytes and verified decompressed
layers total 1042226176 bytes. Every compressed SHA256 and decompressed diff_id
was streamed under a 2GiB hard decompression cap before OCI import. All actual
compressed copies were reserved; import streamed an OCI tar without saving an
extra archive. Admission and ongoing monitoring use min(host, Docker VM) minus
40GiB reserved for the concurrent task, retaining official storage 10GiB, tools
2GiB, evidence 1GiB and safety 5GiB within this task's 26GiB envelope. Runtime
resources remain 1 CPU/2GiB for the parent, with original 600s limits; child main
containers use 1 CPU/512MiB plus the constrained offline sidecar.

`preparation.json` passes production inputs/verify_preparation for all ten
controls at execution identity
`ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1`.
It is a machine-bound receipt referring to ignored raw evidence, not a portable
claim that another machine can skip calibration. Original disk rejection and
older controls remain archived, separate from the current accepted controls.
The requested historical preflight runtime report lacks historical_root; the
local ignored runtime.json supplies explicit frozen paths and unchanged limits.

Ignored evidence and recovery scripts live in
`evals/promptfoo/results/qwen38-suite/diagnostics/modernize-scientific-stack/`:
final-gate-current.json, original-network and current parent calibration,
children-calibration-current.json, CTRF logs, dependency probes, exact layer and
capacity/resource receipts. Unique OCI parts, dependency/Cargo caches and raw
scores are retained; only this task's temporary containers and slot lock were
released. Clippy and Promptfoo configuration tests pass; they do not replace
behavioral calibration. Clippy dynamically used slot 1 with matching absolute
target/build directory and JOBS=1, recorded in clippy-receipt.json.

Focused commands:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/modernize-scientific-stack/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/modernize-scientific-stack/case.json --run-fixtures
```
