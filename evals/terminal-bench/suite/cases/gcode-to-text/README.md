# gcode-to-text independent diagnostics

Suite: Terminal-Bench independent diagnostics; preparation batch 23.
These four standalone tasks use synthetic G-code and an invented pixel alphabet.
No parent inputs, reference-solution prefixes or model traces enter diagnostics.
The public API is `python3 /app/solution.py INPUT.gcode`; stdout is the observable
result. Each checker executes the program on multiple fresh inputs. Coordinates
and strokes are checked against hand-derived numeric arrays, including shape
and numeric types; recognition requires exact uppercase text. Hidden probes,
gold programs and checker are injected only in the verifier phase.

| Child | Behavior | Three rejected programs |
|---|---|---|
| m1 | Absolute XYZ / relative extrusion segments, omitted axes, comments, Z motion | Draws travels; resets omitted axes; draws retractions |
| m2 | Ordered stroke points split at every non-drawing move | Joins strokes; loses start points; reverses points |
| m3 | Independent XYZ/E modes, G92 resets, travels and retractions | Ignores relative XYZ; ignores resets; always relative E |
| m4 | Recognizes different words using a public 5x7 alphabet | Reverses text; lowercases; hardcodes the example |

A deliberately incomplete checker tested only the example and accepted m4's
hardcoded example program: the 20-test red run failed that assertion. Testing
all three words rejects it. The final checker uses two inputs for each geometric
task and three words for recognition. Gold, nop and twelve wrong programs run
in twenty fresh offline fixture containers. The geometric arrays and word
expectations are independently specified, not inferred from candidate output.

The tools-only diagnostic image is the existing pinned
`python@sha256:fcf02c9e248b995ae2ca9aac6fa24b489f34b589dbfdc44698ad245d2ad41d1e`
(linux/arm64). Its `/app` is empty; no task, oracle or test data is baked in.
`environment/Dockerfile` documents that base. Fixture isolation: 1 CPU, 512MiB,
128 PIDs, dropped capabilities, no network, read-only system filesystem and
64MiB tmpfs. The only bind mounts are the temporary candidate directory and
read-only verifier material. Harbor controls use fresh containers, no network,
1 CPU, 512MiB, 1024MiB requested storage, 900s agent / 120s verifier. Requested
storage is not a claim of Docker writable-layer quota enforcement.

The official parent is unchanged: 1 CPU, 2048MiB, 10240MiB requested storage,
900s agent / 900s verifier. Original image:
`alexgshaw/gcode-to-text@sha256:0979ef40c6a3e8c4e7ab5b6c2524c74625a84799e15cd45dd98dcf83b11efd4d`
(linux/amd64). Docker's registry connection timed out; recovery used the same
Registry manifest and SHA256-verified compressed layers, verifying every
uncompressed rootfs diff ID before OCI import. Shared cached layer bytes were
reused only when their hashes matched that manifest. The official two-test
pytest and original oracle execute in the original environment.

`environment/verifier-bootstrap.sh` supplies authenticated, pinned curl,
uv 0.9.5 and pytest 8.4.1 / pytest-json-ctrf 0.3.5 dependency caches only after
the agent exits. The task-local runtime records every dependency SHA256. The
script keeps real apt-get operations and failure codes, with 30-second bounds;
it does not change the official checker or agent environment. No artifact
replay or recovered-score substitution is permitted for parent calibration.

Policy stays token-plan/qwen3.8-flash, single output 32000, input window 200000,
24 requests / 32 tools and one attempt. Preparation makes zero model requests.
Calibration evidence and infrastructure failures are separate from model scores.

Run from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/gcode-to-text/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/gcode-to-text/case.json --run-fixtures
```

Raw logs, input/image/dependency hashes, resource receipts and local runtime
configuration are under ignored
`evals/promptfoo/results/qwen38-suite/diagnostics/gcode-to-text/`.
The requested preflight using global `runtime.json` fails with
`KeyError: historical_root`; this is an identity report. Actual preflight uses
the task-local `runtime-bootstrap.json` bound to calibrated runtime-v2.
`validation.json` binds all diagnostic/checker bytes; `preparation.json`, when
present, binds ten fresh Harbor controls and the unmodified official source.
