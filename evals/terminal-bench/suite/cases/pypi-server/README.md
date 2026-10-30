# pypi-server diagnostics

Four independent synthetic tasks exercise real programs and artifacts. No
upstream solution or hidden test content appears in agent-visible fixtures.

- m1: implement `tinyvec.norm2`; check empty, signed, fractional and large integer
  inputs in isolated interpreters, including input preservation.
- m2: build `coppervec==0.4.2`; validate wheel metadata and every RECORD hash and
  size, install offline into a fresh target, then call its installed API.
- m3: serve a sandbox index on a supplied loopback port; fetch the exact index
  and wheel bytes and require HTTP 404 for an absent project.
- m4: use pip to install `ambervec==0.4.2` through a verifier-owned live index;
  require observed index/wheel GETs, installed metadata and numerical behavior
  from an interpreter outside candidate source.

The supplied wheels are deterministic pure Python distributions. Each task has
one gold, one nop and three meaningful negative implementations. The initial
always-accept checker failed 16 rejection assertions; the final checker passed
all 20 behavioral candidates. Host verification uses its own pip; the initial
Harbor venv pip repair used offline `ensurepip`, without changing the checker.

## Calibration — 2026-10-31

Standard `suite.run preflight` completed the original parent and all four fresh
children through `suite.scoring.harbor_trial` and
`suite.docker:SuiteDockerEnvironment`: all five oracle rewards are 1 and all
five nop rewards are 0. Every first verifier result succeeded in producing the
expected reward; no verifier recovery or environment variant was used.
`preparation.json` binds the unchanged official task files, diagnostic hashes,
frozen generation-2 runtime, execution identity, scoring logs and all 10 results.
It was verified against current inputs with `verify_preparation`.

Execution identity:
`588690e423e955cab7b2c335d302678cae81598b2ec421946065c88f964ce16c`.

The original amd64 image `alexgshaw/pypi-server:20251031` was fetched by Registry
API, with the anonymous token kept in memory. All compressed SHA256 values and
full streamed diff_ids were verified before OCI import. The local name was
bound to the verified manifest; actual parent containers used that same image.

- Manifest: `sha256:d18bb30f47c7dcaa3acdbc31ac7413c98eadca2ea16540bbd380f2f063b95276`.
- Config: `sha256:14bdb79aabffeb5b4635c42038a4eb7de3f4935f49d0bcd53dd9e726b1d6d1b2`.
- Compressed layers C: 83,717,494 bytes; complete uncompressed tar U:
  214,130,688 bytes, checked under a positive 1GiB hard decompression cap.
- Conservative initial reservation: three compressed copies, U cap, official
  S10GiB, B2GiB, A1GiB and 5GiB margin = 20,652,247,138 bytes. Actual initial
  host free was 42,151,325,696 bytes and VM free was 56,257,417,216 bytes.
  Runtime host disk samples and actual cgroup inspections are retained.
  After both parent controls, remaining diagnostics separately passed their
  9GiB cached-image reservation against fresh host and VM samples.

Parent containers retained official 1CPU/2048MiB and both 900-second timeouts.
Children used fresh 1CPU/512MiB main containers and 0.25CPU/64MiB egress sidecars.
Storage is an admission reservation, not a claim of writable-layer quota.
Container-side finite reads reached Debian, PyPI, astral.sh and the uv GitHub
release. Python 3.13.7, pip 25.2 and the frozen Linux ELF `--help` ran successfully
in the actual parent image. ELF SHA256:
`847823c5cd99a6828cddfc2e3e5dbb67c5ddcc9f8693a13d539d8a8fa47c9dae`.
The earlier diagnostic base also passed its offline, read-only ELF probe.
Installed system and pypiserver dependency versions and file hashes are archived.

## Checks and evidence

All of these checks passed again on 2026-10-31:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/pypi-server/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/pypi-server/case.json --run-fixtures
evals/promptfoo/score.sh test
cargo clippy --all-targets --all-features -- -D warnings
```

Host: 5 tests / 20 behavioral candidates. Docker fixtures: 20 checks (4 gold,
4 nop, 12 negatives). Clippy: exit 0, zero behavioral tests; dynamic slot 3,
`CARGO_TARGET_DIR` and `CARGO_BUILD_BUILD_DIR` both
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-3`, JOBS=1.
The owned slot lock was released and its compilation cache retained.

Current raw evidence is ignored under
`evals/promptfoo/results/qwen38-suite/diagnostics/pypi-server/resume-20261031/`:
`final-evidence.json`, `admission.json`, `layers-verified.json`, image import and
network/ELF probes, dependency hashes, resource samples, test logs and command
exit receipts. Full production trials and scoring logs are under
`evals/promptfoo/results/qwen38-suite/preflight/pypi-server/`.
An interrupted infrastructure attempt is retained separately as
`preflight/pypi-server-monitor-interrupted-20261031/`; its resource watcher raced
an unrelated container removal. The watcher was narrowed to this task, and the
complete fresh preflight was rerun. An earlier launcher-path error is also
retained separately. Neither failed attempt is counted as calibration evidence.

No paid model was called. The fixed model and budgets remain unchanged. Next:
freeze the prepared group before a separately authorized single model run.
