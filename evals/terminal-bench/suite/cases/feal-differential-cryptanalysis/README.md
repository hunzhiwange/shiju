# FEAL differential cryptanalysis diagnostics

Four independent synthetic tasks isolate a 16-bit round function (m1), chosen
plaintext XOR pairs (m2), filtering every candidate against every differential
observation (m3), and recovery through a bounded local query callable (m4).
The synthetic function, constants and inputs are independently specified;
no parent solution, input, hidden test or model trajectory enters child images.
Each child has a complete instruction and only its declared input.json.

Candidates implement executable Python programs at /app/solution.py. The checker
runs those programs with independent expected outputs, checks exact JSON types
and complete results, rejects changed inputs and symlinks, and bounds execution.
For m4 it invokes recover(query) against eight fresh keys, enforces integer
plaintexts and at most four queries per key, and verifies the returned keys.
Every child includes gold, nop and three distinct incorrect implementations.
m3 deliberately preserves two equivalent differential candidates: accepting
only the first would fail complete candidate enumeration.

The original parent remains unchanged: source tree
595263f1617cccfc8548ac7528734caf399b35a6, instruction SHA256
b4093780b283e5c9838997769fb7bca090e3f4cbbd2d7cb275ba60cdfd722572,
configuration SHA256
dcb9f2f15047eee0be15ca18c39c357f89187a94aae6baeff276221e2383eecb.
It retains 1 CPU, 2048 MiB, 10240 MiB nominal storage, no GPU, and 1800-second
agent/verifier limits. The official checker is authoritative; this preparation
does not add a 30-second attack assertion absent from that checker.

Diagnostic images contain only synthetic fixtures, using a digest-pinned clean
Python image. Local fixture calibrations use fresh offline containers, 1 CPU,
512 MiB, dropped capabilities and no repository or parent-task mounts.
Harbor runs each oracle/nop in a fresh environment. Completed local containers
are archived and removed to keep resource usage bounded.

The verifier-only bootstrap runs after the parent agent exits. It injects
hash-pinned authentic uv 0.9.5, curl and Python package caches; the original
image's Python 3.13.7 and headers remain in use. Debian repository URLs use
HTTPS. A cache of 52 authentic Debian packages is bound to package versions and
SHA256 values from APT-verified signed indexes (all three InRelease signatures
were separately verified with gpgv). The verifier restores those indexes,
verifies every package hash, and uses real apt --no-download to install gcc,
libc6-dev and curl. The unmodified official script then runs real APT again;
no command success is fabricated.
The original test script and assertions remain unchanged; original failures
and actual pytest counts remain in ignored calibration logs. No artifact replay,
agent environment alteration or paid model call is involved.

Use the task-local diagnostics/feal-differential-cryptanalysis/runtime.json with
the verified runtime-v2; the top-level runtime.json is only an identity report.
Evidence is kept under ignored qwen38-suite/{diagnostics,preflight}/
feal-differential-cryptanalysis. validation.json binds declared fixture/checker
bytes and 20 fresh-container checks; full preparation additionally requires the
10 Harbor calibration receipts.

From the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/feal-differential-cryptanalysis/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/feal-differential-cryptanalysis/case.json --run-fixtures
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/feal-differential-cryptanalysis/runtime.json --parent-id feal-differential-cryptanalysis --output evals/promptfoo/results/qwen38-suite/preflight/feal-differential-cryptanalysis
```

Verified preparation (2026-10-27): 27 unit tests and 20 fresh fixture checks
passed. Ten fresh Harbor trials returned parent and all four child oracle/nop
rewards of 1/0. The official parent executed exactly one test per trial: oracle
passed it (1.73 seconds), nop failed it; no skipped or fabricated test count.
The oracle compiled the actual official C extension. Preparation receipts were
checked again against current source, runtime, adapter and settings.

Original image: alexgshaw/feal-differential-cryptanalysis@sha256:bea93bafe9eab601ca58729f5735a13f8339f52055322d1cf840e3742b54a287,
linux/amd64. Registry manifest/config, all seven compressed layer hashes and
unpacked diff_ids match the local OCI export and RootFS. Resource probes
confirmed 1 CPU and 2147483648 bytes for the parent; nominal storage is retained
without claiming a writable-layer quota. Ignored acceptance.json records source,
image, checker, fixture and dependency hashes, exact commands, exit codes,
resource observations and valid test counts. Earlier dependency failures and
an online installation cancelled before tests are retained separately.

Clippy and score.sh test exited 0. Cargo dynamically claimed shared slot 1;
CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR were both
/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1, with one build
job. The owned lock was released and caches retained. No Rust behavior tests
were run. Clippy retains the existing block 0.1.6 future-compatibility notice.
red.log records 23 tests with 28 failing subcases against the intentionally
incomplete checker; the final checker passes all 27 tests. Only this case's
assets are committed; official sources and shared evaluation settings remain
unchanged and no paid model was invoked.
