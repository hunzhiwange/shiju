# chess-best-move diagnostics

Four independent synthetic artifact tasks; no model requests are needed for
calibration. The official parent package remains unchanged.

- m1 reads a newly generated PNG board with colored piece markers and an RGB
  legend. It writes every occupied square and piece identity. Rank inversion,
  omitted pieces and wrong piece identities are rejected. The orientation is
  explicit, so this measures coordinate recognition without requiring a font.
- m2 enumerates the legal destinations of one knight in a synthetic complete
  FEN. Friendly captures, omitted enemy captures and off-board destinations are
  rejected. Other pieces' moves are outside this narrow task.
- m3 classifies five independent FEN/move pairs for legality and resulting
  check: a pinned rook, capturing a checking rook, castling through attack,
  en passant exposing the king, and promotion giving check. Negative controls
  violate each of the three king-safety boundaries. Expected values were also
  independently checked with python-chess 1.999 / chess 1.11.2, with all five
  positions validated as legal starting positions.
- m4 formats supplied move records as sorted, unique UCI strings. It preserves
  promotion suffixes and rejects SAN, missing promotion and duplicate output.

Each checker reads the actual output artifact, never the final answer or an
agent self-test. Expected data lives only in evaluator-only oracle metadata.
Malformed JSON, duplicate keys, wrong boolean types and extra/missing values
fail. Each child has gold, nop and three distinct negative artifact controls.
No official position, image, solution prefix or model trajectory was copied
into a synthetic fixture.

The base is the pinned upstream Python image in `case.json`; the local
`environment/Dockerfile` documents that dependency and installs nothing. It
contains no parent data, gold, negatives or checker. Only each child's listed
public inputs enter its generated image. Harbor injects gold only for oracle
calibration and checker/oracle metadata only in the verifier phase. Fixture
checks use fresh offline containers, 1 CPU / 512 MiB, read-only root, dropped
capabilities and a 128-process limit. Harbor children also have independent
containers, 900-second agent / 120-second verifier limits and the frozen
24-request / 32-tool, 32000-output / 200000-input, one-attempt policy.

Run from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/chess-best-move/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/chess-best-move/case.json --run-fixtures
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/chess-best-move/runtime.json --parent-id chess-best-move --output evals/promptfoo/results/qwen38-suite/preflight/chess-best-move
```

Preflight output must be a new directory. The original results/runtime.json
is historical identity evidence rather than scheduler configuration. The
case-local ignored runtime selects the calibrated generation-2 runtime and
keeps all parent resources, timeouts, source and scoring intact. A checksum
verified verifier-only dependency bootstrap supplies uv 0.9.5, Python 3.13.7
and the official pytest 8.4.1 / pytest-json-ctrf 0.3.5 dependencies after the
agent exits, under `/opt/vw-verifier-deps`. Official test.sh is unchanged.
The bootstrap script here contains no task artifact or expected answer.

Raw commands, failures, dependency hashes, image identities, resource limits
and timestamps are retained under the ignored results directories
`diagnostics/chess-best-move/` and `preflight/chess-best-move/`.
The initial intentionally permissive checker produced 12 negative-control
failures before the exact artifact comparison was implemented. Unit checks
cover 20 declared artifact controls plus seven malformed/typed controls.
Completion requires 20/20 fixture controls and all ten Harbor oracle/nop
controls; successful compilation alone is insufficient.

Final calibration: 20/20 fixture controls and 10/10 fresh Harbor controls
matched their expected outcomes. The parent oracle passed its one official
pytest test; nop failed that same test because the output file was absent.
`verify_preparation` accepted the complete receipt, including original task
hashes and verifier-only dependency hashes. All validation commands exited 0
except the intentional checker red phase and the retained, repaired probe /
host dependency-install attempts. No model calls or official model scores
were produced. Clippy exited 0 using dynamically claimed slot 1, with both
Cargo target/build directories set to the absolute repository path ending in
`target/rust-slot-1` and one build job. Its lock was released and cache kept.
The backend preserves the official storage declaration but does not enforce a
writable-layer disk quota; actual memory/CPU limits and Harbor egress sidecar
configuration are archived separately. Full evidence is in the ignored
`diagnostics/chess-best-move/acceptance.json`.
