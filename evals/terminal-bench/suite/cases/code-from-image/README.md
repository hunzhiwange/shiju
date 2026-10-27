# code-from-image diagnostic preparation

This case preserves the frozen official task, image, instruction, input image,
reference solution, two official tests, resources (1 CPU, 2048 MiB, 10240 MiB
storage), and 1200-second agent/verifier limits. No paid model calls are made.

The four independent synthetic diagnostics have complete public contracts:

- m1 reads a generated pseudocode PNG and produces an iteration trace plus the
  final printed integer. Negative controls confuse an inclusive bound, parity,
  or initial index. The PNG was visually reviewed; no parent image or answer
  is reused.
- m2 implements a weighted modulo fold through a stdin/stdout JSON API. Six
  subprocess probes cover empty, singleton, wraparound and negative inputs.
  Negatives use zero-based weights, omit reduction, or drop the final item.
- m3 implements an index scan with a negative sentinel and zero-induced skip.
  Nine subprocess probes cover empty input, start at length, first/late
  sentinel, skipped sentinel, and advancing beyond length. Negatives use an
  inclusive bound, omit the skip, or record the sentinel before stopping.
- m4 supplies an executable byte-hashing program and its sample output. The
  checker also executes the program on six new binary inputs and arbitrary
  paths containing spaces, including empty and non-UTF8 files. Negatives all
  carry the correct sample output but confuse digest bytes with hex, include
  the slice endpoint, or read binary data as text. Static output alone fails.

The existence-only checker initially accepted all 12 semantic negative
controls. The fixed checker rejects them, nop, duplicate JSON keys, wrong
JSON types and a static program. Two unit tests run 23 candidate controls.
Twenty additional controls run in fresh offline containers with one CPU,
512 MiB, read-only root, no capabilities, no-new-privileges and a 128-process
limit. Only explicitly declared public synthetic fixtures enter diagnostic
images. Hidden checkers, oracle inputs and gold are injected in their proper
Harbor phases. The diagnostic image is pinned in case.json and needs no
additional dependencies.

The official image is
`alexgshaw/code-from-image@sha256:2be9cdb746ed18c1b2b1404032efae0b2235a1b8fc479ef17730c95409d89f76`
(linux/amd64). Registry manifest/config/compressed layer hashes, decompressed
diff IDs, local layer chain and frozen source hashes are verified in ignored
`diagnostics/code-from-image/image-provenance.json`. The exact image was
started on ARM64 with explicit amd64 platform, 1 CPU and 2048 MiB; /app
contained only code.png, whose hash matches the frozen input. Host and VM
space exceeded the conservative reservation of 19,585,230,113 bytes (three
compressed copies, uncompressed layers, 10 GiB task storage, 2 GiB tool cache,
1 GiB evidence and 5 GiB headroom). The storage declaration is retained;
Docker does not enforce a writable-layer disk quota.

Container network probes reached PyPI and Debian bookworm. The uv installer
returned HTTP 403 through Python's HTTP client. Verifier-only provisioning
supplies checksum-pinned uv 0.9.5 and dependency wheels after the agent exits,
using the existing verified cache. No task, input, oracle or official scorer
bytes change. The original test.sh still runs its installer and pytest;
uv's archive and wheel resolver can use the verified local cache. The root
results/runtime.json is historical identity evidence and lacks scheduler
fields; the case-local runtime below uses the calibrated generation-2 runtime.

Run from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/code-from-image/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/code-from-image/case.json --run-fixtures
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/code-from-image/runtime.json --parent-id code-from-image --output evals/promptfoo/results/qwen38-suite/preflight/code-from-image
```

Preflight requires a fresh output directory. Raw commands, timestamps, image,
dependency/checker/fixture hashes, isolation/resource observations, TDD failure
and calibration receipts remain in the ignored diagnostics/preflight folders.
Model, token limits, request/tool budgets and one-attempt policy stay frozen.

Final evidence: the original parent oracle passed both official tests and nop
failed both. All ten fresh Harbor oracle/nop controls matched their expected
rewards, and verify_preparation accepted the complete receipt. The 20 fresh
fixture controls and 23 local candidate controls passed; Promptfoo configuration
validation exited 0. Required Clippy exited 0 in dynamically claimed slot 1,
with CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR set to
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1` and one build
job. The owned lock was released and cache retained. The existing block 0.1.6
future-compatibility notice remains. Exact command exits, timestamps, evidence
hashes and resource observations are in ignored
`diagnostics/code-from-image/acceptance.json`. Next: include this prepared case
in the full-suite freeze before separately authorizing paid evaluation.
