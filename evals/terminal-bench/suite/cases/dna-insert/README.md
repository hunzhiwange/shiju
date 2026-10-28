# dna-insert diagnostics

Four independent synthetic tasks isolate circular insertion localization (m1),
small primer candidate enumeration (m2), forward/reverse-complement constraints
(m3), and circular target reconstruction with byte-level digest verification (m4).
No parent sequence, reference solution, hidden test, or model trajectory enters
public fixtures. m2 explicitly uses the synthetic Wallace rule and a deterministic
minimum-length tie-break; it does not substitute for official oligotm scoring.
Each child states its full input/output, coordinate, orientation and budget contract.

The checker validates actual result.json files, complete typed structures, unchanged
input SHA256, duplicate keys, regular-file boundaries and output size. Each child
has gold, nop and three meaningful negative controls. The independent
`oracle/verify_fixtures.py` recomputes each gold and enumerates the unique m1 answer.
A permissive checker first failed 27 rejection controls across 25 tests; the final
checker passed all 26 tests including independent oracle auditing. The initial
m1 ambiguity found by auditing was fixed before the final fixture calibration.

Only declared public inputs enter fresh environments. Oracle, checker and gold
are injected in verifier/solution phases. Offline fixture calibration uses 1 CPU,
512MiB, no capabilities, no-new-privileges, 128 PIDs, read-only root and a bounded
tmpfs. Harbor children retain 900/120-second agent/verifier limits. The unchanged
parent retains 1 CPU, 4096MiB, 10240MiB declared storage, and 1800-second limits;
Docker does not enforce writable-layer storage quotas. Model/output/input policy
remains token-plan/qwen3.8-flash, 32000/200000, one attempt; preparation makes no
model calls.

Commands from repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/dna-insert/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/dna-insert/case.json --run-fixtures
python3 evals/terminal-bench/suite/cases/dna-insert/oracle/verify_fixtures.py
```

Raw commands, failures, source/digest/dependency identities, isolation and resource
receipts are retained under ignored diagnostics/dna-insert and preflight directories
in evals/promptfoo/results/qwen38-suite. The historical runtime.json lacks
historical_root; the ignored per-case runtime selects the verified existing v2
runtime with unchanged source/case roots and parent timeout.

The Clippy gate `cargo clippy --all-targets --all-features -- -D warnings` exited 0.
The final 2026-10-28 gate dynamically acquired slot 1; CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR were
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`, jobs=1.
The owned lock was released and cache retained. No Rust sources changed.
Promptfoo configuration validation exited 0. These static gates do not establish
parent or diagnostic scoring readiness.

All 20 final fresh offline fixture controls and eight standalone fresh Harbor
child oracle/nop controls passed. The original image is linux/amd64 digest
`sha256:4dd8760694e355de85ecf03605cb487ccdb33322f6afad513d2920829baa33b2`,
config `sha256:2e68029a49d11b454994967b2eddb94333ef321b688f865ada3c09a3b534c110`.
Compressed layers total 29724629 bytes; uncompressed layers total 80645632 bytes.
The OCI import verified compressed SHA256, uncompressed diff_ids and local layer
chain. Two identical, verified base layers were read from the existing local
preparation cache; only this task's output directory was written. Actual startup
reported x86_64, memory.max=4294967296, cpu.max=100000/100000 and the expected
7238-byte public sequence file. The original source tree remains frozen at
`a88b1748a1857e70022d95dcbb39ca0c5bd813ae`.

Final preparation passed on 2026-10-28: all ten fresh Harbor controls completed
(parent and four children each oracle=1, nop=0), with current source and scoring
evidence bindings verified through verify_preparation. The official parent
oracle passed its one unmodified test_primers test; nop executed that same test
and failed because primers.fasta was absent. No tests were skipped. This is
calibration evidence, not a model capability score.

Before startup, host and Docker VM free space both exceeded the conservative
18.2GiB admission estimate (official 10GiB storage, cache/archive reserves and
5GiB margin retained). The original agent installed its own dependencies and
completed within its unchanged 1800-second timeout. There was no dependency
preinstallation, environment change, verifier recovery, model call or budget
increase. An earlier 1800-second apt timeout remains archived separately as
unscored/infrastructure; it is not included as an accepted score.

Passing preflight (exit 0):

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/dna-insert/runtime.json --parent-id dna-insert --output evals/promptfoo/results/qwen38-suite/preflight/dna-insert-recovery-20261028
```

Current acceptance, capacity, command/exit-code, Clippy slot and fresh calibration
receipts are retained under diagnostics/dna-insert as recovery-*-20261028 files;
recovery-final-20261028.json binds the final result. validation.json and
preparation.json bind this case's declared bytes and current execution identity.
All owned containers and the Cargo slot lock were released; caches are retained.
Include these five independently scored cases in the later suite freeze.
