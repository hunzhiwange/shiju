# build-pmars diagnostics

Four independent, offline diagnostic tasks; no paid model requests. Every
candidate has a fresh container, 1 CPU / 512 MiB, 900 s agent / 120 s verifier,
24 turns / 32 tools, and the fixed Qwen3.8-Flash 32000/200000 policy.

- m1 checks a **synthetic** Debian-style source archive and version metadata,
  then compares every extracted source byte. It rejects wrong package versions,
  missing C sources and altered source content.
- m2 accepts only a Makefile and compiles clean, real pMARS 0.9.2 sources. The
  checker inspects dynamic dependencies and runs a battle and cdb stepping.
  Controls retain X11, disable the debugger, or break linking.
- m3 runs an executable Python launcher against the installed pMARS, with 1,
  7 and 13 rounds and both warrior orders. Complete stdout/stderr/status must
  match independent direct invocations. Fixed rounds, swapped paths and invented
  results are rejected.
- m4 assembles and runs a 1–3 instruction synthetic warrior. It must survive
  against a terminating opponent for 3 and 11 rounds, in both orders. Immediate
  death, an invalid opcode and a wrong entry point are rejected.

The tool image contains only Python, GCC, make, libc/ncurses development tools,
and unmodified publicly distributed pMARS source code with a headless build.
It contains no official parent warriors, tests, solution, gold or negative
fixtures. The Dockerfile pins the base image and verifies the downloaded
Debian source archive SHA256; pMARS is GPL-2-or-later (see environment/COPYING and the source archive).
Immutable public input digests are bound in evaluator-only oracle metadata;
replacing an archive or opponent cannot redefine the expected behavior.
Public inputs and synthetic warriors are separate from evaluator-only oracle
metadata and candidate controls. No parent reference prefix or model trajectory
is used to generate these problems. No changes to official resources, source,
scoring, model, budget or baseline configurations are permitted.

Run from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/build-pmars/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/build-pmars/case.json --run-fixtures
```

Fixture validation is separate from complete parent/diagnostic Harbor oracle/nop
calibration. Ignored raw evidence is stored in
`evals/promptfoo/results/qwen38-suite/diagnostics/build-pmars/` and preflight
output, with hashes, image provenance, resource observations, timestamps and
initial failures retained. Completion requires all ten Harbor controls and all
twenty fixture controls; a clean build alone is insufficient.

Verifier setup uses only checksum-verified uv 0.9.5, Python 3.13.7 and the
official pytest 8.4.1 / pytest-json-ctrf 0.3.5 dependencies. The local scheduler
configuration is `diagnostics/build-pmars/runtime.json` beneath the ignored
results root. Bootstrap runs after the oracle/agent has finished, under
`/opt/vw-verifier-deps`; it does not modify the parent source, candidate binary,
warriors or official scorer. The original runtime evidence JSON lacks scheduler
paths and is retained unchanged. The first stalled GitHub downloader was
terminated after its process identity was recorded; that zero reward had no
executed tests and is an infrastructure failure, never a valid nop control.

The immutable-input regression first failed against a forged empty source
archive, then passed after evaluator-only digests were added. Final unit
validation runs 2 tests, including 20 executable artifact subchecks. Source
Makefile whitespace and the GPL license text are retained verbatim. The initial
preflight prepared its child package paths before the integrity repair; all
four packages were updated **before any child trial started**, proven equal to
fresh final-source materialization, and actually executed in fresh containers.
The original preflight metadata is retained; `final-binding/preparation.json`
binds the final executed bytes to the unchanged real scores. The standard
`verify_preparation` check validates the final receipt before delivery.

Final evidence: 20/20 fixture controls and 10/10 fresh Harbor controls passed;
parent oracle 4/4 official pytest tests passed, nop 0/4 passed as expected.
`verify_preparation` accepted the final receipt. Clippy exited 0 using dynamically
claimed slot 1, absolute target/build directories at the repository's
`target/rust-slot-1`, and one build job; its lock was released, cache retained.
No Rust behavior tests or model calls were made. Full commands, exit codes,
slot paths and source/dependency/scoring hashes are in the ignored
`diagnostics/build-pmars/acceptance.json`.
