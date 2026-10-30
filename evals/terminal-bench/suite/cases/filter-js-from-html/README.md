# filter-js-from-html diagnostics

Four independent synthetic tasks isolate HTML structure parsing (m1), removal
of dangerous nodes and event attributes (m2), entity/control/case-obfuscated URL
handling while preserving safe content (m3), and a UTF-8 in-place file interface
with spaces in paths, idempotence and preservation of other files (m4).

m1 reads HTML on stdin through solution.py and emits ordered JSON parser events.
m2–m4 run filter.py against actual files. The independent checker compares
parsed output with hand-specified sanitized structures, including text,
attributes and safe links. It rejects missing programs, crashes, modified
fixtures, output-file symlinks and stdout-only sanitizers. Each child has gold,
nop and three distinct meaningful incorrect implementations. These are narrow
synthetic contracts, not claims of general browser XSS safety. No upstream data,
reference solution or model output enters the diagnostic environments.

The upstream task remains read-only at tree
7939565b055a4b4fa4d87ddc76b4250f03810362, with the official instruction,
reference solution, two pytest tests, image, 1 CPU, 8192 MiB memory,
10240 MiB storage, and 1800-second agent/verifier limits unchanged. The task
configuration is authoritative where the upstream README differs. Diagnostics
retain the suite's single-attempt policy, 32000 output tokens, 200000 input
window, token-plan/qwen3.8-flash and fresh environments. Preparation calls no
paid model.

validation.json binds fixture/checker hashes and 20 fresh offline restricted
Docker checks. Full readiness also requires a preparation receipt containing
all ten fresh Harbor oracle/nop trials and actual upstream test counts.
Raw image, resource, dependency and calibration evidence stays in ignored
qwen38-suite/diagnostics/filter-js-from-html and preflight/filter-js-from-html.

The verifier bootstrap uses authentic hash-checked cached tools and dependencies
only after the agent exits. Original-source calibration remains blocked by the
official helper's kill-before-quit cleanup and swallowed browser exceptions.

On 2026-10-30 the user explicitly authorized A+B, then separately C, for
`filter-js-from-html-compat-cleanup-load120`. `compat/cleanup.patch` removes the
premature driver kill; `compat/infrastructure.patch` propagates real startup,
navigation and alarm failures while retaining the normal no-alert timeout.
`compat/page-alarm.patch` increases the per-page alarm from 20 to 120 seconds;
full page loading and the subsequent 3-second alert check remain in place.
Assertions, attack inputs, reference solution, browser/driver versions, 1 CPU,
8192 MiB and both 1800-second limits stay fixed. The task-local materializer
checks the entire upstream tree and pins the original image digest. It writes
a separate package; it never edits upstream files. The earlier A+B-only full
calibration failed on its seventh batch due to navigation exceeding 20 seconds;
its raw reward 0 is classified as infrastructure/unscored, never capability 0.

`compat/probe.py` checks the patched helper in the verifier's sh/uv process
chain using two real blank-page browser sessions, a real missing-binary startup
failure, and synthetic navigation/alarm faults with real browser cleanup. Fault
injection exists only in this unscored probe. Full parent calibration runs the
patched helper directly, with no injection, browser replacement or reaper.

`compat/calibrate.py` requires successful probe and eight child Harbor receipts,
then runs fresh parent oracle/nop trials. It rejects browser infrastructure
errors and incomplete batch execution independently of reward. Its output is
`compatibility-preparation.json` in the ignored results directory, never an
official `preparation.json`. Compatibility scores must not enter original
official aggregates or be frozen as original readiness. No paid model is called.

Completed on 2026-10-30: all ten Harbor oracle/nop calibrations passed under
their respective identities. The compatibility parent oracle passed both tests,
all 28 browser batches (439 attack vectors) and 12 clean HTML samples; nop failed
both tests as expected. All four diagnostic oracle/nop pairs and 20 offline
fixture checks passed; 25 focused unit tests and the required Clippy gate passed.
The five-path lifecycle probe left no active browser processes at any endpoint.
An additional fresh `compat/canary.py` control detected a real JavaScript alert
and left no active browser processes. See `compat/calibration.json` for hashes,
resource limits and separate raw evidence locations. Original readiness remains
blocked and no original `preparation.json` exists.

To reproduce the authorized compatibility calibration after restoring the pinned
image and cached verifier dependencies, use new output directories:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python evals/terminal-bench/suite/cases/filter-js-from-html/compat/run_probe.py --runtime evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/runtime.json --output evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/compat-probe
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python evals/terminal-bench/suite/cases/filter-js-from-html/compat/calibrate.py --runtime evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/runtime.json --output evals/promptfoo/results/qwen38-suite/preflight/filter-js-from-html/compat-review --probe-receipt evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/compat-probe/probe-receipt.json
```

The second command includes all eight child trials unless an already verified
`--children-receipt` is supplied. Admission failures are preserved separately;
only failures before agent/scorer execution are eligible for an admission retry.

Validation from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/filter-js-from-html/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/filter-js-from-html/case.json --run-fixtures
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/runtime.json --parent-id filter-js-from-html --output evals/promptfoo/results/qwen38-suite/preflight/filter-js-from-html/harbor
```
