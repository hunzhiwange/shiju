# Create-time build resources — 2026-10-28

The pinned Harbor Docker backend applies CPU/memory policy to runtime services,
then invokes `compose build` without builder cgroups. Its automatic egress
sidecar image build has the same gap. A later `docker update` cannot constrain
an earlier RUN. The original task-62 interruption and task-61 controller-local
proof remain unchanged under their diagnostic evidence directories.

`suite.docker:SuiteDockerEnvironment` is now the common backend selected by
`scoring.harbor_trial`: production workers, parent/child preflight oracle/nop,
and separate verifier recovery use it. No installed Harbor files are patched.

- Resolve Harbor's Compose configuration, retaining the same contexts,
  Dockerfiles, explicit/implicit tags, build arguments, target, platform and
  network settings. An unspecified build network remains unspecified; parents
  can still use their original build dependencies. Unsupported build features
  fail explicitly before execution; there is no unlimited fallback.
- Execute `docker build` with `DOCKER_BUILDKIT=0`, CPU period/quota, memory and
  swap limits supplied before any RUN. Use task/service budgets, not a fixed
  replacement for every parent's declared resources. Reuse Harbor's exact
  content/platform sidecar image tag and existing cache. Missing sidecar images
  are built through the same limited builder.
- Add only resource limits to the runtime Compose overlay and pass `--no-build`
  to `up`. Main uses its declared task/service limits. The egress sidecar keeps
  explicit limits or uses the existing calibrated 0.25 CPU / 64 MiB budget.
  Other services require explicit limits; unaccounted scaling is rejected.
- Serialize build/start admissions with one shared lock and inspect active
  containers before launch. Count both NanoCpus and classic CPU quota/period.
  Reject unbounded active work and aggregate reservations above 4 CPU / 8 GiB.
  The existing `dhb168-local-docker-*` business reservation remains separate.
  Busy admission fails before execution; it does not trigger a model retry.
- Archive resource admissions, build stdout and original errors. Nothing calls
  `docker update`. Task bytes, FROM/ref/platform, checker, startup override and
  Trial candidate selection are preserved.

Validation is archived locally under
`evals/promptfoo/results/qwen38-suite/build-resource-fix/`:

- `red.log`: actual shared entry selected `[None, None]` instead of the limited
  backend before the repair.
- `unit-validation.json`: 35 focused tests, including ten build/admission/hash
  tests, safe dotted IDs, preparation, SHA-bound CMD override, verifier bootstrap
  and scoring. `test-run.log`: 18 scheduler tests, including real oracle/nop and
  separate verifier-recovery containers. No model calls.
- `release-real/real-proof.json`: two real standard Harbor nop Trials against
  an offline cached amd64 fixture. The task-tree SHA is identical before/after.
  One actual RUN container was captured with CpuPeriod=100000,
  CpuQuota=100000, Memory=MemorySwap=536870912 at creation; the second build used
  cache and created no further RUN. Both fixture verifier rewards were 1.
  `runtime-inspects.json` records both main containers at 1 CPU / 512 MiB and
  both sidecars at 0.25 CPU / 64 MiB. Its execution identity is pinned below.
- Original development failures remain archived: an implicit Compose tag was
  initially missing from our builder mapping; two harness assertions expected
  the wrong lifecycle spelling/container count. A concurrent test launch was
  correctly refused by the shared resource lock; the final checks ran serially.
- `clippy.log`: `cargo clippy --all-targets --all-features -- -D warnings`, exit 0,
  dynamically claimed slot 1,
  `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`, with identical
  absolute CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR and CARGO_BUILD_JOBS=1. The owned
  lock was released and cache retained. Existing `block 0.1.6` future warning only.
  No Rust source changed; no full Cargo tests, fmt check, wasm or Makefile ran.

## Identity and fresh regression boundary

`state.execution_identity()` now hashes every byte of `docker.py` alongside its
callers. This covers builder limits, the classic-builder choice, cache/tag and
network handling, runtime limits and aggregate admission policy. A focused test
proves changing this helper changes the frozen execution identity.

- Previous execution identity:
  `d4653db1223aed8a0017e73b7bda8ed72e1dfc302b65432fdbc6f13a01f1a53a`
- New execution identity:
  `f759534dd79bbe107a90b14ddc75b553f285607cd7e9c3fd5ca10d841c43f8e5`
- Helper source SHA-256:
  `6339921b8b1ab7ad0715fee5c3937343037b359e6045b9b77419df0e97d89032`

All preparation receipts and frozen execution manifests using the old scheduler
identity are affected, across every parent and all four diagnostic children.
Their fresh Harbor execution/calibration evidence must be produced before
freezing again. Do not rewrite historical proof hashes or treat old bounded
controller runs as validation of this production backend.

The adapter transport source identity, frozen Agent/ELF, runtime-v2 identity,
official source/checkers, model budgets and case files are unchanged. Diagnostic
`validation.json` covers the direct, already limited checker-only Docker path;
it does not invoke this build backend, so its checker implementation/file hashes
remain valid. Parent/child Harbor calibration is a separate execution layer and
is covered by the new scheduler hash. `identity-impact.json` records exact
before/after identities; no old preparation or validation files were rewritten.
