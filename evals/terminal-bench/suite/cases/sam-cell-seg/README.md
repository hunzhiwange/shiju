# sam-cell-seg diagnostics

Four standalone synthetic Python exercises accompany the unchanged official
Terminal-Bench 2.1 `sam-cell-seg` task:

- m1 parses mixed comma/JSON coordinate fields, preserves row and point order,
  forms xyxy boxes and computes polygon area.
- m2 executes the official MobileSAM `vit_t` CPU predictor and checks actual
  IoU scores, sampled low-resolution logits and mask pixel counts against
  separately measured `predict_torch` controls. Three hidden box prompts use
  the public synthetic 32×48 RGB image. Empty masks on this synthetic image
  are valid measured results; the logits and scores remain discriminating.
- m3 resolves positive-logit ownership before retaining each cell's largest
  four-connected component, with explicit pixel/component tie rules.
- m4 updates CSV geometry by cell ID through arbitrary CLI paths, preserving
  all extra columns and row order with exclusive bounding-box maxima.

Each child has a complete instruction, public synthetic examples, an executable
reference program, a missing-program nop, three semantic wrong programs and
independent hidden controls. m2 negatives omit image initialization, use the
wrong box or fabricate zero predictions. Other negatives cover xy order,
rectangle omission, bounding-box area, disconnected islands, overlap, pixel
ownership ties, inclusive maxima, positional row association and lost columns.
The checker executes candidates, reads their actual outputs, and rejects
malformed types, missing files, nonzero exits and timeouts. No Agent self-tests,
source keywords, parent solution prefixes or prior model transcripts are used.

## Model and tool environment

The image is pinned in `case.json`:
`vw-sam-diagnostics:task85-single@sha256:ae5f6b72b92a255343868f7d9d7274548a9ccd6033d4e773eedf6da58ab93548`.
It derives from the reviewed clean amd64 tool leaf
`32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf`.
Only generic libraries, official MobileSAM source and its public weights were
added. No original task input, solution, test or answer entered the tool image.

`environment/tool.Dockerfile` records the executed constrained build recipe.
Download the two files named in `environment/tool-sources.json` from their
pinned official URLs into a disposable build context, verify their complete
SHA256 values, and place that Dockerfile alongside them. Build with
`DOCKER_BUILDKIT=0 docker build --memory 1073741824 --memory-swap 1073741824
--cpu-period 100000 --cpu-quota 100000`. Perform capacity admission first.
`environment/requirements.lock` inventories the resulting installed versions;
MobileSAM is installed from the checked official archive, not from PyPI.
The image digest and archived distribution RECORD hashes freeze the complete
installed closure, including transitive packages resolved during this build.
The small `environment/Dockerfile` documents the final child base;
`suite.diagnostics` materializes only declared public fixtures.

Source commit: `34bbbfdface3c18e5221aa7de6032d7220c6c6a1`.
Source archive SHA256:
`c4b85466ae0a1826433d8290998d3b10704a2f4ba5da399e8c7d9cb46de7580e`.
Weights SHA256:
`6dbb90523a35330fedd7f1d3dfc66f995213d81b29a5ca8108dbcdd4e37d6c2f`.
Diagnostic versions include PyTorch 2.6.0+cpu, torchvision 0.21.0+cpu,
timm 1.0.19 and NumPy 2.2.6 on Python 3.13.7.
The original parent retains its official PyTorch 2.5.1 and Python 3.11 setup.

Ordinary FP32 inference and an allocator/MKLDNN adjustment both OOM-killed at
512MiB; FP16 alone exposed MobileSAM's explicit float prompt coordinates.
The declared gold uses one thread, FP16, inference mode, allocator trimming and
CPU autocast around `predict`, preserving unmodified MobileSAM source and the
1024-pixel encoder. Real inference succeeded within the original 512MiB/120s
limits. The prompt documents this reproducible CPU recipe. No GPU, replacement
SAM, prerecorded mask or relaxed budget was used.

## Verification and provenance

All raw logs and unique OCI/source archives remain under the ignored directory
`evals/promptfoo/results/qwen38-suite/diagnostics/sam-cell-seg/`.
This includes all OOM/dtype failures and the initial RGB-reversal negative,
which was removed because its sampled FP16 outputs were indistinguishable.
Expected values and tolerances were unchanged when replacing that fixture
with the missing-image initialization error.

An accept-all checker produced 12 failures in 21 unittest checks before the
behavioral checker was implemented. The final unittest run passed 21 checks (20 real Linux
controls plus schema), and the separate validate command passed 20 fresh
Linux controls. The standard preflight passed all 10 oracle/nop trials:
parent and m1..m4 each scored exactly 1/0. The parent oracle passed all nine
official tests. All three commands exited 0, as did `score.sh test` and Clippy.
`inputs` and `verify_preparation` subsequently verified the complete receipt,
including all ten main-container inspects and eight sidecars. No resource
monitor violations were observed. No paid model calls were made.

`validation.json` is portable hash-bound calibration evidence. The generated
`preparation.json` is ignored because it contains machine-specific evidence
paths; the canonical copy remains under the ignored preflight output. Its
execution identity, original scoring files and all ten rewards were rechecked.
The live parent checkpoint hash and all 16 MobileSAM source files matched the
pinned official archive and the diagnostic image. Both earlier preflight
startup failures (historical report and observer interpreter path) remain in
the raw evidence; neither changed the shared runtime or official task.

The original manifest is
`sha256:d76bdeef19d8113f4094013249c982eca93e62e46130484846269b2d378879d0`;
config `sha256:792f56b9413a280dfbeb0c5b64067e4f7217f0fba16cd84918eb28928f406fd2`.
All eleven compressed-layer SHA256 values and decompressed diff_ids matched.
Compressed C=411493212 bytes; exact U=1135279616 bytes. Downloading used a
positive 4GiB full-stream U cap; import streamed verified parts without an
additional archive. The tool archive is 802131456 bytes, with complete layer
verification and exact U=2187352576 bytes under a positive 3GiB cap.
Admission accounted for every retained compressed/archive copy, S/B/A/safety
reservations, both host and Docker VM free space, and the then-active peer's
26GiB future-growth reservation. The scheduler subsequently released that
reservation. Original resources/timeouts and diagnostic policy are unchanged.
Storage is admitted and monitored; these receipts do not claim a writable-layer
quota unsupported by the Docker backend.

The frozen generation-2 ELF ran offline `--help` without credentials in both
the exact original parent image and the diagnostic image. The standard backend
is `suite.docker:SuiteDockerEnvironment`, execution identity
`ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1`.
The original package's complete file tree matches the frozen catalog.
Foundational catalog/contract/runtime/adapter checks passed 48 tests.
Clippy passed using dynamically claimed slot 1, with both `CARGO_TARGET_DIR`
and `CARGO_BUILD_BUILD_DIR` set to
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`, jobs=1;
the owned lock was released and cache retained. No Rust files changed.

From the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/sam-cell-seg/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/sam-cell-seg/case.json --run-fixtures
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/sam-cell-seg/runtime.json --parent-id sam-cell-seg --output evals/promptfoo/results/qwen38-suite/preflight/sam-cell-seg-current
```

The plan's public `results/qwen38-suite/runtime.json` is an immutable historical
report, so its literal preflight command failed with missing `historical_root`.
The case-local ignored execution config supplies the verified generation-2
runtime, absolute roots and original 7200-second parent timeout. The historical
report and `plan.md` remain unchanged. Oracle/nop rewards are calibration;
model policy remains token-plan/qwen3.8-flash, 32000 output tokens, 200000 input
tokens, one attempt. No readiness for the other 88 parents is inferred.
