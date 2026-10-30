# PyTorch model CLI — calibrated evaluation preparation

Status: preparation calibrated on 2026-10-31. **Framework LLM requests: 0.**
Oracle/nop rewards below check the environment and grader; they are not framework
or model capability scores. The actual framework evaluation belongs to task 165,
after an experiment manifest is frozen. No paid attempt was started.

The four independent synthetic Python diagnostics cover:

- m1: flat weight-array counts and row-major normalized grayscale PNG pixels.
- m2: a small affine/ReLU/affine inference chain, including alternate weights.
- m3: normalized image preprocessing, bias, shapes and command-line errors.
- m4: ten-logit argmax, lowest-index ties, exact stdout and prediction-file overwrite.

PNG contracts explicitly use 2x2, 8-bit grayscale, noninterlaced, filter-zero
images. Each child has separate public inputs, a complete instruction, a gold,
a nop and three meaningful incorrect implementations. Hidden checks execute the
actual programs on multiple independent inputs. These tasks do not replace the
parent's compiled C/MNIST task or contain its reference solution or test data.

Fresh checks passed: 21 host unittest checks, 20 offline Docker fixtures
(4 gold, 4 nop, 12 negatives), and ten standard production-backend controls
(parent and m1..m4, each oracle=1/nop=0). The original parent oracle passed all
six official tests, including real C inference on 50 MNIST images; the original
nop failed all six. First scores were used, with no artifact replay or verifier
retry. Eight earlier standard child controls also passed and remain separate.
An earlier accept-all checker mutation failed 16 controls; historical red logs
are preserved. `validation.json` binds the declared diagnostic files and checker;
`preparation.json` binds the ten standard controls and execution identities.

The unchanged official parent image is `alexgshaw/pytorch-model-cli:20251031`:
manifest `sha256:c996884b5fb5fb2c0c07d67da2c4f974f271f07234ed28e1a868240375c57a67`,
config `sha256:2d19236650cc866ba441762e3d89edfa452756e0e4833cfdb984257d50b446e7`.
All six compressed digests and uncompressed diff_ids were verified:
C=44,342,024 and U=125,624,320 bytes, within the positive 1GiB pre-download U cap.
The original instruction, input PNG/PTH, libraries, reference solution, tests,
1 CPU/2048MiB/10240MiB/GPU=0 and 900-second timeouts remain unchanged.

The shared, tool-only child image is pinned in case.json and Dockerfile:
`vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf`.
Its `/app` is empty, with no `/tests` or `/solution`; tool inventories and layer
identities were inspected. The frozen runtime-v2 Linux ELF passed offline,
read-only `--help` in both original parent and child images. All ten standard
main containers used the declared resource limits, were unprivileged and had
no host mounts. All eight child main containers shared their standard
0.25 CPU/64MiB egress sidecar namespace, initialized with `no-network` and an
empty allowed-host list. Docker bridge metadata is retained without claiming
that these bridges use Docker's `Internal` flag. Fixture containers were offline and read-only.

The first official verifier could not finish Debian HTTP downloads, the fixed
uv installer URL returned 403, and a subsequent MNIST download failed with a
TLS EOF. `environment/verifier-bootstrap.sh` repairs dependencies **after the
agent exits**: Debian repositories use HTTPS; SHA-bound original uv 0.9.5
installer/release caches satisfy the original installer request; four authentic
MNIST files are checked against the pinned torchvision 0.22.1 resource MD5s and
fill missing verifier dataset files. Candidate executable, weights and prediction
hashes are verified unchanged. The original test still creates MNIST PNGs and
executes the real candidate binary. No expected predictions or task outputs are
injected. Bootstrap policy, file hashes and output are in the preparation receipt.

The task's prescribed historical `runtime.json` is an identity report, not an
execution configuration (`historical_root` is absent). Fresh preflight uses the
verified runtime-v2 through the task-local, ignored `runtime-20261031.json`.
The final scheduler SHA is
`e5e2bd2b537679a3f10a63d88710ec7992eb510fcbe82620fa435cf10798e9a0`.

Reproduction from repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/pytorch-model-cli/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/pytorch-model-cli/case.json --run-fixtures
PYTHONPATH=evals/terminal-bench DOCKER_BUILDKIT=0 COMPOSE_DOCKER_CLI_BUILD=1 evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/pytorch-model-cli/runtime-20261031.json --parent-id pytorch-model-cli --output /absolute/new-preflight-directory
```

`evals/promptfoo/score.sh test` and required
`cargo clippy --all-targets --all-features -- -D warnings` exited 0.
Clippy dynamically claimed slot 5, with both CARGO_TARGET_DIR and
CARGO_BUILD_BUILD_DIR at
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-5`, jobs=1,
0 Rust behavior tests. Its owned lock was released. Subsequent exclusive,
byte-verified APFS compression retained cache paths, contents, modes and mtimes.
No Rust source, default Promptfoo configuration or baseline was changed.

Raw successes, failed controls, resource/disk/network receipts, source and
package provenance, cache-compression receipts and dependency caches remain in
ignored `evals/promptfoo/results/qwen38-suite/diagnostics/pytorch-model-cli/`.
The successful full standard run is `preflight-20261031-seventh/`;
`final-evidence-20261031.json` summarizes calibration with zero LLM requests.
A topic-capacity stop, disk-admission rejections and an auxiliary monitoring race
were retained separately; none is counted as a model result. Task-owned trial
containers and monitoring processes were cleaned up; shared caches were retained.
