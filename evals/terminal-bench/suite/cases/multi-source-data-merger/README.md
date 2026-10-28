# Multi-source data merger diagnostics

Four independent synthetic tasks prepare the data-processing parent:

| Child | Observable behavior | Incorrect controls |
| --- | --- | --- |
| m1 | Read JSON, quoted CSV and genuine Parquet; normalize schema/types | omitted Parquet, string IDs, wrong boolean status |
| m2 | Match integer-normalized stable IDs and distinct source membership | merge by name, omit tertiary-only user, duplicate normalized ID |
| m3 | Per-field priority, null fallback and present empty strings | reverse priority, empty as null, omit conflicts |
| m4 | Complete typed Parquet and exact JSON conflict report | drop unique user, wrong ID type, empty conflict report |

The checker executes `merge.py ROOT` on new directories with reordered records
and shifted IDs. Independent literal expectations are verifier-only. It checks
complete rows, source membership, real Parquet schema and full conflict values;
no candidate self-tests, keywords or saved transcript count as evidence. Each
child receives only its synthetic public sample and complete specification.
Its explicit null/empty semantics do not change the original parent's scoring.
A red test omitted source_c membership and detected the resulting false
acceptance of m2/skip-c. Restoring full membership checks passed all 20 tests.

The tools-only amd64 image extends reviewed task52 single-manifest Python with
one pyarrow 22.0.0 installation layer. Its /app is empty; parent inputs, tests,
solutions, credential paths and secret environment names are absent. The frozen
SHA-verified x86_64 ELF passed real `--help` offline with read-only root/mount and
1 CPU/512MiB limits. Tools image:
`vw-multi-source-diagnostics:task62@sha256:c25e85c44903fc0ef8355e59d691d923ca36695066da97fc64481cb49f90bcf2`.
Standard children preserve the materializer's exact FROM/ref/task bytes and
1 CPU/512MiB preset, with 0.25 CPU/64MiB sidecars. All current controls invoke
public `suite.scoring.harbor_trial` and `suite.docker:SuiteDockerEnvironment`;
there is no private Trial, Compose, builder or main-platform override.

The complete upstream task stays unchanged. Parent image manifest:
`sha256:8b32782078ff7383a1b4e5d3cecca8ce287e30f50bb4c0e5db009a18064e666e`;
config `sha256:5096d3409e5c692afd62a136f90a2e52dd87653aa6637a3e1ca31ad30fab1096`.
Its 220,871,490 compressed bytes and 543,109,120 uncompressed tar bytes were
stream-verified against every compressed SHA256 and diff_id. The 2GiB U cap
was enforced during decompression, then original manifest/config/amd64/RootFS
were verified after streaming OCI import. No archive file was created.
Admission conservatively reserved three compressed copies, Ucap, original
S10+B2+A1+safety5 and task63's future 11GiB: 33,948,611,014 bytes in total,
of which this task's peak is 22,137,450,950 bytes (below its 28GiB ceiling).
Fresh host/VM snapshots were 50,620,604,416 / 80,562,110,464 bytes. Capacity
monitoring retained the other task's reservation throughout parent controls.
Original parent 1 CPU/2GiB, storage declaration 10GiB and 900-second timeouts
remain intact; storage reservation is not a claim of enforced writable quota.

The original-container dependency probe found apt/PyPI reachable but uv's
fixed installer URL returned 403. `environment/verifier-bootstrap.sh` injects
only SHA-bound real dependencies after the Agent: authentic uv 0.9.5
installer/release, a relocated CPython 3.13.7 from the reviewed tools image
(without site-packages), and 13 pinned wheels. A real original-container probe
verified SQLite 3.46.1, pytest 8.4.1, pandas 2.3.3 and pyarrow 22.0.0. The
unchanged official test.sh still invokes its real apt/curl/uv/pytest commands;
the exact installer request is served from verified bytes, other curl requests
use real curl, apt performs its real bounded operations, and authentic uv
resolves offline wheels. No Agent dependency/image or official scorer changes,
artifact replay, fake apt success or substituted reference solution are used.

Current evidence is in ignored
`evals/promptfoo/results/qwen38-suite/diagnostics/multi-source-data-merger/b35499b5/`.
All ten fresh production controls bind execution identity
`b35499b54cd173f716f856c17ecb9baf971ef1326c5b4ebc4656adf9d95fb5df`.
Parent-only verifier bootstrap was added after child calibration; final checking
proves every child setting, task byte, image, fixture/checker, source/runtime,
Harbor, adapter and scheduler identity unchanged. `preparation.json` binds
original scoring logs and is checked by production `verify_preparation`.
No paid model was called. Model/token policy remains unchanged.

The requested launcher with public historical runtime.json failed with
`KeyError: historical_root`; the task-local execution config references frozen
runtime-v2 and original timeouts. Historical failed/aborted and older-identity
controls remain separate. The initial tools build briefly lacked predeclared
limits before update; it is not claimed as bounded calibration. Current shared
backend builds constrain RUN containers at creation. The original raw
capacity-probe.json was overwritten during an earlier recovery; its original
numbers remain documented in historical task evidence, but its bytes were
not recreated or backdated. New stage receipts use distinct directories.

Final verification: the parent oracle passed all three official tests (reward 1)
and nop failed all three (reward 0), with no skips or infrastructure exceptions.
All four fresh child oracle/nop pairs scored 1/0. The unchanged 20 checker-only
controls were hash-verified and reused, and the focused unittest command was
rerun with 20 passing tests. Final `verify_preparation` passed the combined ten
controls; no owned containers remain. `cargo clippy --all-targets --all-features
-- -D warnings` exited 0 using dynamically claimed slot 1, identical absolute
CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`, JOBS=1.
Its lock was released and cache retained. No Rust files changed, and Clippy is
not counted as behavior coverage. Exact commands, exit codes and receipts are
in b35499b5/final-gate.json, focused.log and clippy-receipt.json.
