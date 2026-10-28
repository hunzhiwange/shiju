# SQLite / gcov diagnostics

Four independent tasks use real SQLite 3.50.4 and synthetic SQL/probe inputs.
They keep the suite's fixed Qwen3.8-Flash 32000/200000 policy, 24 requests /
32 tools, 900-second agent and 120-second verifier limits, and one attempt.
No model requests were made during preparation.

- **m1** preserves a fixed source snapshot, compiles a synthetic version probe,
  and validates actual GCC/gcov output. Mutations alter source, omit coverage,
  or omit compilation.
- **m2** compiles the real SQLite CLI with coverage on both translation units
  and linking. The checker executes two query workloads and parses gcov's JSON
  for executed SQLite source lines. Mutations omit instrumentation, emit notes
  without runtime counters, or supply a fake program.
- **m3** runs arbitrary SQL files against fresh database paths with coverage
  relocation. Hidden synthetic workloads check rows, stdout, fresh counters,
  and SQL error propagation. Mutations ignore inputs, discard coverage, or
  swallow SQLite errors.
- **m4** installs the existing instrumented CLI through PATH. The checker
  resolves the executable, runs it from another directory, and parses fresh
  gcov counters. Mutations use the wrong directory, a broken link, or a fake
  executable.

The image contains only the reviewed pure tool dependency and public SQLite
amalgamation source/binary/coverage notes. It has no official parent solution,
tests, model trajectories, diagnostic gold or oracle metadata. The Dockerfile
pins its tool base and verifies the public source ZIP SHA256. SQLite is public
domain; its original notices remain in sqlite3.c, shell.c and sqlite3.h.
Source: https://www.sqlite.org/2025/sqlite-amalgamation-3500400.zip

The checker binds source, binary and matching coverage-note hashes in hidden
oracle metadata. It validates those dependencies before and after candidate
execution. Immutable public inputs cannot redefine expected outcomes. Gold and
three meaningful incorrect implementations per child are injected separately;
no official parent answer prefixes are used. The candidate scripts are invoked
by the independent checker, rather than accepting agent self-test reports.

Direct fixture controls use fresh offline containers (1 CPU, 512 MiB), with a
read-only root filesystem and no repository/parent mounts. Standard Harbor
children use `suite.scoring.harbor_trial` and its common resource-limited
`SuiteDockerEnvironment`; no private platform, Compose, or Trial override.

Preparation evidence is local under
`evals/promptfoo/results/qwen38-suite/diagnostics/sqlite-with-gcov/`.
All 20 direct fixture controls and all ten standard Harbor controls passed
calibration: each oracle receives reward 1 and each nop reward 0. The original
parent verifier executed three tests successfully for the oracle and three real
failures for nop. `suite.preparation.verify_preparation` accepts the final
machine-local `preparation.json` (ignored by Git).

The first parent oracle failed on an apt HTTP 502 before compilation; its raw
reward and infrastructure classification remain archived. A later unchanged
oracle passed after actual index GET recovery. The single preflight then hit a
shared admission lock before starting nop. Only that unstarted nop was rerun;
the final receipt binds its real score, the successful oracle, and the eight
fresh child controls after verifying identical package bytes and execution
identity. The failed preflight was retained; no score or task artifact was
replayed and no successful oracle was repeated.

Final Clippy and Promptfoo configuration checks passed. Preparation made no
paid model calls and does not constitute a model capability score. The local
`acceptance-final.json` binds scores, checker/fixture hashes, image identities,
resource evidence and the final preparation receipt.
