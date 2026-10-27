# KV store gRPC diagnostics

Four independent synthetic diagnostics for `kv-store-grpc`. Original task files,
solution, seven official tests, scoring, 1CPU/2048MiB/10240MiB resources and
900-second Agent/verifier timeouts remain unchanged. No paid model was called.
Suite policy remains token-plan/qwen3.8-flash, 32000 output tokens, 200000 input
tokens and one attempt. Diagnostic verifier timeout remains the suite's 120s.

| Child | Observable behavior | Incorrect controls |
| --- | --- | --- |
| m1 | Compile submitted proto3 schema; inspect field numbers, scalar types and RPC signatures; serialize signed boundaries | String numbers, wrong field number, wrong service |
| m2 | Execute JSON operations; preserve independent keys, overwrites, Unicode, case and signed bounds | No overwrite, absolute values, case folding |
| m3 | Execute strict JSON operations; reject bad types/ranges without state changes; distinguish missing keys | Missing becomes zero, accepts boolean, accepts overflow |
| m4 | Start a real gRPC process; independently compiled client probes persistence across two channels | No overwrite, no storage, wrong port |

Each child has a complete standalone prompt, synthetic input, gold, nop and
three meaningful incorrect controls. Checkers score program output and actual
RPC calls, never candidate claims, tests or source keywords. m3 error semantics
are synthetic and do not alter the parent's missing-key behavior. For m4 the
checker compiles candidate server bindings from the supplied schema and its own
trusted client bindings separately; this child isolates real service behavior.
The parent independently retains its original code-generation requirements.

## Verification evidence

Twenty focused tests and twenty local fixture controls pass. The initial checker
accepted the case-folding mutant because probes did not collide; `red.log`
records that failure. Adding colliding mixed-case keys rejected it without
changing the contract. `validation.json` binds current case/checker/fixtures and
the shared diagnostics implementation. Resumed stages verified the unchanged
bytes instead of rerunning successful fixture or child controls.

Eight fresh Harbor child controls pass under the final shared identity from
commit `1a19b2cf3`: each oracle has reward 1 and each nop reward 0. Every control
uses a new offline container. Main is 1CPU/512MiB, sidecar 0.25CPU/64MiB; all
actual images are amd64 and inherit the exact pinned diagnostic RootFS chain.
Local fixture containers additionally have read-only root, 128 PIDs, dropped
capabilities and no-new-privileges. Every trial's containers were removed after
preserving resource, result and scoring receipts. No public materializer or
Harbor file was changed.

Final fresh original-parent oracle collected seven tests and passed all seven
(reward 1); fresh nop collected seven tests and failed all seven (reward 0).
Their CTRF test names match all seven original test functions, with zero skipped
or pending tests. Neither result has an exception, Agent/verifier timeout or
verifier recovery. The parent used the unmodified production `harbor_trial`,
1CPU/2GiB and process-scoped `DOCKER_DEFAULT_PLATFORM=linux/amd64`.
Independent live inspection saved both parent containers' limits and image
identities. The public `verify_preparation` gate accepted all ten controls
against current source, runtime-v2 and fixture/checker bytes. Existing eight
child controls were reused only after checking their exact final identity;
none was replayed or rewritten to manufacture a new identity.

`preparation.json` binds all ten controls and their original scoring files.
Raw logs, resource receipts, original pytest/CTRF, capacity samples and
`final-gate.json` live in the ignored directory
`evals/promptfoo/results/qwen38-suite/diagnostics/kv-store-grpc/`.
The combined preflight receipt is under
`evals/promptfoo/results/qwen38-suite/preflight/kv-store-grpc/`.
The historical `qwen38-suite/runtime.json` is an identity report, not a launcher;
the task-local ignored `diagnostics/kv-store-grpc/runtime.json` supplies verified
runtime-v2/frozen paths and this parent's unchanged 900s budget, with no startup
or recovery override. Preparation combines two new parent controls with eight
already-valid child controls and runs the production acceptance gate, avoiding
a duplicate ten-control preflight run.

## Images, dependencies and capacity

Diagnostic image (linux/amd64):
`vw-kv-grpc-diagnostics@sha256:5235d587637d5065c1724a0a307c9d0d2fc150cdbd541e0c3174c8845ab40bcc`.
The base is the preexisting Cython diagnostic image
`sha256:7a6db18e11071ff5a05ea8259da78073bd5730b85d39399511385e2a684749d7`.
Its local `vw-win311-base:task47` alias was verified before an offline build.
Image probes found `/app` empty and `/tests`, `/solution`, `/oracle` absent.
Four wheels totaling 9,862,641 compressed bytes were installed offline after a
successful container PyPI probe. Their exact versions and SHA256 are:

| Dependency | Version | Wheel SHA256 |
| --- | --- | --- |
| grpcio | 1.73.0 | a73c72922dfd30b396a5f25bb3a4590195ee45ecde7ee068acb0892d2900cf07 |
| grpcio-tools | 1.73.0 | 0bdf66c37431fec41b40f209d1caa6e38f743cf414e727d9b61c9d2d83b04e52 |
| protobuf | 6.33.6 | e9db7e292e0ab79dd108d7f1a94fe31601ce1ee3f7b79e0692043423020b0593 |
| setuptools | 84.2.1 | ca140b45d5325a9478f880e351484f62fef15f336462e449ca0c03c873d589c2 |

Task-local child builds used `DOCKER_BUILDKIT=0` and
`COMPOSE_DOCKER_CLI_BUILD=1`, explicit amd64 `FROM`, and the existing `:task48`
tag qualified by the unchanged digest. Task-local Compose specifies the main
platform and sidecar limits. No global builder or Docker configuration changed.

Original parent manifest:
`sha256:3399400800dcb207634daa42bc1b052e831e285cc9d221eea66c47bc0fc79791`.
Original config:
`sha256:b9412389bb791988d341f7dcbf456fb68feea133d9e3e6a705fbea976a8acb2b`.
All five complete compressed blobs already existed in the shared task25 cache.
Read-only SHA256, gzip CRC/EOF and decompressed diff_id checks proved
C=44,164,493 and exact U=125,045,760 bytes. The gzip tail lower bound was not
mistaken for exact U. Shared caches were neither copied, deleted nor modified.

After explicit window release, fresh host/VM availability was 21,227,847,680 /
92,200,378,368 bytes. Admission retained 2C + 256MiB Ucap + original 10GiB storage
+ 2GiB tools + 1GiB evidence + 5GiB margin = 19,684,117,274 bytes.
The original OCI manifest/config and verified cached blobs streamed directly to
`docker load`, without layer downloads or an on-disk archive. Import verified
original digest, every config field, exact RootFS chain and amd64 architecture.
Capacity monitors recorded no floor breach. Original-parent PyPI probes for all
six needed packages returned 200. The unmodified reference solution and
verifier then installed their original pinned packages normally; no
verifier-only repair or modified dependency injection was needed.

Focused commands from repository root (all exited 0):

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/kv-store-grpc/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/kv-store-grpc/case.json --run-fixtures
evals/promptfoo/score.sh test
cargo clippy --all-targets --all-features -- -D warnings
```

Final Clippy atomically acquired shared slot 1 with CARGO_TARGET_DIR and
CARGO_BUILD_BUILD_DIR both set to
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`,
CARGO_BUILD_JOBS=1. Its lock was released and cache retained. Clippy is a lint
gate with zero behavior tests; behavior evidence is the twenty focused fixture
cases, eight fresh child controls and two original-parent controls. No Rust was
modified, and no full cargo test, fmt check, wasm check or Makefile ran.
