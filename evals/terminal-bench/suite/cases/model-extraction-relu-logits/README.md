# ReLU black-box extraction diagnostics

Four independent synthetic Python JSON programs write `/app/solution.py`:

- m1 queries an imported scalar `forward(x)` API, preserving signed outputs,
  duplicates, ordering and empty input.
- m2 locates every slope change on a supplied grid, including negative changes,
  the last interior knot, irregular spacing and affine functions.
- m3 normalizes incoming rows by positive Euclidean norm, rescales biases and
  outgoing weights, and orders neurons without sign flips or deduplication.
- m4 compares signed outputs on unseen vectors, including neuron permutation,
  positive scaling, changed biases, unequal widths and tolerance boundaries.

The checker executes submissions on 4, 5, 5 and 6 independent probes. m1/m2 get
fresh synthetic callable modules for each probe. m3/m4 use frozen arithmetic
expectations. No original parent parameters, solution or hidden scoring data
enter child fixtures. Each child has executable gold, empty nop and three wrong
programs. The red existence-only checker incorrectly accepted all 12 negatives;
actual execution and structured numeric comparison reject them. All 20 focused
tests and 20 fresh fixture controls passed. Numbers must be finite, booleans
must be JSON booleans, and keys/list lengths must match exactly.

The clean amd64 tool image is
`vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf`.
The restricted probe found `/app` empty and `/tests`, `/solution`, `/oracle`
absent. Standard materialization uses this exact FROM. Local controller builds
use `DOCKER_BUILDKIT=0 COMPOSE_DOCKER_CLI_BUILD=1`, leaving
`DOCKER_DEFAULT_PLATFORM` unset. No platform, CMD or Trial override is used.
All eight standard fresh child controls passed (oracle=1, nop=0), with actual
amd64 image ancestry verified. Main containers used 1 CPU / 512 MiB; offline
egress sidecars used 0.25 CPU / 64 MiB. Hidden files enter only their phases.

The original parent manifest is
`sha256:52fe1f089f38650f0dc22d7531ee6e01ebd526de1b349aaa2ecb331dca9fabce`,
config `sha256:cc17ca081e08612a89a3d185fc4af6735ee2a08eaeb9478f8b2474392c4feac8`,
platform linux/amd64. All compressed SHA256 and uncompressed diff_id values
were verified: C=83,690,938 and U=217,829,888 bytes. Original layers were streamed
to docker load without an archive file, and manifest/config/RootFS were checked.
Original instructions, data, solution, scoring, 1 CPU / 2048 MiB / 10240 MiB
resources and 900-second agent/verifier timeouts remain unchanged.

The initial capacity admission failed and was preserved. After capacity
restoration, admission required 20,048,382,836 bytes including two compressed
copies, a hard 512 MiB uncompressed cap, 10 GiB official storage, 2 GiB tools,
1 GiB evidence, 5 GiB safety and 16 MiB metadata. Actual host free was
20,260,110,336 bytes; VM free was 91,992,514,560 bytes. Unknown U was never zero;
gzip ISIZE was only a lower bound before full verification. During execution,
remaining B/A reservations were reduced only by exact own-container SizeRw and
new own-evidence st_blocks high-water measurements. S=10 GiB and safety=5 GiB
remained fully reserved. Unattributed host/VM changes received no credit.
Oracle peak own writes were 215,310,336 bytes; nop peak was 214,863,872 bytes.
All phase monitors passed and all task containers were removed.

In the original container, apt/PyPI returned 200; the fixed uv installer URL
returned 403. The verifier-only bootstrap supplies the SHA-bound authentic
installer, uv 0.9.5 release and offline wheels, after agent completion. The
original test.sh still installs curl, runs the original installer and invokes
its pinned tools. Actual probes verified Python 3.13.7, pytest 8.4.1, NumPy
2.3.1, SQLite dynamic-library closure and a real SQLite query. There is no
replacement checker or artifact replay. Dependency hashes are bound in the
preparation policy and ignored wheel inventory.

Fresh parent oracle passed the official test and scored 1; fresh nop failed
that test and scored 0. Verifier times were 20.59 and 14.57 seconds. Both had no
harness exceptions, timeouts or scoring recovery. `preparation.json` and the
ignored `final-gate.json` passed production inputs/verify_preparation for all ten
controls. The eight child controls retain their original evidence: an explicit
identity-transition receipt proves only parent verifier dependency transport
and its aggregate runtime hash changed; child settings, bytes, image, checker,
runtime binary, adapter and Harbor identity are unchanged.

Commands, from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/model-extraction-relu-logits/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/model-extraction-relu-logits/case.json --run-fixtures
evals/promptfoo/score.sh test
cargo clippy --all-targets --all-features -- -D warnings
```

All four checks exited 0. Final Clippy used dynamically acquired slot 1 with
CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR both
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`, and one build
job. Its lock was released and the cache retained. Clippy runs no behavior
tests; no Rust files changed. No paid model was called.

Raw logs, resource receipts, scripts and continuation evidence are retained in
`evals/promptfoo/results/qwen38-suite/diagnostics/model-extraction-relu-logits/`.
The specified historical report runtime preflight failed with missing
`historical_root`; this failure remains archived. Case-local runtime configs
reference the existing verified runtime-v2. The parent execution config adds
only verifier dependency transport. Shared runtime/report files are unchanged.
Final production acceptance is also retained under the parent's preflight path.
