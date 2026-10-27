# gpt2-codegolf diagnostics

Four independent synthetic exercises isolate checkpoint parsing (`m1`), a
layer-normalization/linear/GELU residual forward step (`m2`), numerically stable
softmax with deterministic argmax (`m3`), and compiling and running a small
dependency-free C inference entry point (`m4`). Every instruction defines its
complete runtime input and output contract. These are synthetic formats and
operations, not partial copies of the parent solution or TensorFlow format.
No official weights, BPE data, hidden tests or prior model traces enter public
fixtures. Checkers execute candidate programs and compile the C source with
GCC, then exercise multiple unseen numeric inputs and malformed files.

Each child has a gold, missing-program nop and three faulty implementations.
The 20 focused tests pass. Weakening the checker to a single JSON probe and
source existence for C caused 11 tests to fail; restoring all runtime probes
rejected every negative. The separate fixture validator also passed all 20
controls. Expectations use independent population-moment/logistic-form GELU
and sorted-index argmax calculations. C checks compute expected results in
Python and vary model shape, embeddings, biases, token IDs and vocabulary.

The diagnostic image is the digest-pinned public compiler/Python tool image
in `case.json`, reused from build-cython-ext; its Dockerfile contains only
public compiler/Python packages. Its empty `/app`, image history, tool versions
and offline one-CPU/512MiB cgroup probe are archived. Fresh child environments
contain only their own synthetic fixtures. No paid model was called.

The complete parent remains frozen at image
`sha256:537e7bd26b02db5c761230515fa09dc05f89403c6737fbdae4b5793ebc2c7066`,
linux/amd64, one CPU, 8192MiB, 10240MiB declared storage, and separate original
900-second agent/verifier limits. All compressed layer SHA256 and uncompressed
diff_id values were verified. Compressed layers total 614179061 bytes; exact
uncompressed tar size is 852089856 bytes. Bounded temporary download reserved
an explicit U ceiling; exact-U admission preceded import. Full OCI blobs,
index/layout, manifest and config remain archived; redundant verified parts
and the import tar were removed after rehashing the complete retained blobs.
Writable-layer quota enforcement is not claimed.

`environment/verifier-bootstrap.sh` runs only after the agent exits. It installs
SHA-pinned authentic cached uv, Python 3.13.7, pytest 8.4.1 and its CTRF plugin;
a pinned Linux SQLite shared library supports the cached Python build. It
bounds redundant real apt operations while preserving failure codes. Official
source, solution, tests, expected output and scoring are unchanged. The first
attempt failed during verifier setup due to missing libsqlite3; its exception
is retained as infrastructure evidence and excluded from valid controls.
The repaired bootstrap was independently probed with pytest before fresh
calibration. It does not alter the agent environment or replay artifacts.

Raw logs, resource observations, dependency hashes, infrastructure failures,
capacity receipts and OCI identity live under the ignored directory
`evals/promptfoo/results/qwen38-suite/diagnostics/gpt2-codegolf/`. Final scoring
receipts are under the matching preflight directory. `validation.json` binds
all declared fixture/checker bytes. `preparation.json` binds
the full official tree, runtime-v2 identity, dependencies and scored controls.

Focused validation from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/gpt2-codegolf/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/gpt2-codegolf/case.json --run-fixtures
```

The historical `results/qwen38-suite/runtime.json` is an identity report, not
an execution configuration. Preflight uses this task's archived `runtime.json`
configuration pointing at the verified runtime-v2 and verifier-only bootstrap.

Repository checks: `evals/promptfoo/score.sh test` and
`cargo clippy --all-targets --all-features -- -D warnings` both exited 0.
Clippy dynamically acquired shared slot 1 with absolute
`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`
and matching `CARGO_BUILD_BUILD_DIR`, `CARGO_BUILD_JOBS=1`. Its lock was released;
the cache remains. Clippy is a build/lint gate, not behavior-test evidence.

Final fresh calibration passed: the complete official parent oracle collected
one test and passed (reward 1, 5.27 seconds of pytest); its fresh nop collected
one test and failed (reward 0, 0.05 seconds). Neither had an exception or agent
or verifier timeout. Each of the four children independently produced oracle
reward 1 and nop reward 0 in a fresh environment. The production
`verify_preparation` gate accepted all ten controls against the current bytes,
including original scorer receipts and verifier-only dependency identity.
All task containers and sidecars were removed after their result files were
saved. Raw first-attempt setup failure remains separately archived.
