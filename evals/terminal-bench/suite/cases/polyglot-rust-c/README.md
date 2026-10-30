# Rust/C++ polyglot calibration

Four independent synthetic tasks require one unchanged `/app/shared.rs` source:
a constant marker (m1), a sum of squares (m2), strict decimal arguments and a
64-bit result (m3), and matching polynomial/residue entry points (m4).
Each checker compiles fresh Rust and C++ binaries, then checks their actual
exit status and exact stdout against an independent numeric oracle. Old binaries,
agreement on incorrect answers, and self-reported success cannot pass.
Each child has gold/nop and three semantic negative controls. The synthetic
sources contain no upstream solution prefix, Fibonacci probes or model traces.

Calibration completed on 2026-10-30 without model calls:

- Linux fixtures: 20 controls, exit 0 (4 gold, 4 nop, 12 semantic negatives).
- Complete public `suite.run preflight`: 10 controls, exit 0. The original parent
  and m1..m4 each score oracle=1 / nop=0 on the first official scorer.
- Clippy: `cargo clippy --all-targets --all-features -- -D warnings`, exit 0.
  Dynamic slot 3 uses this repository's `target/rust-slot-3` for both target and
  build directories, JOBS=1. The owned lock was released; cache retained.
  The earlier slot-1 incremental-cache disappearance is archived separately.
- Host focused unittest: exit 0, one test method with 20 controls. The installed
  stable compiler directory must precede rustup in PATH when no default
  toolchain is configured outside this repository. Global rustup was unchanged.
- `evals/promptfoo/score.sh test`: exit 0; configuration valid.
- `suite.preparation.inputs` and `verify_preparation` validate all original
  eight file hashes, the current runtime/Harbor/scheduler identity, all scorer
  evidence and all dependency hashes. `validation.json` and `preparation.json`
  bind the successful calibration to its exact inputs.
- Original parent main containers use 1 CPU / 2048 MiB; diagnostic mains use
  1 CPU / 512 MiB and egress sidecars 0.25 CPU / 64 MiB. Original parent storage
  reservation and 900-second agent/verifier limits remain in force. Docker's
  writable-layer storage quota is not claimed to be enforced.

The original parent is the genuine linux/amd64 Registry manifest
`sha256:3ba84f0e138a4b65e58c53f920a0fc852571e6402901c3bdbb876e9dab6e6c5e`,
config `sha256:42e1c101034156ab6a0cbefc63ee654ab7fccb668e1580a090abba89244f6a93`.
Every compressed layer SHA and uncompressed diff_id was checked. Compressed
bytes C=271241474; complete tar U=781531136, below the admitted hard 2-GiB cap.
Streaming OCI import creates no on-disk OCI archive or complete part copies.
Admission reserved all three compressed and two capped uncompressed copies,
original S10+B2+A1 per parent, 5-GiB safety and metadata against min(host, VM).
The actual original image has rustc 1.75.0 and g++ 13.3.0; the latter differs
from the upstream prose's 13.2.0, so the pinned original image was retained.
The frozen runtime ELF runs with its unchanged read-only sidecars.

Only the verifier receives hash-pinned, authentic cached curl/uv 0.9.5,
Python 3.13.7, pytest 8.4.1/plugin wheels, a CA bundle and SQLite library after
the agent exits, through the existing public service-bootstrap mechanism.
`environment/verifier-bootstrap.sh` neither touches `/app` nor replaces the
original test scripts or checker. Its apt wrapper executes the real apt-get,
retains its exit status and bounds redundant dependency installation inside
the original verifier budget. Python/pytest use the frozen offline cache.
This is original-environment scoring, with no artifact replay or scoring change.

The diagnostic image remains the genuine single amd64 manifest
`vw-polyglot-rust-c-tools:task70@sha256:20a0f6d71e716c237e1ccc84c243548d75eac2ded6395004217ff55da77124d9`.
It uses Python 3.13.7, g++ 12.2.0 and official Rust 1.75.0 distributions.
The reproducible recipe, official download hashes and license notes are in
`environment/`. Its complete tar U=1471391744 includes reused base layers and
repeated empty-layer references; inspect Size was not used as tar U.

Current execution identity: `94dec6f0c90690b5c0ea348c8142a22b0cec1d62fa174df762f6dc93cb7653df`.
Diagnostic implementation: `2365b523747f771c31c18689f42b99cae85ed73b9b77f762138e4a33b3f776c4`.
Ignored raw evidence is under
`evals/promptfoo/results/qwen38-suite/diagnostics/polyglot-rust-c/recheck-20261030/`;
the successful full preflight is under
`evals/promptfoo/results/qwen38-suite/preflight/polyglot-rust-c-20261030-sqlite/`.
Earlier capacity denials, a resource-admission rejection and the missing-SQLite
verifier-bootstrap attempt remain separate infrastructure evidence. No failed
attempt was counted as a pass. Older stdout-check mutation evidence remains
valid for the unchanged checker: disabling stdout comparison caused 11 failed
assertions. All owned calibration containers, networks and volumes were removed;
images, dependency archives and original evidence remain available.
