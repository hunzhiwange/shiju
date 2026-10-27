# MIPS port diagnostics

Four independent synthetic tasks exercise the porting boundaries without
copying the original DOOM code, reference patch, WAD, emulator or hidden tests:

- m1 configures GNU cross-assembler/linker arguments. The verifier actually
  builds a frozen probe and executes the resulting ELF to check its entry.
- m2 supplies a real MIPSel ELF that sums unsigned bytes, including empty,
  NUL-containing and high-byte buffers. Six independent input buffers run.
- m3 supplies a real MIPSel ELF that opens, writes and closes a 2x2 top-down
  BGRA BMP through the synthetic VM's custom syscall ABI. Three different
  pixel buffers must produce the exact header and flushed image bytes.
- m4 implements a bounded ELF/MIPS interpreter. Seventeen generated programs
  exercise entry relocation, arithmetic, zero register, memory, branch and
  return delay slots, plus five invalid inputs. Expected register values are
  independently calculated, rather than inferred from a candidate or oracle.

These are explicitly synthetic ABI contracts. They retain the original
little-endian MIPS register and custom write/open/close syscall conventions;
these are not Linux MIPS syscall numbers. Synthetic loading uses PT_LOAD and
e_entry, and terminating functions return through a sentinel. The original
parent keeps its own section/symbol loader, endless rendering loop and official
scoring unchanged. Child diagnostics never replace official parent evidence.

Each child has one executable gold and three wrong variants. The m1 wrong
configurations select the wrong endian, entry or unsupported machine. The m2
variants sign-extend bytes, skip the last byte or return a constant. The m3
variants use Linux syscalls, truncate pixels or omit close. The m4 variants
hardcode a sample, ignore e_entry or omit delay slots. The independent verifier
executes artifacts; final-answer keywords and agent self-tests are not scored.
Assembly sources beside the ELF fixtures document how gold and negatives were
built with GNU Binutils 2.40. No paid model calls are part of preparation.

The clean amd64 tool image is
`vw-mipsel-diagnostics:task54@sha256:cb07016b218e9875ee039483a73567c5841b9bbae19f4bdaab5257e6cce9a93a`.
It extends the verified task52 single-platform image with exactly
`binutils-mipsel-linux-gnu=2.40-2cross2` for amd64. All library dependencies
already exist in that base; no cross GCC/Clang, apt index or upgrades are added.
The base has empty `/app`, with `/tests`, `/solution`, `/oracle` and parent
material absent. The declared digest is used verbatim by standard
materialization; no alternate FROM, platform, reference or Trial override is
needed. Keep DOCKER_DEFAULT_PLATFORM unset; local legacy builds use
DOCKER_BUILDKIT=0 and COMPOSE_DOCKER_CLI_BUILD=1.

To reconstruct the tool build context, obtain the single Debian bookworm deb
from `https://deb.debian.org/debian/pool/main/b/binutils-mipsen/binutils-mipsel-linux-gnu_2.40-2cross2_amd64.deb`
and place it beside `environment/Dockerfile`. Check SHA256
`e803da3a36d96b7e16ae54a0e84c850a10e959d39f4a83efd02bcadc18db9e11`
and length 2,795,020 bytes before building. Its data.tar is 16,568,320 bytes;
only an ldconfig trigger is present. This input is an external system package,
not a committed fixture. Build offline with legacy flags
`--cpu-period=100000 --cpu-quota=100000 --memory=512m --memory-swap=512m`.
Preparation retained its own intermediate RUN containers to inspect actual
limits before removing only those containers. Both RUN containers had the
required limits. The conservative incremental peak budget was 186.6622 MiB,
under the authorized 192 MiB; observed host free-space delta peaked at
34,115,584 bytes. No apt temporary allocation was needed.

Focused commands from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/make-doom-for-mips/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/make-doom-for-mips/case.json --run-fixtures
```

Raw red/green logs, capacity checks, package metadata, dependency probes and
standard Harbor calibration evidence are retained in the ignored directory
`evals/promptfoo/results/qwen38-suite/diagnostics/make-doom-for-mips/`.
The deliberately weak existence-only checker missed all twelve wrong
artifacts (20 controls, twelve assertion failures and no errors). The strict
checker passed 24 focused tests, including four interpreter contract tests.
The final validation receipt binds all 20 fresh oracle/nop/negative controls.

The historical runtime preflight fails with missing `historical_root`. A
task-local runtime-v2 configuration preserves that historical runtime file.
Verifier bootstrap supplies the authentic uv 0.9.5 installer/release and real
Python3.13 wheels after the agent phase. It does not replace the official
checker, reward computation, source, model, resource limits or 900 second
parent agent/verifier timeouts. Fixed dependency identities and actual parent
calibration results must be taken from the final preparation receipt.

Final calibration accepted the unchanged official parent: oracle passed all
three tests (VM stdout, frame existence, and >=95% reference-image similarity)
and scored 1; nop failed all three and scored 0. Oracle execution took
291.153286 seconds, within the original 900 seconds. The nop's expected
missing-frame wait is an official failing test, not a passing timeout artifact;
neither parent control had a Harbor exception or agent/verifier deadline.
Both parent controls used the original amd64 manifest
`sha256:80a9459f1b94b692ea8346f68e6b864f6c4d984fbdf6ffd68ad332308300c2ec`,
config `sha256:c503768044b13ea799f26ee48c83c5885cd9697114bc3e320e1e5f926276188f`,
1 CPU, 2 GiB memory and the original resource/time contracts. Full compressed
SHA256 and uncompressed diff_id verification measured C=189,544,020 and
U=474,560,000 bytes. The positive unknown-U cap was 512 MiB, with a separate
16 MiB metadata margin. Retained verified parts streamed directly into Docker;
no duplicate full OCI archive was written and no other task was deleted.

The task-private controller kept the policy's original S10/B2/A1/5 GiB budget.
Entry required all 18 GiB. During execution it charged only this exact
container's measured SizeRw to B and new task-owned evidence blocks to A;
S10 and the 5 GiB safety margin stayed fully reserved. Every sample bound the
host/VM minimum, exact own IDs, measured high-water and remaining budgets.
Oracle peak B was 1,383,952,384 bytes; nop peak B was 299,655,168. Each stayed
below B2 GiB with a 128 MiB stop guard. The older overly conservative helper
is preserved with its hash for audit; public policy, original task and formal
worker behavior were not edited. An immediate second entry check briefly
failed during asynchronous Docker space reclamation. After actual capacity
recovered, only the not-yet-started nop resumed; oracle was not repeated.
That admission failure remains separate from valid calibration results.

`preparation.json` binds both parent controls and all eight fresh standard
child controls to the same current production input identity. The final gate
checked all three official test identities and actual passed/failed counts,
raw scoring hashes, dependency bootstrap, resources, source identity and
phase-budget receipts, then `verify_preparation` accepted all ten controls.
`validation.json` binds the twenty fixture controls. No task containers remain.

The final Clippy command was
`cargo clippy --all-targets --all-features -- -D warnings` (exit 0, no behavior
tests). It dynamically acquired slot 1, with
`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`,
matching `CARGO_BUILD_BUILD_DIR` and `CARGO_BUILD_JOBS=1`; the lock was released
and cache retained. Promptfoo configuration checks also exited 0. No Rust
sources, public scheduler code, baseline, model, output/input budget or paid
model attempts changed.
