# Qwen 3.8 suite scheduler

Run from the repository root with the pinned Harbor 0.24.0 environment:

```bash
export PYTHONPATH=evals/terminal-bench
PYTHON=evals/promptfoo/results/task25/harbor-venv/bin/python
"$PYTHON" -m suite.run --help
```

The runtime configuration contains absolute `historical_root`, `source_root`,
`cases_root` and a `tasks` array of `taskId`, `workingDirectory`,
`agentTimeoutSec`. Each official timeout must match `catalog.json`. Diagnostics
use `/app` and 900 seconds unless explicitly configured. Optional `recovery`
contains either `{"mode":"service"}` or `{"mode":"artifact","paths":["/app/result"],"image":"IMAGE@sha256:DIGEST"}`.
Artifact recovery requires oracle/nop calibration in the exact pinned image.
The runtime directory must pass `suite.runtime` generation 2 verification; the
original historical runtime remains available for legacy consumers.

```bash
"$PYTHON" -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json \
  --runtime /absolute/runtime.json --parent-id PARENT --output /absolute/preflight/PARENT
"$PYTHON" -m suite.run freeze --catalog evals/terminal-bench/suite/catalog.json \
  --runtime /absolute/runtime.json --cases-root /absolute/cases --output /absolute/frozen
export VW_SUITE_PROFILE=/absolute/temporary-profile.json
"$PYTHON" -m suite.run run --manifest /absolute/frozen/manifest.json \
  --parent-id PARENT --output /absolute/experiment
"$PYTHON" -m suite.run resume --manifest /absolute/frozen/manifest.json \
  --parent-id PARENT --output /absolute/experiment
```

Preflight runs ten oracle/nop trials without a model. Raw scoring stays under
its output; a successful receipt is also written to `cases_root/PARENT`.
Freeze requires all 89 parent receipts and 356 diagnostic calibrations, then
binds all 445 task trees, execution settings, runtime, adapter and Harbor source.
It does not generate missing questions or replace official scorers.

The profile is a temporary mode-0600 JSON file accepted by `suite.agent`.
Use the same experiment output for every parent. Models run serially, each
case once, with no Harbor or provider retry. Diagnostics share a durable
24-request/32-tool ledger across nested tools and child agents. Mandatory
trusted gates reject missing state, trust, disabled hooks and hook failures.
No model credential belongs in command arguments or committed files.

A reservation is durable before worker launch. Resume attaches only to the
recorded process identity and held worker lease. A lost or ambiguous attempt
remains `unknown`, blocks new requests globally, and requires investigation;
resuming never reissues it. Finished attempts preserve their first score and
append the same contract record only once. An intentionally new paid attempt
requires an explicitly authorized new frozen experiment.

Verifier recovery keeps the first failure, original logs and unchanged artifact
SHA. Only verifier work is repeated, within the original scoring time budget.
Service recovery uses the original live environment. Missing rewards and
infrastructure failures remain unscored. Transport completion and official
reward have separate evidence in raw results, contract history and CTRF.

Offline checks (no Qwen requests):

```bash
"$PYTHON" -m unittest discover -s evals/terminal-bench/suite/tests -p test_run.py
"$PYTHON" -m unittest discover -s evals/terminal-bench/suite/tests -p test_preparation.py
"$PYTHON" -m unittest discover -s evals/terminal-bench/suite/tests -p test_budget.py
"$PYTHON" -m unittest discover -s evals/terminal-bench/suite/tests -p test_agent.py
"$PYTHON" -m unittest discover -s evals/terminal-bench/suite/tests -p test_runtime.py
```

`test_run.py` includes real offline containers and countable loopback model
processes. Native gate calibration/build receipts are archived under
`evals/promptfoo/results/qwen38-suite/runtime-v2/`; they are local evidence,
not official benchmark scores. `runtime-v2.diff` records the exact repair over
the preserved historical source, including its provider-retry boundary fix.
