# Caffe CIFAR-10 diagnostics

Status: all four children and the separately identified network compatibility
parent are calibrated. Original-network readiness remains a separate identity. These
synthetic standard-library Python tasks do not install Caffe or replace the
full parent's training and official accuracy tests.

- m1 reads synthetic planar CIFAR records and checks labels, dimensions and
  the ten-output classification boundary.
- m2 computes stable softmax/cross-entropy and one full SGD update.
- m3 executes an explicitly specified seeded sampling recurrence for exactly
  the requested number of updates, including zero iterations.
- m4 writes an exact binary float32 checkpoint, reloads its parameters and
  performs classification with a defined tie rule.

Each child is independently understandable. Only its instruction, synthetic example and
public contract enter the candidate environment. Gold programs, hidden controls
and the twelve semantic negative programs remain verifier/solution material.
The checker runs actual programs on multiple inputs, including malformed data,
overflow-prone logits, nonzero labels, repeat runs, stale output and float32
rounding. Host validation contains 20 candidate controls and one schema check.
An accept-all checker mutation causes sixteen failures. An initial precomputed
seed-zero expectation was incorrect (0.625 instead of 0.5); manual recurrence
recalculation corrected it before the final host run.

The base digest was inspected and started as linux/amd64 with Python 3.13.7;
/app was empty and no /tests or /solution material was present. validation.json
binds all 20 fresh offline Docker controls. Eight standard Harbor child trials
passed: oracle reward 1 and nop reward 0 for each child. Shared admission-lock
failures occurred before candidate execution and are archived separately.

The original upstream task is read-only. The previously verified original
CIFAR archive may accelerate only its exact public HTTP URL through a temporary
task-local loopback proxy in a separately recorded network compatibility run.
Both http_proxy and https_proxy are candidate-visible; HTTPS and non-target
traffic is relayed to the existing loopback HTTP proxy on port 7897.
Such a result must retain the original image, dependencies, solution, data,
500 iterations, resources, timeouts and unmodified official checker. It must
never be labeled an unchanged original-network score. The original archive
SHA256 is c4a38c50a1bc5f3a1c5537f2155ab9d68f9f25eb1ed8d9ddda3db29a59bca1dd.

No model requests have been made. Raw evidence resides in the ignored
`evals/promptfoo/results/qwen38-suite/diagnostics/caffe-cifar-10/` directory.

compat/cache_proxy.py validates the archive length, SHA256 and official MD5
before listening on a dynamic 127.0.0.1 port. Its real-socket tests cover exact
GET/HEAD scope, Range and other HTTP passthrough, CONNECT passthrough and
truncated or same-length corrupt data rejection. The complete host run has
23 passing tests. The canonical proxy also delivered all original archive
bytes with matching SHA256 in a real host transfer. Those controls are not
parent training scores.

The reproducible compatibility entrypoint is `compat/calibrate.py`. It requires
an unchanged catalog-matching upstream tree, the verified original archive,
and all eight current child receipts. It selects linux/amd64 explicitly,
waits for the shared 4 CPU / 8GiB window, validates actual image/resources,
checks host and VM disk space, and preserves official scorer logs and resource
receipts. Cleanup targets only its own containers. Standard Harbor removes the
parent container at trial completion: the model was verified by the official
tests, but no parent weight-file backup was recovered after that cleanup. A startup admission
race can be retried only before any candidate or scorer runs. Completed scores
are never retried. Its task-local proxy is stopped on exit/interruption.
The receipt is named compatibility-preparation.json, separate from the
original-network preparation.json and original official aggregates.

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python \
  evals/terminal-bench/suite/cases/caffe-cifar-10/compat/calibrate.py \
  --source evals/promptfoo/results/qwen38-suite/source/tasks/caffe-cifar-10 \
  --archive evals/promptfoo/results/qwen38-suite/diagnostics/caffe-cifar-10/deep-recovery-20261029/cifar-10-binary.tar.gz \
  --children evals/promptfoo/results/qwen38-suite/diagnostics/caffe-cifar-10/recovery-20261030/children-final-calibration.json \
  --output evals/promptfoo/results/qwen38-suite/diagnostics/caffe-cifar-10/recovery-20261030/parent-amd64
```

The output directory must be fresh. Host controls can be repeated with
`PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/caffe-cifar-10/tests -p 'test_*.py'`.

Final 2026-10-30 calibration: parent oracle reward 1 (six official tests passed),
nop reward 0 (six official tests failed), with the original image and unchanged
source/solution/checker. Training completed exactly 500 CPU iterations; final
training-time test accuracy was 0.5641 and the official independent 100-iteration
accuracy test passed. The original train_quick.sh subsequently attempts its
extra GPU stage and aborts; that original behavior is preserved in raw logs.
It does not undo the completed CPU stage or its official test results.

The original archive downloaded at 198 MB/s inside the task through the local
cache. This measures cache delivery, not public-origin speed. The reference
solution installed every dependency itself (330MB and 87MB dependency batches
completed) and compiled the original Caffe revision
9b891540183ddc834a02b2bd81b31afae71b2153. No dependency image or trained weights
were injected into a candidate. The ephemeral proxy and both parent containers
were removed. No paid model request was made.

Final validation: 23 host tests, 20 offline fixture controls, eight child Harbor
controls, parent oracle/nop and Clippy all passed their expected outcomes.
See compatibility-calibration.json for compact source/image/hash evidence;
complete raw receipts are in the ignored recovery-20261030/parent-amd64 folder.
These compatibility scores must remain separate from original-network results.
