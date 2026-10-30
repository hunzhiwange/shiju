# install-windows-3.11 diagnostics

Preparation is complete for the execution identity recorded in
`preparation.json`. Production `verify_preparation` accepts all 10 fresh Harbor
controls: parent and m1–m4 each have oracle reward 1 and nop reward 0, without
Trial exceptions or scorer recovery. The parent oracle passes all four official
tests; the nop passes only the disk-content test and receives reward 0.
Twenty unittest controls and twenty independent validator controls also pass.
Shared execution-code changes require fresh calibration; do not relabel old
receipt hashes.

The original parent keeps its official source, solution, tests, Windows image,
2 CPU / 4096 MiB limits and separate 3600-second agent/verifier timeouts. Its
image is `alexgshaw/install-windows-3.11:20251031` at manifest
`sha256:c6695097316ef8810abce0545e347e65354aef9d51d4f7f662c3242e742f6a3e`.
All 14 compressed layer hashes and diff IDs were verified offline: compressed
C is 573,371,248 bytes and exact tar U is 1,576,205,312 bytes. The bounded
startup override restores only the original image CMD (`main.command: null`);
Python/uv/wheels are transported only during the verifier phase.

Four independent synthetic BIOS guests contain no Windows files or upstream
solution material. `environment/make_boot.py` reproduces their raw floppies.

- m1 checks the live QEMU disk, snapshot backing file and immutable base hash.
- m2 checks genuine COM1 boot output and a fresh random keyboard challenge,
  requiring the exact serial echo after each key.
- m3 checks VNC display :1, QMP and real keyboard-driven framebuffer changes.
- m4 checks sustained running state and three fresh display responses.

Each child has a gold launcher, absent-launcher nop and three independent
faults. The no-boot-disk fault actually omits the disk: a network-first boot
order was insufficient because firmware could fall back to the floppy.
The checker waits for observed boot/keyboard events within bounded deadlines;
it does not treat a fixed startup delay as failure. Launcher return remains
limited to 5 seconds, and the verifier remains limited to 120 seconds.
Prewritten serial output, configuration text and self-reported success do not
satisfy the live checks. The earlier deliberately weakened existence-only
checker accepted all 12 faulty launchers; that red evidence remains archived.

Diagnostics use the unchanged genuine amd64 QEMU 7.2.22 image pinned in
`case.json`, with 1 CPU / 512 MiB, no network, a read-only root filesystem,
no capabilities and a bounded 64 MiB writable tmpfs. `/var/tmp` links to `/tmp`
for QEMU snapshot overlays. Harbor controls keep the 900/120-second
agent/verifier budgets. Model policy remains Qwen3.8-Flash, 32000 output tokens,
200000 input tokens, 24 turns, 32 tools and one attempt.

Run fixture checks on a native Linux filesystem. The trusted fixture controller
must create directories owned by the container's root UID: with all container
capabilities dropped, root cannot write another UID's mode-755 directory.
Do not use a 9p-mounted directory for QMP Unix sockets. For example, on the
native Linux checkout with its prepared Python environment:

```sh
sudo env PYTHONPATH=evals/terminal-bench python3 -m unittest discover -s evals/terminal-bench/suite/cases/install-windows-3.11/tests -p test_cases.py
sudo env PYTHONPATH=evals/terminal-bench python3 -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/install-windows-3.11/case.json --run-fixtures
```

The local Mac uses an isolated complete x86_64 Linux VM through Lima/QEMU,
with its own Docker endpoint. Existing Docker Desktop, Rosetta and binfmt
settings are unchanged. Genuine syscall 282 creates a real FD and receives a
128-byte kernel SIGUSR1 event both in the VM and in the original parent image.
The earlier Rosetta failure remains unscored infrastructure, with all raw
scores/logs preserved; it is neither an ability result nor a valid nop.

Working evidence, the VM recipe, native fixture source hashes, capacity and
resource receipts live under the ignored results directory
`qwen38-suite/diagnostics/install-windows-3.11/local-x86/`. Native fixture
receipts are copied back byte-for-byte only after comparison against all
current canonical declared hashes and the actual validator implementation.
Production preflight uses `qwen38-suite/preflight/install-windows-3.11-x86/run`.
Use the same native endpoint for subsequent runs; the original image cannot
start its QEMU 5.2.0 under the unsupported Rosetta backend.

Clippy passed using dynamic slot 1, matching `CARGO_TARGET_DIR` and
`CARGO_BUILD_BUILD_DIR` at the repository's `target/rust-slot-1`, and one build
job. The lock was released and the cache retained. This lint check ran no
behavior tests; fixture and Harbor controls provide the behavioral evidence.

A fresh endpoint also needs Harbor's unmodified egress sidecar prebuilt under
its normal content/platform hash tag. Its `COPY --chmod` instructions require
BuildKit. The local preparation used a dedicated docker-container builder with
1 CPU / 512 MiB and no swap, then checked the native base RootFS prefix, all
four copied file hashes and both executable modes before deleting that builder.
The ordinary production runner subsequently consumes that exact cached image;
no shared suite, Harbor Dockerfile, agent image or network policy is modified.

The unchanged original solution exits 1 after trying to start a duplicate
websockify listener already managed by supervisord. This raw exit code and
port-conflict log are preserved; the untouched official verifier still passes
all four real VM checks. Neither the reference script nor scoring was changed.

The final parent oracle's raw UTC span is 8426.006368 seconds because the Mac
slept during the run. Power logs independently record 5750 seconds of complete
host suspension; including a conservative 14-second timestamp margin gives
an awake-time upper bound of 2690.006368 seconds. Harbor's original
`asyncio.wait_for(3600)` deadline on the host monotonic clock remained enabled.
No raw timestamps or timeout values were rewritten. The full clock audit and
minimal sleep/wake records accompany the ignored calibration evidence.

The task VM is stopped after calibration to release resources, retaining its
verified images and disk. Restart it with
`LIMA_HOME="$HOME/.lima-task47" limactl start win311`, then select
`DOCKER_HOST="unix://$HOME/.lima-task47/win311/sock/docker.sock"` for the run.
Reacquire the suite resource reservation before launching another trial;
`local-x86/reserve-resources.py` records it in the existing Desktop pool.
