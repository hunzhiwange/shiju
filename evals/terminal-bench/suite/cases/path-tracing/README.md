# path-tracing diagnostics

Four independent synthetic tasks isolate P3 target interpretation (m1), sphere
silhouette rendering (m2), floor/clamp/color conversion (m3), and standalone C
compilation with generated image comparison (m4). Public instructions specify
coordinates, interfaces and output rules. Parent source, image and solution remain
in the ignored official task package; none is copied into these fixtures.

Each child has a gold program, nop and three semantic negative programs. The
checker executes candidates on multiple unseen synthetic inputs and compares
actual dimensions, pixels or structured results. m4 compiles a fresh executable,
removes the submitted source before execution and compares the newly generated
PPM. Existing submitted output is ignored. Parent tests and scoring are unchanged.

Run from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/path-tracing/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/path-tracing/case.json --run-fixtures
```

Host checks use Python standard library and cc on macOS. Formal Linux amd64
checks use gcc -static. Host temporary directories are not a security sandbox;
execute only reviewed fixtures there. The shared container checker uses fresh,
offline, isolated containers. Its 20 controls cover all gold/nop/negative programs.
The two host test methods additionally reject three fixed replay programs, for
23 candidate controls. Historical TDD evidence removed the final equality gate:
13 assertions failed; restoring it passed. Missing-asset failures are not counted
as semantic TDD evidence.

## Verified preparation — 2026-10-30

All ten fresh standard Harbor controls passed through public `harbor_trial` and
`SuiteDockerEnvironment`; `verify_preparation` checks the complete receipt against
current source, runtime, adapter, checker and execution identities.

| Task | Oracle reward | Nop reward |
| --- | --- | --- |
| Original path-tracing | 1 | 0 |
| m1 | 1 | 0 |
| m2 | 1 | 0 |
| m3 | 1 | 0 |
| m4 | 1 | 0 |

The original oracle passed all five official tests, including static compilation,
zlib-compressed source size <2100 bytes, execution in chroot and cosine pixel
similarity >=0.99. The original nop failed as expected. All rewards are from the
first scorer; no verifier recovery or paid model calls were used.

Execution identity:
`94dec6f0c90690b5c0ea348c8142a22b0cec1d62fa174df762f6dc93cb7653df`.
Current `validation.json` binds the 20 checker controls; `preparation.json` binds
the ten standard controls and their exact raw scoring evidence. Earlier receipts
and failed shared-resource admissions remain in ignored logs, separate from the
successful final controls. Historical hashes and scores were not overwritten.

The tool-only single-platform Linux amd64 image is pinned in case.json to
`vw-path-tracing-tools:single@sha256:731ea515c4600aed7c9e5f6169e71366d0e7cc91a26e54863f13a35fbd5c6e32`.
Its config is `sha256:d91779a0643e542fcf3f1814c439844c39a89ad71973ab4a0e99d872e1ccab7c`.
It contains Python, gcc/static libc and Valgrind, without parent task materials.
The local named reference preserves the original cached tool Config and RootFS;
only its OCI index reference was canonicalized to the existing amd64 leaf.

The original parent image is
`sha256:4b44725102cd627dfa895191a5a30d16dae3c5623bb43da6459f79135db6b482`
(config `sha256:189b026301d200f7d2dec5cc9dc617c3c70163e687b407201a6a9300fdf8d655`).
Full compressed SHA256 and uncompressed diff_id streams were verified, with
C=399386034 bytes and exact U=1095040000 bytes, below a positive 2GiB hard cap.
Streaming OCI import preserved the manifest, Config and RootFS without a disk
archive. Download/import capacity receipts account for compressed copies,
metadata and the original 18GiB reserve; cached final parent trials were admitted
again after host capacity recovered. No unrelated caches or business containers
were removed by this task.

Actual runtime inspections confirm main containers at 1 CPU, with parent
2048MiB and children 512MiB. Child egress sidecars use 0.25 CPU/64MiB. Shared
builder admissions and logs preserve limits from creation. There were no private
Trial/Compose/Builder or platform overrides and no post-start resource changes.
Owned containers, networks and volumes were cleaned after verification.

Final ignored evidence is under
`evals/promptfoo/results/qwen38-suite/diagnostics/path-tracing/`:
`children-calibration-final.json`, `parents-calibration-complete.json`,
`nop-final3-receipt.json`, `preparation-final.json`, `ready-final.json`,
per-trial scoring/resource receipts and `clippy-final-retry.json`.
Clippy (`cargo clippy --all-targets --all-features -- -D warnings`) and Promptfoo
configuration validation exited 0. Clippy dynamically claimed slot 1 with absolute
`CARGO_TARGET_DIR` and `CARGO_BUILD_BUILD_DIR` ending in `target/rust-slot-1`,
`CARGO_BUILD_JOBS=1`; its owned lock was released. No Rust source changed and no
full Rust unit tests, fmt check, wasm check or Makefile commands were run.
