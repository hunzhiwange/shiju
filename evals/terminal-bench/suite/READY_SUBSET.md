# Independent ready-subset experiments

Maintenance status: **已完成，待回归** — implementation and focused Python
checks complete; final host Clippy awaits a stable shared cache.

The default `freeze` and scope-less manifest validation still require the frozen
89-parent catalog and all 356 children. Ready-subset is a separate, explicit
experiment; it does not overwrite `qwen38-tb21.json` or turn missing tasks into
passes. No new paid attempts or real experiment manifests were created while
implementing this interface.

## Freeze after preparation is complete

Task 99 may use this entry only after all feasible preparation and required
fresh revalidation have finished:

```bash
PYTHONPATH=evals/terminal-bench \
  evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run freeze-ready-subset \
  --catalog evals/terminal-bench/suite/catalog.json \
  --cases-root /absolute/suite/cases \
  --runtime /absolute/first-case/execution.json \
  --runtime /absolute/second-case/execution.json \
  --selection /absolute/approved-selection.json \
  --output /absolute/new-independent-experiment
```

`--runtime` may repeat. Configurations must retain the same absolute
`historical_root`, `source_root`, `cases_root`; every explicit parent/child row,
including recovery paths, is preserved. Identical duplicate rows are deduplicated;
conflicting duplicate task IDs or different root strings are rejected. Historical
runtime reports are not execution configurations and are never rewritten.
Configuration file locations and other parents' rows do not enter a parent's
normalized preparation fingerprint.

The selection document has exactly `kind`, `selected`, `excluded`:

- `kind` is `ready_subset`.
- `selected` is a nonempty list of unique official parent IDs. Each contributes
  its parent plus all four children, in catalog order.
- `excluded` has one object per remaining parent, with exactly `parent_id`,
  `status`, `reason`, `evidence`. Status is `blocked`, `platform_rejected`, `stale`,
  `not_ready` or `calibration_failed`; reason is nonempty. Evidence is a nonempty
  list of `{path, sha256}` descriptors pointing to existing absolute files.
  Use immutable archived failure evidence, not a temporary task note that may
  later be deleted. Missing, altered or duplicate proof paths are rejected.

Selected and excluded must partition all 89 catalog IDs without duplicates,
overlap or foreign IDs. Selection is never inferred from whichever cases pass.
Every selected preparation must pass the same current source/runtime identity,
normalized settings, task bytes and ten oracle/nop first-score checks as full
freeze. Missing, stale or failed selected evidence aborts the freeze. The new
output directory must not already exist. Exclusion evidence remains SHA-bound
when workers validate the resulting manifest.

`run`/`resume` retain the existing frozen-manifest, first-attempt and global
no-replay barriers. An unselected group is rejected before output admission;
the worker also checks membership before reading the profile or starting a
Trial. Parent/model/diagnostic budgets and original resource/time limits are
unchanged. The platform-rejected tasks 12/27/38 were not executed, retried,
cleaned or moved to another session by this work.

## Report without changing the full denominator

```bash
PYTHONPATH=evals/terminal-bench \
  evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.report \
  --manifest evals/terminal-bench/suite/catalog.json \
  --experiment /absolute/new-independent-experiment/manifest.json \
  --results /absolute/independent-history.jsonl --audit-only
```

The report retains 89 original-parent / 356-child denominators and adds
`experiment_scope` with exclusions plus `selected_execution.complete`. Records
must match the frozen experiment ID, selected task/kind, variant descriptor,
adapter source SHA and runtime SHA. Compatibility variants remain separate from
original official scores. CLI source hashes include the independent manifest.
Default report acceptance still requires full official/diagnostic coverage;
`--audit-only` is the existing inspection mode, not a declaration of full-suite
success. `selected_execution.complete` means all selected cases were scored,
not that they all earned reward 1.

## The second authorized compatibility variant (task 56)

The original task-13 descriptor and pre-Agent planarity/numpy proof remain
compatible. The only added variant is:

- Parent `mcmc-sampling-stan`, ID `mcmc-sampling-stan-compat-cmake3283`.
- Image `vw-mcmc-stan-cmake-compat@sha256:<64 lowercase hex>`.
- Base `alexgshaw/mcmc-sampling-stan@sha256:073fc36a3347b4f6b43bd6e98339542c41ed531c771af8bee40ba10f00cfabaf`.
- CMake 3.28.3, with the exact seven package/version/architecture/body-SHA entries
  approved in `cmake-compatibility-proposal`: cmake, cmake-data, libarchive13t64,
  libjsoncpp25, librhash0, libuv1t64 and libxml2. The approved closure's canonical
  SHA is `5fb755bed7fb032a15d63c0eb495a19b2253e345db2181c621190cd5d416cebb`.

Task 56's owner, after actual download/build/provenance/C/U verification, supplies
`cases/mcmc-sampling-stan/variant/` with these five files:

1. `contract.json`: `variant_id`, `original_parent_id`,
   `counts_as_original_official:false`, `dependency_phase:image_build_before_agent`,
   `base_image`, `image`, `cmake_version:3.28.3`, plus `dockerfile_sha256`,
   `packages_lock_sha256`, `sha256sums_sha256`, `base_packages_sha256`.
2. `Dockerfile`: the approved immutable base; recipe bytes bound by the contract.
3. `deb-lock.json`: the exact approved package closure and base; no existing
   packages upgraded. Metadata status does not confer authorization or prove
   package bodies were downloaded. Package names, versions, architectures,
   filenames and body hashes must match the hard-coded approved closure.
4. `SHA256SUMS`: the exact seven approved body hashes/filenames in proposal order,
   two spaces between hash/name and a trailing newline.
5. `base-packages.json`: the original base's complete package-name mapping to
   `{version, architecture}`, before adding the seven missing packages. This is
   provenance evidence to be collected by task 56, not fabricated by this tool.

The runtime row uses the existing `environment_variant` field with `{id,
contract_sha256}` or the fully resolved descriptor. It cannot combine a variant
with a startup override or verifier recovery. Materialization still changes only
the task name and selected image, preserving official task/checker/resource bytes.

The standard Docker backend selects the image before Agent, then runs a fixed,
read-only Python probe using `dpkg-query -W` and `cmake --version`. The stored
`environment-variant.json` must have `phase:before_agent` and facts containing
`cmake:"cmake version 3.28.3"`, the seven installed version/architecture pairs,
and `base_inventory_sha256`. The latter hashes compact sorted JSON of the
remaining installed package mapping; it must equal the original base inventory.
Additional/missing packages, upgrades, changed versions or missing proofs fail
closed. No arbitrary package/variant/probe script interface was added.

This implementation did not create the real task-56 files/image, install or
extract packages, run C/U checks, or claim calibration readiness. The owner must
still complete those authorized steps and standard oracle/nop preparation.

## Identity and validation

`execution_identity()` now includes `selection.py` and `report.py` as well as
all existing execution, variant and schema bytes. All old preparation receipts
and frozen execution manifests become stale under the new identity. Revalidate
with real current execution evidence; do not edit old hashes to make them pass.
The diagnostic checker implementation, case/checker/fixture/image bytes and
frozen ELF were not changed: checker-only `validation.json` remains valid when
those bytes match, while Harbor preparation/control evidence needs the new
execution identity. No historical proof or runtime report was rewritten.

Local red/green logs, exact test commands, the new identity and Clippy receipt
are under `evals/promptfoo/results/qwen38-suite/ready-subset/`. Tests explicitly
select synthetic modules/classes with no Docker, network or model calls; they
include full-gate regression, missing/stale selected evidence, exclusion
coverage/drift, merge conflicts, early worker rejection, original task-13
compatibility and the second variant's mocked metadata/start protocol. No real
experiment freeze or final regression was started. Cargo validation uses a
dynamically claimed shared slot, equal absolute target/build paths and JOBS1;
owned locks are released and caches retained.

Validation for this change (2026-10-28): 84 explicitly selected no-Docker,
no-network, no-model tests passed. Host Clippy is **pending final regression**:
its initial invocation exited 0 (`clippy.initial.log`), but the final invocation
exited 101 because `libhtml2md-6af5599bf612bdb3.rmeta` disappeared from the active
shared slot. The slot cache and lock directory were observed missing during that
validation; this work did not remove them. The wrapper subsequently reported the
already missing owner file during cleanup. Both raw logs are preserved; this is
not counted as a final passing check, and no unrelated Rust/cache repair was made.
The initial success applies only to the Rust inputs observed at that invocation;
no complete Rust-input snapshot was captured, and other Rust/Cargo changes were
present later. Input equivalence is therefore not asserted. No fresh-cache retry
was started; wait for stable shared caches before rerunning Clippy.

Both invocations dynamically claimed slot 1,
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`, with
CARGO_BUILD_BUILD_DIR equal to CARGO_TARGET_DIR and CARGO_BUILD_JOBS=1. The owned
Cargo processes finished; there is no surviving owned lock to remove. No cache
or other session's lock was deleted. No Rustfmt or full Cargo test ran. The
initial passing run had only the existing `block 0.1.6` future compatibility
notice. Original scope/variant red failures are retained separately.

Previous execution identity:
`ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1`

New execution identity:
`b35499b54cd173f716f856c17ecb9baf971ef1326c5b4ebc4656adf9d95fb5df`

This invalidates every old preparation/frozen-execution receipt using the previous
identity, not only newly selected parents. All 89 parent groups' Harbor evidence
must satisfy the new identity before admission; the checker-only exception above
does not imply that their old Harbor controls were retested. Task 56 can now
finish its own package/build/provenance work using the fixed protocol without
needing another shared execution change for this specific variant. Actual freeze
and paid execution remain deferred to task 99 after the remaining preparation
and required fresh evidence are complete.
