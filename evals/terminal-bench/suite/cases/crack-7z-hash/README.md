# crack-7z-hash diagnostics

Four independent tasks use small synthetic 7z archives, not upstream secrets,
reference outputs, or model traces. No paid model is invoked. Each task submits
one Python CLI program; hidden black-box checks run the actual program against
both a public archive and a separate archive, including changed passwords and
UTF-8 content. The installed 7z tool is available to candidates.

| Child | Contract | Negative controls |
| --- | --- | --- |
| m1 | Sorted regular-file inventory with full paths and uncompressed byte sizes | directory entries, packed sizes, basename-only paths |
| m2 | Validate dictionary candidates against archive integrity; reject no match | first candidate, always fails, skips passwords containing hyphens |
| m3 | Exact member extraction; wrong passwords/missing members fail without output | ignores password, treats missing member as success, splits filenames |
| m4 | Save exact target bytes, including UTF-8 and trailing newlines | strips whitespace, saves password, selects root-level decoy |

Checkers compare artifacts and process return codes, require input integrity,
and reject unexpected output files. Agent-visible fixtures contain only each
child's synthetic archive, instruction, and (m2) small dictionary. Hidden probes,
checker, and reference implementations are injected only for scoring/solution.
The diagnostic image contains Python and p7zip, with no task inputs baked in.

The intentionally permissive initial checker failed 16 of 20 nop/negative
controls. The corrected checker passes 21 focused tests (including syntax validation of every control program) and 20 fresh offline
fixture calibrations. Each container has 1 CPU, 512MiB, 128 PIDs, no network,
dropped capabilities, read-only rootfs, and explicit candidate/hidden mounts.
The image build recipe is `environment/Dockerfile`; `case.json` pins its actual
linux/arm64 manifest digest. `validation.json` binds fixture/checker hashes.

From the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/crack-7z-hash/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/crack-7z-hash/case.json --run-fixtures
```

The canonical amd64 parent remains blocked. Its unchanged oracle exceeded
1800 seconds under cross-architecture execution; a separate one-thread attempt
was interrupted without scoring. The canonical failure is retained in
`canonical-status.json` and the ignored raw evidence, including the original
AgentTimeoutError. No compatibility result counts as original-official success.

The user approved the independent `crack-7z-hash-arm64-compat` variant on
2026-10-30. It keeps every upstream file unchanged except the copied task's
name and image selection. Original instruction, encrypted input, complete John
command/search stages, reference solution, official tests, 1 CPU, 4096MiB,
10240MiB storage declaration, and 1800/900-second limits remain in force.
There are no paid model calls in preparation.

The variant uses Ubuntu24.04 ARM64 and the identical John commit
`8b5bfefbdc9b643513037f30c8200f5dc0ffc841` / tree
`ee4163c03a0f0792d2f3fb928751bc6a889e6029`, built in `/app/john/src` with
`./configure --without-openssl && make`. It changes architecture, native
ASIMD/AES-CE code paths, compiler/libc/dependency bytes, and 116 common package
versions compared with the original image. Examples: GCC13.3.0 package revision
moves from `13.3.0-6ubuntu2~24.04` to `...24.04.1`; libc6 moves from
`2.39-0ubuntu8.6` to `...8.9`; libgomp1 moves from
`14.2.0-4ubuntu2~24.04` to `...24.04.1`. This is a compatibility environment,
with different performance, rather than an exact reconstruction of amd64.

`variant/packages.lock` fixes the complete 231-package installed closure.
`variant/deb-lock.json` and `SHA256SUMS` bind the 139 added package bodies to
SHA256 values from authenticated Ubuntu APT indexes. Downloads use HTTPS;
installation is offline. The read-only before-agent proof verifies native
architecture, full installed versions, every regular dpkg-owned file's bytes,
John binary hash, clean tracked source tree and exact Git commit/tree. It also
rejects leftover John pot/session files and an existing solution artifact.
The image does not preinstall the oracle's Perl LZMA module, 7zip, curl, uv,
Python, verifier packages, candidate hints or answer files.

The exact source tar, public Git objects, locked debs, authenticated indexes,
OCI layers, build logs and staging scripts are retained under the ignored
`evals/promptfoo/results/qwen38-suite/diagnostics/crack-7z-hash/arm64-approved-20261030/`
directory. To reproduce the build, use its `build-approved.py` staging script
with those SHA-bound assets; `variant/Dockerfile` is the reviewed build recipe.
No upstream archive, target plaintext, hidden checker or official solution is
committed here. Only synthetic diagnostic archives are checked in.

The local execution configuration is the ignored approved directory's
`runtime.json`; its parent selects the variant explicitly. Preparation uses the
standard `suite.run preflight` entry and records a fresh oracle/nop pair for
that variant plus four independent diagnostic pairs. `preparation.json` is
valid only with matching scheduler/runtime/fixture/image identities and all ten
controls. It is never a model score. The backend preserves the 10240MiB storage
declaration but does not enforce a Docker writable-layer storage quota.

Calibration completed on 2026-10-30: the compatibility parent and all four
children each scored oracle=1 / nop=0 in ten fresh standard Harbor trials.
The parent oracle ran for 612.164150 seconds, below its 1800-second limit;
its unchanged official checker passed both tests. The canonical parent remains
blocked, and no model was run. The complete preparation receipt was verified
against current source, scheduler, runtime, image and scoring evidence.

Current verification passed: 21 topic tests, 20 fresh fixture controls
(4 gold, 4 nop, 12 meaningful negative programs), 91 focused shared/scheduler
tests, Promptfoo's configuration/checker checks, and the required
`cargo clippy --all-targets --all-features -- -D warnings` (exit 0).
Clippy used dynamic slot 2 with both target/build directories set to
`$REPOSITORY/target/rust-slot-2` and `CARGO_BUILD_JOBS=1`; its lock was released.
An earlier invocation failed while the host Rust toolchain was being replaced;
that log is retained separately and the successful fresh invocation supersedes
it. This task changed no Rust files. Repository-wide Cargo tests, formatting,
wasm checks and repository Makefile commands were not run.

The final ignored `arm64-approved-20261030/completion.json` binds the preparation,
validation, execution config and variant contract hashes. Use its `runtime.json`
for the next explicitly requested model experiment; keep compatibility and
original-official results separate.
