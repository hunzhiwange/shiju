# hf-model-inference diagnostics

Four independent synthetic offline exercises isolate model/tokenizer validation
(m1), small-sample local inference (m2), request/load errors (m3), and a real local
HTTP service (m4). Each instruction defines its full input/output contract,
writable scope and budget. The synthetic bag-of-words format contains no
Hugging Face weights, official hidden tests, solution prefixes or model traces.

The checker executes candidate programs with unseen model directories, varied
weights/bias, repeated and unknown tokens, extreme logits, malformed files and
invalid requests. m4 starts real services, sends repeated HTTP requests and
checks the wildcard listener in Linux /proc/net/tcp. It uses no artifact replay.
Unexpected checker exceptions remain infrastructure failures through the shared
runner. Each child has a gold, missing-program nop and three faulty programs.

The 20 focused tests pass. A weak single-probe/source-only checker let all 12
negatives through, producing 12 test failures (exit 1); the repaired checker
rejects all of them. The independent fixture validator passed all 20 controls
in fresh offline 1CPU/512MiB containers. validation.json binds declared bytes.
The public compiler/Python diagnostic base is reused by its exact digest from
build-cython-ext/gpt2-codegolf and contains no parent task material.

Eight fresh Harbor child controls also passed: each oracle scored 1, each nop
scored 0, with no exception or timeout. Their original scoring evidence and
package bytes were rechecked and reused without rerunning children for final
preparation. Controls ran serially; each main container used 1CPU/512MiB and its
sidecar 0.25CPU/128MiB. All own containers/networks were cleaned up.

The complete parent remains unchanged, using official linux/amd64 manifest
sha256:a88d98a849e81bf42754973d1ed58f5971eb76315bf94d2b19cabedd7d7af9c7
under Registry index
sha256:0ef475aebda8abb8cbf3a1d97aee8ac4b09ca9720550e25a4a46a47930cd3bad.
The frozen runtime requires x86_64 ELF. Compressed layers total 328,580,704 bytes;
full decompression measured 1,162,221,568 bytes. All six SHA256 and diff_id values,
config identity and imported RootFS chain match. Gzip tails were used only as a
lower bound before bounded download; exact-U admission preceded import. The
three-copy peak plus U, original storage, cache, archive, 5GiB safety and 32MiB
metadata reserve was 21,508,870,944 bytes and passed capacity admission. Complete
OCI blobs/manifest/config remain archived; redundant verified parts and import
tar were removed only after rehashing the retained archive. The local official
name was normalized to this verified digest after containerd name lookup failed.

The parent retained 1CPU, 2048MiB, 10240MiB declared storage and separate original
900-second agent/verifier limits. No writable-layer quota enforcement is claimed.
The independent container dependency probe imported the original image's torch,
transformers and Flask, collected all four official tests, and reached PyPI and
Hugging Face config/tokenizer/model endpoints. No alternate Python was needed.

The verifier-only bootstrap injects SHA-bound authentic wheels after the agent
exits and configures offline pip for the unchanged official test script. It
preserves the image's Python/torch/transformers and the running agent service.
It changes no expected answer, agent environment, official file or scoring code.
First-attempt sidecar resource setup, image-name lookup and unsupported PIP_*
environment configuration failures are archived as infrastructure evidence and
excluded from valid controls. Memory/swap settings, local name binding and a
verifier-phase pip configuration file repaired only those setup paths. The final
bootstrap identity was independently probed before fresh parent calibration.

Final fresh parent oracle collected four tests and passed all four (reward 1,
3.26s pytest); fresh nop collected four and failed all four (reward 0, 3.04s).
Neither result has an exception or agent/verifier timeout. The production
suite.preparation.verify_preparation gate accepted all ten controls against
current source, runtime-v2, fixture/checker and verifier dependency identities.
The receipt assembler supplies explicit null recovery_score fields for reused
children; this schema correction did not rerun or change any control.

Raw logs, resources, capacity/OCI receipts, dependency hashes, infrastructure
failures and final-gate.json are under the ignored directory
`evals/promptfoo/results/qwen38-suite/diagnostics/hf-model-inference/`.
Final parent receipts are under the matching preflight directory's `final/`;
child receipts remain in `diagnostics/hf-model-inference/child-harbor-v2/`.
preparation.json binds these controls and their original scoring evidence.
The task-local runtime.json points to verified runtime-v2. Historical
results/qwen38-suite/runtime.json is an identity report, not execution config.
No paid model was called; model/budget policy remains unchanged.

Focused commands from repository root (all exited 0):

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/hf-model-inference/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/hf-model-inference/case.json --run-fixtures
evals/promptfoo/score.sh test
cargo clippy --all-targets --all-features -- -D warnings
```

Clippy atomically acquired shared slot 1 with both CARGO_TARGET_DIR and
CARGO_BUILD_BUILD_DIR set to
/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1,
CARGO_BUILD_JOBS=1. Its lock was released and cache retained. Clippy is a lint
build gate with zero behavior tests; behavior evidence comes from the 20 focused
tests, 20 fixture controls and ten Harbor controls. Already passing checks were
not repeated during the resumed preparation stages.
