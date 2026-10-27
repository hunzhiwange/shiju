# count-dataset-tokens diagnostics

Four independent synthetic tasks use the actual Qwen2.5-1.5B-Instruct tokenizer;
no parent dataset rows, reference outputs, hidden tests or model traces enter
agent-visible fixtures. The only writable artifact is `/app/result.json`.
Network access and input modification are forbidden. The parent remains the
unmodified frozen Terminal-Bench task, with 1 CPU, 2048MiB, 10240MiB storage,
and 900-second agent and verifier timeouts. No model requests are made here.

| Child | Observable result | Three negative controls |
| --- | --- | --- |
| m1 | Ordered IDs for exact chemistry/biology/physics domains | science label, case folding, all domains |
| m2 | Separate reasoning/solution counts for English, Chinese and numeric text | concatenated fields, character counts, extra special tokens |
| m3 | Every JSONL record, empty/null text, chunks inside a JSON key, CRLF and no final newline | dropped empty record, counted null markers, dropped last record |
| m4 | Per-row, per-domain and grand totals with selected row count | inconsistent total, omitted solutions, aggregate without audit rows |

The checker parses real candidate JSON files, verifies input SHA256 and compares
complete typed structures against frozen synthetic expected results. It rejects
missing outputs, malformed/duplicate keys, booleans as integers, symlink outputs,
modified inputs and unexpected fields; JSON key order and whitespace are free.
The Qwen counts were computed independently before any model run with tokenizers
0.22.2 and checked against the tiny fixtures (m3 total 6, m4 total 13). Gold
artifacts never become public environment files. Fixture oracle calibration
checks artifacts, not a claimed implementation strategy.

Tokenizer provenance: Qwen/Qwen2.5-1.5B-Instruct revision
`989aa7980e4cf806f80c7fef2b1adb7bc71aa306`, Apache-2.0.
Download `tokenizer.json` and `LICENSE` from
`https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct/resolve/989aa7980e4cf806f80c7fef2b1adb7bc71aa306/`.
SHA256:

- tokenizer.json: `c0382117ea329cdf097041132f6d735924b697924d6f6fc3945713e96ce87539`
- LICENSE: `832dd9e00a68dd83b3c3fb9f5588dad7dcf337a0db50f7d9483f310cd292e92e`

Place these files with `environment/Dockerfile` in a separate build context.
The tokenizer and license are installed at `/opt/qwen`; hidden material is never
part of that image. `case.json` pins the resulting image manifest. Raw build,
dependency, source, isolation and calibration evidence lives in the ignored
`evals/promptfoo/results/qwen38-suite/diagnostics/count-dataset-tokens/` directory.

Local verification from repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/count-dataset-tokens/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/count-dataset-tokens/case.json --run-fixtures
```

The supplied historical `runtime.json` is an identity receipt, lacking execution
configuration fields. A per-case execution configuration uses the verified
runtime-v2 generation and unchanged source/case roots. Verifier-only bootstrap
installs authentic cached uv 0.9.5 and pytest 8.4.1 / pytest-json-ctrf 0.3.5
wheels after the agent exits; it does not change official tests or their expected
answer, the agent environment, model, budget, or timeouts. Raw failures remain
archived. Full parent and child oracle/nop calibration is required for readiness.

Current deterministic evidence: 26 focused unit tests, 20 fresh offline fixture
controls, and eight fresh Harbor child oracle/nop controls passed. The initially
permissive checker failed the nop/negative/input-integrity controls before the
minimal correction. Diagnostic image manifest:
`sha256:eddb05e0cc7ad8c56bfebeb2876686ce8c2cf6f37ccff767de6ae5521cc11ab4`.

Clippy and `evals/promptfoo/score.sh test` exited 0. Clippy dynamically acquired
slot 1; `CARGO_TARGET_DIR` and `CARGO_BUILD_BUILD_DIR` were both
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`, with
`CARGO_BUILD_JOBS=1`. The owned lock was released; cache was retained. There
were no Rust changes or Rust behavior tests. Docker preserves the official
storage declaration but does not enforce a writable-layer quota.

Final acceptance (2026-10-27): all ten preflight controls passed with oracle=1,
nop=0 for the untouched parent and each of the four children. The parent
produced 79586 and its original pytest collected and passed one test.
`verify_preparation` accepted the complete source/runtime/scoring receipt.
The actual parent tokenizer SHA256 matches the pinned diagnostic tokenizer.
No model requests, scoring changes, agent-environment changes or budget changes
were made. Raw results and exact command/exit-code evidence are archived in
`diagnostics/count-dataset-tokens/acceptance.json`. The stopped preliminary
package-download probe is preserved as an interrupted, unscored probe.

Successful preflight command (exit 0):

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/count-dataset-tokens/runtime.json --parent-id count-dataset-tokens --output evals/promptfoo/results/qwen38-suite/preflight/count-dataset-tokens-valid-runtime
```

Next: include these five independently scored cases in the full-suite freeze.
