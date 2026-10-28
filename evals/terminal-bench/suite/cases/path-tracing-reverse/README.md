# path-tracing-reverse diagnostics and preparation

Four independent synthetic C tasks cover observed input/output reproduction (m1),
a modular state machine (m2), floor/clamp and nonfinite behavior (m3), and exact
miniature P3 renderer equivalence (m4). Each specifies argv, stdout, exit code,
source path, source size and tool budget. m1 provides an inspectable Python probe;
the other tasks provide complete numerical contracts. These narrower tasks do not
copy parent source, solution, pixels or model trajectories into public fixtures.

The checker compiles actual submitted C and executes it against frozen synthetic
vectors independently calculated with Python arithmetic. It compares integer
values, dimensions, sample counts and process exit codes, with strict decimal
syntax; valid negative zero is equivalent to zero. Every probe uses a new working
directory and compilation, then removes copied source before execution. Each
child has gold, nop and three semantic negative programs. Constant replay, invalid
C and a reference-dependent program are also rejected. Source is limited to 65536
bytes and zlib-compressed size <2100 bytes. The original parent scoring is unchanged.

Host smoke tests use macOS cc without static linking. Linux fixture calibration
uses real gcc -static and libm in fresh offline containers. Host temporary
directories are not a security sandbox; only reviewed fixture programs run there.
The generic tool-only amd64 single-manifest image is pinned in case.json and
validation.json. No parent materials enter that image. No new tool image was
built, pulled or imported for this case.

The complete upstream parent uses original manifest
`sha256:7911e42479d8f6d8af1248bc52442a0efbc5a33e3ecf535fdc20d252a2c64142`,
linux/amd64, compressed layers 170980081 bytes and verified complete layer archive
U=411981824 bytes. Every compressed digest and uncompressed diff_id, original
config and imported RootFS chain were checked. Original source bytes, resources
1 CPU/2048 MiB/10240 MiB and agent/verifier timeouts 1800 seconds remain intact.

Container dependency probes recorded GitHub/Pillow timeouts and apt TLS failure;
the original image also lacks Python. The parent therefore uses the shared
service-mode verifier bootstrap only after the agent exits. It injects authentic,
hash-pinned cached uv 0.9.5, CPython 3.13.7, SQLite runtime and official wheels for
pytest 8.4.1, pytest-json-ctrf 0.3.5, NumPy 2.3.1 and Pillow 11.2.1. It exercises the
real installer and imports that exact dependency closure offline. The apt wrapper
runs the real apt-get with bounded redundant network calls and preserves failure;
it never produces a score. No parent tests or expected values are edited, and no
verifier dependency is available to the agent phase. All bootstrap inputs and
environment are bound in preparation.json.

Completed evidence on 2026-10-28:

- 23 host unittest methods pass. A deliberately permissive checker first caused
  22 assertion failures; a later negative-zero regression separately reproduced
  rejection of valid output before integer value comparison fixed it.
- 20 fresh Docker gold/nop/semantic-negative fixture controls pass.
- Eight shared standard Harbor child controls pass, each oracle=1/nop=0. A shared
  resource lock refused one earlier m3 nop attempt before scoring; that unscored
  infrastructure record is retained. Five valid controls were reused and only
  the three outstanding controls were subsequently run.
- Complete original parent oracle reward=1 with all three official tests passed;
  nop reward=0 with all three tests failed. These are calibration controls, not
  model scores. Fresh original containers were used for each.
- The actual frozen x86_64 ELF, SHA-256
  `847823c5cd99a6828cddfc2e3e5dbb67c5ddcc9f8693a13d539d8a8fa47c9dae`,
  runs --help with exit 0 in the fixed original parent image using its declared
  frozen bundle. Probe: no network, read-only root/bundle, 0.25 CPU, 256 MiB;
  loader output is archived. No API key, model call or new Agent dependency.
- Final inputs/verify_preparation accepts the complete two-parent/eight-child
  receipt. Only the parent verifier policy changed after child controls; every
  actual child setting, fixture, image, checker, frozen runtime, adapter and
  scheduler input was proved unchanged. Old child receipts were not rewritten.
- Promptfoo configuration/test command and final mandatory Clippy pass. Clippy
  used dynamic slot 1, absolute target/build directory
  `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`, JOBS=1;
  owned lock released and compilation cache retained. No Rust source changed.

Standard scoring calls public suite.scoring.harbor_trial directly through
SuiteDockerEnvironment, execution identity
`f759534dd79bbe107a90b14ddc75b553f285607cd7e9c3fd5ca10d841c43f8e5`.
Main and egress-sidecar resource inspections and preset build admissions are
archived. There is no private Trial/Compose wrapper or platform override.

Original 18 GiB S/B/A/safety reserve and actual min(host,VM) admissions were kept.
The initial bounded fetch was interrupted by scheduling, then resumed with the
old partial bytes and failed range responses preserved. Complete image import
had a separate admission after exact U verification. The reviewed stream importer
has three compressed-copy paths: host blobs, host OCI tar, and containerd ingest
promoted into the content store. Two uncompressed copies were conservatively
reserved. Runtime monitoring separately measured original workspace S, verifier
cache B and own evidence A, kept the 5 GiB safety margin and recorded no violation.
No incomplete layer, skipped dependency or unscored attempt was counted as ready.
No paid model was called.

Ignored evidence root:
`evals/promptfoo/results/qwen38-suite/diagnostics/path-tracing-reverse/`.
Key files: completed-checkpoint.json, parents-calibration.json,
children-calibration-production-resume.json, verifier-input-impact.json,
verifier-closure-final-receipt.json, frozen-agent-probe.json,
resumed-layer-verification-final.json, import-receipt.json, resource/capacity
receipts, original interrupted fetch/range logs and clippy-verifier-final.log.
All original source/OCI bytes and verifier dependencies are retained.

Commands from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/path-tracing-reverse/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/path-tracing-reverse/case.json --run-fixtures
```

The public runtime.json is a report without historical_root. Preparation used the
ignored runtime-parent.json referencing runtime-v2 and the pinned parent bootstrap.
The full preflight CLI was not rerun because it would duplicate already valid
controls; its public execution backend and final verification gate were used for
all ten controls. Consult suite/BUILD_RESOURCES.md before any future calibration.
