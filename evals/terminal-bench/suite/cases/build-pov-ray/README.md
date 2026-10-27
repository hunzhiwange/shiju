# build-pov-ray independent diagnostics

No model is invoked by preparation. The original task remains byte-identical to
catalog source revision `7131e4375048a0e408a8fb404b5f499d726b695b`.
Original resources remain 1 CPU / 2048 MiB / 10240 MiB, with 12000-second
agent and verifier limits. Official tests, reference image and solution are
never copied into diagnostic inputs or images.

| Child | Independent task | Black-box evidence | Rejected mutations |
| --- | --- | --- | --- |
| m1 | Extract a synthetic legacy source layout | Exact relative paths and SHA-256 of extracted bytes | Flattened tree, missing platform, corrupt C source |
| m2 | Compile/link two synthetic C translation units | Real ELF executed on five integer inputs; inputs remain intact | Missing object, wrong arithmetic, shell wrapper |
| m3 | Drive installed POV-Ray 2.2 CLI | Real renders of two synthetic scenes at distinct sizes, compared with independent renders | Fixed dimensions, wrong scene, forged reference |
| m4 | Compare real small TGA renders | Equal, one-pixel, metadata-only, size-mismatch and truncated images | Header-only comparison, metadata comparison, accepted truncation |

Each child is self-contained, offline, fresh, 1 CPU / 512 MiB, with its own
oracle, nop and three negative controls. The fixed suite model/output/input
policy and request/tool limits are unchanged. A normal process exit does not
substitute for the official reward.

## Dependency identity and reconstruction

Original registry manifest:
`alexgshaw/build-pov-ray@sha256:874baf49341e2853a15a9184e308c7eb632e97f434359fcff2891c6e5ca1b39b`
(linux/amd64, explicitly started on the ARM64 host).
Diagnostic image:
`vw-povray-diagnostics@sha256:44a3d9e4b3cd66189b0be3fecd2438a924a744914ce3aa1ea56ef19620df14a5`
(linux/arm64). Its compiler/Python base is the previously reviewed, fixture-free
`vw-cython-diagnostics@sha256:e9ee4d2ebe7c9ff17917ef72979da36ee52876f5fc098ee1338db029f4cbe247`.
Only the public renderer binary is added; the base's existing Python packages
are not used by these standard-library checkers.

Download the public `POVSRC.TAR.Z` from
`https://www.povray.org/ftp/pub/povray/Old-Versions/Official-2.2/`;
compressed SHA-256 is `4d8a7073fadaca82827f1354428393cd13e4d3f71a5a3149fd7d6fffd77293d4`
(and retained with all three archive identities in local `archives/identity.json`).
Decompress it to `POVSRC.TAR`, then run `environment/build_renderer.py` in a
writable empty directory inside that compiler image. The script checks the
uncompressed archive SHA-256 before selecting the Unix build sources. It
makes explicit modern-C compatibility/RNG fixes and a defined successful
exit status. These affect only the independent diagnostic dependency.
No change is applied to the original task or its solution.

The independently rebuilt binary matches the image dependency SHA-256:
`b704c8942485ea2296c28d19cbc78b768646780dae8e8ef975f9835231cb887b`.
Build `environment/Dockerfile` with a context containing only that `povray`
binary. Preserve the exported OCI archive and platform manifest; locally built
multi-platform index references must be imported under the exact platform
manifest digest for Harbor/BuildKit to resolve them without a registry pull.
All blob hashes and the local RootFS layer chain are verified before use.

## Validation evidence

Raw evidence is retained under the ignored directory
`evals/promptfoo/results/qwen38-suite/diagnostics/build-pov-ray/`.
`red-tests.log` records the missing-assets red test; `checker-red.log` proves
the corrupt-source negative escaped the initial path-only layout checker.
The fixed checker compares all file bytes. A second red test
(`reference-overwrite-red.log`) proves a candidate could overwrite a colocated
reference image; m3 now snapshots reference pixels in evaluator memory before
executing the candidate, and rejects an executable reference-forgery control. The original rendering orientation
was measured before fixing the synthetic TGA contract to top-left origin.
`validation.json` binds all declared fixture/checker bytes and 20 actual
fresh-container controls (4 gold, 4 nop, 12 meaningful mutations).

The original oracle passed all 3 official tests, including render SSIM 0.8731
against the unmodified threshold 0.87. The fresh original nop ran and failed
all 3 official tests, reward 0. All eight fresh child Harbor controls returned
the expected oracle=1/nop=0. Full oracle/nop and child Harbor receipts
are collected in `preparation.json`; their raw scoring paths and hashes are
verified by the common preparation validator. No historical or paid model
result is counted as a preparation control.

The supplied preflight command initially failed because its `runtime.json`
was an evidence report, lacking `historical_root`. Preparation uses a local,
explicit scheduling configuration referencing the already frozen generation-2
runtime, original source root and this case root. Already valid controls are
reused with their original evidence, without rerunning the oracle.

Focused unittest: one test method, 20 executable controls. Diagnostic CLI:
20/20 controls. Promptfoo configuration validation and JavaScript syntax checks pass. Required Clippy
exits 0 (no Rust behavior tests), dynamically claimed shared slot 1 at
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`;
`CARGO_BUILD_BUILD_DIR` matches, jobs=1. The owned lock is released and cache
retained. See local `clippy.json` for the exact command and exit status.
