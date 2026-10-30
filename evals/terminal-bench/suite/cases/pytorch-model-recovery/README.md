# PyTorch model recovery diagnostics

Four independent synthetic CPU/PyTorch exercises: m1 tensor keys/shapes/counts;
m2 exact Linear/ReLU/Linear reconstruction and TorchScript; m3 output-only
training; m4 trained-state serialization and independently measured MSE.
Each solution accepts explicit input/output paths. The checker generates two
additional datasets with different dimensions, runs real candidate programs,
checks input hashes, and verifies actual tensors and serialized inference.
Printed assertions, candidate-authored tests and self-reported losses cannot
substitute for those checks.

Each child has gold, nop and three distinct incorrect programs. Negatives cover
axis/count/bias errors, missing activation/extra parameters/wrong weights,
no training/all-layer training/wrong-layer training, and pickle-only output,
false loss reports and constant inference. Only synthetic public inputs enter
the standard task environment. Hidden checker/oracle files are uploaded only
for verification, and gold is injected only for oracle calibration.

The tools-only image is linux/amd64, single OCI manifest
`sha256:04cddad837de2801ece50dbf8d4faa35890fd2bf53320add089eeb7f1d485586`.
It extends the clean, pinned task52 Python 3.13 tools base with genuine
`torch==2.7.1+cpu`. To reconstruct the context, fetch the exact filenames in
`environment/dependencies.json` into `environment/wheels/`, enforce their byte
sizes and SHA256 values, then build `environment/Dockerfile` through the shared
constrained builder. Published hashes were checked against the PyTorch CPU
index or the corresponding official PyPI release metadata. Wheels and caches
remain in the ignored evidence directory, not in Git.

The actual classic builder RUN was captured with CPU quota/period
100000/100000 and memory=swap=1073741824 from creation. Installation used
network none. Final compressed layers total 729,119,319 bytes; complete
uncompressed layers, including repeated references, total 1,902,276,096 bytes.
All layer SHA256/diff_ids and the config/manifest chain matched. The image's
/app is empty; /tests and /solution are absent. Both this tools image and the
original parent image passed real offline/read-only startup of the frozen
runtime-v2 Agent with its verified sidecars. Python, PyTorch and Linux shell
entry points were also exercised. Actual child runtime limits remain
1 CPU/512MiB, with the standard network-denial sidecar at 0.25 CPU/64MiB;
direct fixture controls use read-only rootfs and network none.

The unchanged parent registry tag resolved to linux/amd64 manifest
`sha256:63c804250786f1450028e68f66efac1b0ab58718e6bc98686e62f3cd72b0f395`,
config `sha256:038b41ad0d7a8bc956173db3e27d7460dae907d859b844b4df1afd7826cdb114`.
All compressed layers and uncompressed diff_ids matched before OCI import;
C=269,651,480 bytes and U=948,914,688 bytes. Import streamed verified blobs
without writing a second archive and preserved original config and RootFS.
The source task, inputs, reference solution, five official tests, original
1 CPU/2048MiB/10240MiB storage and 900-second agent/verifier limits are retained.
Host/VM space was measured separately; admission used the smaller value and
reserved official storage, tool cache, evidence and a 5GiB margin. Controls
waited when other topics occupied the shared window or host space was low.
No unrelated container, image, Cargo cache or task artifact was removed.

Inside the original parent, Debian HTTPS, PyPI and the PyTorch CPU index were
reachable, but the official Astral installer returned HTTP 403. The standard
adapter's verifier-only bootstrap therefore uploads hash-pinned authentic
uv 0.9.5 and complete CPU/pytest wheels after the agent exits. The uv archive
hash was checked against GitHub's official release asset digest.
`environment/verifier-dependencies.json` records provenance and verifier env.
The bootstrap uses the original environment, HTTPS APT repositories and a
bounded wrapper invoking the real apt-get; nonzero/timeout status is preserved.
The official test.sh remains unchanged, including its installer invocation.
UV_OFFLINE/UV_FIND_LINKS keep pytest==8.4.1, torch==2.7.1 and
pytest-json-ctrf==0.3.5 on verified local wheels and avoid CUDA downloads.
No dependency installation occurs in the checker or before agent execution.

On 2026-10-31, the real-PyTorch host unittest passed 21 checks. The prescribed
Harbor Python lacks torch: its 20 behavior checks explicitly skip, with only
one format check effective; those skips are not successful behavior evidence.
Fresh fixture validation passed all 20 Docker controls: gold/nop/three negatives
for each child. Eight separate standard Harbor child controls also passed,
all four first oracle rewards 1 and all four first nop rewards 0. Earlier
missing-checker and all-accept-checker mutation failures remain archived; the
final checker was not weakened. `validation.json` binds the checked bytes.

The unchanged parent reference solution passed all five official tests, and nop
received first reward 0 (one integrity test passed, four model tests failed).
Initial full preflight rejected a wheel upload target containing `+cpu`;
packing the exact verified wheels into a hash-bound tar archive repaired this
without broadening adapter target validation. A later run was interrupted by
an overconservative task-local capacity watcher after both parent controls and
three child controls had passed. Those results remain separate. The controller
now admits already-installed images without re-reserving their C/U and monitors
the required 5GiB free margin during execution. Before reuse, the production
identity check also detected an unrelated concurrent shared-helper change and
refused the old receipt. No old hashes or rewards were rewritten.

Fresh full `suite.run preflight` completed with exit 0 under execution identity
`e5e2bd2b537679a3f10a63d88710ec7992eb510fcbe82620fa435cf10798e9a0`:
all ten first scores matched (parent and four children, oracle=1/nop=0), with
no verifier replay or reference-solution patch. The current parent oracle
passed 5/5 official tests; nop passed only the original-weight integrity test.
`verify_preparation` passed again against current source, exact task bytes,
runtime, adapter, case settings and raw scoring hashes. `preparation.json` is
the successful current receipt; prior failures and partial receipts remain
separate in the ignored evidence directories. Unfinished admission/resume
controllers and a cancelled redundant child controller were not published as
readiness. No owned trial container remains.

Final admission measured 21,718,011,904 host bytes free; the latest VM snapshot
was 51,705,802,752 bytes. Existing verified images required no additional C/U.
The parent reservation was S=10GiB, B=2GiB, A=1GiB plus 5GiB margin;
continuous disk samples and all actual main/sidecar limits are archived.

Evidence root:
`evals/promptfoo/results/qwen38-suite/diagnostics/pytorch-model-recovery/resume-20261031/`.
It preserves original failures, image/config/layer identities, build and runtime
resource captures, capacity/network probes, fixture checks, the eight child
controls, successful full preflight and local runtime configuration. Historical top-level runtime.json
is a report; current controls use an ignored execution config referencing
runtime-v2. Model policy stays token-plan/qwen3.8-flash, output 32000,
input 200000, one attempt, 24 turns/32 tools. No paid model was run.

Required Clippy and Promptfoo configuration validation passed. Clippy used
dynamically acquired slot 3, identical absolute target/build directories
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-3`, and one job;
exit 0, zero behavior tests. Its owned lock was released and cache retained.
No Rust source was changed or formatted; no full Cargo tests, fmt check, wasm
check or Makefile command ran.
