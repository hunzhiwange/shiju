# large-scale-text-editing diagnostics

Independent synthetic CSV exercises cover reading and emitting the exact format
(m1), constructing three actual editing macros (m2), fresh repeated execution and
first/final row boundaries (m3), and a 4096-row batch with unique values (m4).
Every instruction gives the complete transformation and output path. Internal
spaces must survive trimming; field and row order, uppercase, delimiters, suffix,
and final LF are compared as bytes. Parent solution/test data are never copied
into these public inputs. No paid model is called.

The checker executes Vim on restored pristine inputs plus fresh short boundary
probes, independently computes expected bytes, requires a zero process exit,
and rejects missing artifacts. A narrow explicit macro grammar prevents shell,
file and scripting commands; expanded keys, rather than source characters, are
counted against the 200-key bound. Gold and three faulty implementations per
child cover field order, lost internal spaces, missing final row or cursor
movement before editing that skips the first row and corrupts row boundaries. All fixture controls use
fresh isolated containers via suite.diagnostics. Infrastructure exceptions are
not scored as intentional rejection.

The diagnostic image is the pure tool amd64 base
`vw-win311-base@sha256:7a6db18e11071ff5a05ea8259da78073bd5730b85d39399511385e2a684749d7`
plus Debian Bookworm Vim `2:9.0.1378-2+deb12u2`; its final digest is
`sha256:0d3e8435beffc1c4c74d368cacec98328c82912844f7eab0a70f1a8aa3ff3227`.
Actual RootFS ancestry, empty `/app`, tool versions and cgroup limits were
checked. Legacy builder HostConfig enforced quota/period 100000/100000 and
512MiB memory+swap. It added 13,912,955 bytes within a 512MiB peak reservation.
Each fixture uses a new offline one-CPU/512MiB container; scorer controls also
bound their egress sidecar to 0.25CPU/64MiB. No other task's data or answers enter
the tool image.

The unchanged official parent is linux/amd64 manifest
`sha256:719adca3f1388220546ce6a155eee56eff3c4fe318183100320606a210f6b59c`,
config `sha256:4ca0b429596fd62b9fab0a787e276ee7ee3faf2761b56473232327218c462c56`.
All eight compressed layer SHA256s and decompressed diff_ids were verified:
C=73,308,403 bytes, exact U=258,071,552 bytes, below the 512MiB U cap.
Five complete shared cached layers were read without modification; only three
missing layers were downloaded into this task's ignored evidence directory.
Admission reserved two compressed copies, the full U cap, original 10GiB
storage, 2GiB dependency cache, 1GiB archive and 5GiB safety margin, requiring
20,010,840,550 bytes against host 21,099,995,136 and VM 92,138,008,576 bytes.
A streamed OCI import avoided writing a redundant archive. Manifest/config,
platform and RootFS were checked after import. Writable-layer quota enforcement
is not claimed. Original parent resources stay 1 CPU, 2048MiB, 10240MiB storage,
1200-second agent and verifier. Source, reference solution and scoring stay
unchanged; runtime-v2 and public implementation identities are SHA-bound.

The real parent probe found Python 3.13.7, SQLite and Vim available, PyPI
reachable, but the original uv installer URL returned HTTP 403. Only after the
agent exits, `environment/verifier-bootstrap.sh` supplies the complete authentic
uv 0.9.5 installer/release and pytest wheels via SHA-bound caches. The precise
installer URL is served from its cached bytes; all other curl requests delegate
to real curl. The original installer still executes and downloads its real
release via file://. Original apt-get commands retain real statuses and a
90-second bound. Offline uv resolves the original system Python and pytest
8.4.1/pytest-json-ctrf 0.3.5 wheels. An independent dependency probe actually ran
apt, installer, binary SHA checks, uv and pytest successfully. Its first probe
with all Linux capabilities dropped failed at apt setgroups; the corrected
probe matches Harbor's actual default capabilities and preserves that failed
infrastructure evidence. No official checker or agent environment was modified.

The task-specified suite-level `runtime.json` is a historical identity report,
not a launcher configuration (missing `historical_root`); its prescribed
preflight command exited 1 before calibration. The ignored task-local
`diagnostics/large-scale-text-editing/runtime.json` supplies verified generation-2
runtime/source/case roots and parent-only verifier dependency settings. Fresh
child controls were run before adding that parent-only setting; their unchanged
bytes, child settings, image ancestry and scorer identity are explicitly checked
when assembling final evidence. No child control receives the parent bootstrap.

Evidence, exact commands, source/image/dependency hashes and original logs live
under ignored `evals/promptfoo/results/qwen38-suite/diagnostics/large-scale-text-editing/`.
`validation.json` binds 20 independent fixture controls. Final production `verify_preparation` passed against all ten fresh scorer controls.
The official parent oracle collected/passed all 5 tests (reward 1); nop collected
and failed all 5 tests (reward 0). Neither skipped tests, raised a Harbor
exception or timed out. All four child oracle/nop pairs scored 1/0 respectively;
20 focused tests and 20 separate fixture controls passed. `preparation.json`
binds final inputs and every original scorer evidence file. All task containers
and the Cargo lock were removed, retaining their verified caches.

Host-only development evidence (not container calibration): the deliberately
weakened artifact-existence checker failed 12 of 20 controls. An initially
ineffective negative with trailing cursor movement was detected by 3 failing
controls and replaced with movement before editing; all 20 real-Vim host
controls then passed. Logs remain in the ignored diagnostics directory.

Repository gates passed: `evals/promptfoo/score.sh test` and
`cargo clippy --all-targets --all-features -- -D warnings` (exit 0).
Clippy used dynamically acquired slot 1,
`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`,
matching `CARGO_BUILD_BUILD_DIR`, and `CARGO_BUILD_JOBS=1`; the lock was released
and cache retained. No Rust source changed. These gates run zero behavior tests.

Focused commands (from repository root):

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/large-scale-text-editing/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/large-scale-text-editing/case.json --run-fixtures
```

The task-local `calibrate-children.py`, `calibrate-parent.py` and `finalize.py`
archive exact fresh controls and feed the same production inputs and
`verify_preparation` gate used by preflight. The already verified eight child
controls were reused after proving that only the parent-only verifier setting
changed, preserving all child settings and bytes. A corrected future full
preflight uses the ignored task-local `runtime.json`; the suite-level historical
identity report remains unchanged. Official, diagnostic, fixture calibration
and the invalid-launcher failure remain separate evidence categories.
