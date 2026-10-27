# Synthetic merge diagnostics

These four independent exercises use generated repositories and integer grids,
not parent task data, solution prefixes, or model traces. Each submission has
one stable entry point: `sh setup.sh WORKSPACE` for m1–m3 and `algo.map(g)` for
m4. Each child uses fresh inputs and the fixed suite model/budget policy.

- m1 imports two synthetic HEAD-only Git bundles into named branches and checks
  exact tip identities, checkout and clean worktree.
- m2 retains both branch histories through a merge commit and preserves files
  contributed by each branch. Branch1 starts at the first tip and advances on merge.
- m3 resolves a real overlapping edit, retains both histories, and implements
  the explicitly specified multiply-then-add rule on independent rectangular grids.
- m4 rotates unseen nonsquare, singleton and negative-valued grids clockwise,
  retaining input values and leaving the caller's input unchanged.

Gold, nop and three distinct faulty programs per child execute through the
checker; no self-report or keyword matching contributes to a pass. Negative
controls include swapped/missing imports, wrong checkout, reset instead of merge,
unmerged histories, lost files, either-sided conflict resolution, an unfinished
merge, identity/transpose/counterclockwise mappings. A deliberately permissive
initial checker failed 16 of 20 controls before the real checks were implemented.

The diagnostic base is the reviewed amd64 tool image
`vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf`.
Standard materialization retains this exact FROM; local legacy build environment
variables are process-scoped. No platform override or alternate image enters the
materialized tasks. Agent-visible fixtures exclude checkers, oracle data and gold.

The official parent package remains unchanged. Its registry manifest is
`sha256:bfc2a235f2ceea64ffb1ffc5529c2acebf570bff61f3668f0e5669ef570c17d4`,
config `sha256:1994f7ed8975cf2a616e00daa45fc556e842418f8040398053f064cf0bba3c69`.
Compressed layers total 29,725,592 bytes; verified uncompressed layers total
80,648,704 bytes. Cached layers were verified without modification; missing tiny
layers were fetched with bounded ranges. Import streamed the original manifest,
config and layers without an extra archive file. Capacity records include host
and VM free space, all retained copies, a positive 256MiB decompression cap,
official 10GiB storage, 2GiB tools, 1GiB evidence, 5GiB safety and metadata.

Preparation evidence lives in the ignored directory
`evals/promptfoo/results/qwen38-suite/diagnostics/merge-diff-arc-agi-task/`.
The specified historical preflight fails because the public report runtime lacks
`historical_root`; a task-local execution JSON uses the existing runtime-v2.
The verifier-only bootstrap supplies hash-bound real Python 3.13.7, uv 0.9.5,
and pytest dependency caches after the agent phase. Official test.sh, tests,
reference solution, resources and timeouts are unchanged.

Focused verification (from the repository root):

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/merge-diff-arc-agi-task/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/merge-diff-arc-agi-task/case.json --run-fixtures
```

The original preflight failure is retained in `historical-preflight.log`. Promptfoo static validation and mandatory Clippy are
recorded separately. Clippy used dynamically acquired slot 1 with absolute
`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`,
matching build-dir and one job; exit 0, no Rust changes or Rust behavior tests.
The lock was released and cache retained. No paid model calls were made.

Final evidence: `final-gate.json` records 20 passing focused tests, 20 isolated
fixture controls and 10 fresh Harbor controls. Parent oracle passed all five
original tests; nop failed all five. Every child oracle scored 1 and nop scored
0. Calibration used standard generated Dockerfiles, offline children at 1 CPU /
512MiB and egress sidecars at 0.25 CPU / 64MiB; parent retained 1 CPU / 4096MiB
and both original 900-second timeouts. Production `inputs` and
`verify_preparation` accepted `preparation.json`. Resource inspect records,
image identities, dependency hashes, capacity samples and official CTRF logs
are retained. All source hashes remain unchanged and no task containers remain.

The initial cap-dropped dependency probe could not run apt; the default Docker
capability probe matching Harbor succeeded, with all four dependency endpoints
returning 200. A first parent calibration stopped before official scoring because
the cached Python lacked libsqlite3. The verifier bootstrap now installs the
real system runtime libraries; both final controls were rerun from fresh parent
containers and passed. Initial infrastructure failures are retained separately.
