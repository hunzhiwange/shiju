# Codex model-source evidence

Implementation complete; retain task 13 for the coordinated formal regression.
Host: macOS / Darwin arm64, 2026-11-04. Linux and Windows are unverified.
Only the model upstream is a fixture: the adapter, engine, subprocesses, file
reads and production Rust ACP request path are real. No paid model was contacted.

## Version and runtime boundaries

[Identity verification](codex/identity.json): published codex-acp **2.5.0**, actual
adapter-resolved Codex engine **0.165.0**. The patched adapter bundle is pinned to
SHA-256 `b47dda5f2a998dab3aaf97f7f7e48dc499bebf9906cd5c2bb103756ae5974c37`.
The [patch instructions](../../../../patches/acp-model-source/codex/README.md)
include original hash checking, application and marker verification. Wrong patch
hashes are rejected. Installation is repository-local under target; global CLIs
and user account/config directories were not modified.

`drivers/codex.rs::configure` uses private 0700 runtime directories, 0600 config,
a request-owned provider id, CODEX_CONFIG/MODEL_PROVIDER and a dedicated bearer
environment variable. The token is absent from config/argv. Conflicting explicit
environment and non-ACP arguments fail before publication. Lossy cross-protocol
transport is rejected before CLI launch; this driver requires a Responses model.
The selected `resolved.limit.context`, supplied by the profile owner through
`model_context_limit`, becomes native `model_context_window` rather than a global
minimum. That shared metadata wiring belongs to the profile owner's change.

Native state lives in a separate owner/0700-verified history cache under the
runtime's user-private OS temporary directory, keyed by the existing logical
launch reference (tenant/project/session/provider/model/generation). Symlinks and
broad permissions are rejected. Runtime credential directories are removed and
bearer access revoked after each request, while the same logical session can
restore its history. OS temporary-cache cleanup may remove that history; this is
not a promise of durable storage across OS cleanup. Provider/model/session changes alter the managed history namespace; cli_local
bypasses it. Application source-change session creation remains task 33 integration work. No account files are copied into the cache.

The fixed adapter patch binds ordinary, auxiliary, resumed and forked thread
requests and every turn to the selected model/config. It removes the hardcoded
auxiliary model, suppresses raw credential-bearing logs, clears Node/CODEX_PATH
injection, and limits the engine environment. The managed entry is fail-closed
on the [unpatched real engine](codex/unpatched-rejection.json), which exits 2 on
the sentinel flag. Native cli_local retains its command/args/env and model config.

## Root-level Responses compatibility repair

The serial window changed only `model_relay/responses.rs`, `responses_tests.rs`
and `responses_http_tests.rs`. Real Codex emits complete developer/user messages
with client-generated IDs, no item_reference and no previous_response_id. The old
native validator classified those IDs as unissued upstream references and
returned 403 before the model service.

The native boundary now removes bookkeeping IDs only from known, materialized
message/local-tool payloads and preserves their data. Incomplete/unknown payloads,
opaque reasoning, citations and genuine item/response references keep their
ownership checks. Real references still reject other tenants, projects, sessions,
providers, models and generations. The HTTP test verifies both forwarding of
inline data without IDs and refusal to forward forged references.

## Validation

| Check | Result |
| --- | --- |
| Original Codex red command | 3 named tests executed; missing-driver behavior fails; Cargo 101 |
| Inline-ID red command | complete inline request reproduces 403; true-reference negative test passes; Cargo 101 |
| `cargo test -p vw-agent --lib --all-features acp_model_source_codex -- --nocapture` | **6 passed, 0 failed, 2 explicit real-CLI entries ignored**, exit 0 |
| `cargo test -p vw-agent --lib --all-features acp_model_source_relay_responses -- --nocapture` | **6 passed, 0 failed**, including real HTTP/stream/cancel/reference isolation, exit 0 |
| `cargo fmt --all` | exit 0 before each Rust verification phase |
| Private fixture contracts | 3 passed, exit 0; these are fixture-honesty checks, not supplier proof |
| Patch syntax/application and actual resolved engine pin | pass; wrong hash rejected |

Red commands hit the intended behavior failures, not compiler errors or zero
tests. The pre-existing macOS test unwinder aborted after those panics, so they
are not represented as ordinary 1-failed summaries. The final focused build
reported the workspace test binary's large __eh_frame linker warning; no unrelated
linker/build configuration was changed. The coordinated global check must assess
that workspace condition. No Clippy success is claimed: the scheduler explicitly
reserves formal global Clippy/regression until all implementations finish.

## Real production request evidence

The [private entry](codex/verify_production.py) reuses task 10's unchanged
slot/version/native-config/process harness. Its vendor-specific ignored Rust
entries call **production do_stream_request_acp**. Only the upstream fixture,
test selectors and explicit Responses predicate differ from the shared generic
Chat/Messages entry. The [private manifest](codex/manifest.json) isolates native
config and injects forbidden native model/credential values in session-model.

The same newly verified production test artifact was used without another Cargo
rebuild. Reports explicitly say cached-production-binary, give the actual binary
command, binary_exit_code and effective test count. They do not claim the generic
shared Cargo acceptance entry has passed.

| Source | Effective tests / exit | Actual generation requests | Models and lifecycle |
| --- | --- | --- | --- |
| [session-model](codex/session-model-cached.json) | **1 / 0** | **9** | primary → switched; all lifecycle flags true |
| [cli-local](codex/cli-local-cached.json) | **1 / 0** | **7** | always native primary; all lifecycle flags true |

Every request is authenticated `/v1/responses`; fixture errors are zero and real
tool results reach two. New session, second turn, cancellation after model I/O,
restart/restore, model switching, isolation and handshake all pass. Directory
probes are not counted as generation. Neither snapshot contains credentials,
request bodies, private paths or supplier stdout/stderr. Fixed bundle/engine pin
verification was also run separately against the same installed artifacts.

[Direct managed evidence](codex/direct-managed.json) separately proves title
requests, fresh-model auxiliary routing, removal of the old runtime directory,
lease/provider rotation and restoration with history containing no token bytes.
[Direct local](codex/direct-cli-local.json) proves native auxiliary inheritance;
[missing credential](codex/direct-managed-missing-credential.json) exits 1 with no
model request. Suggestions share the patched turn gate but were not individually
triggered. The final context scalar is additionally covered by the production
request evidence above.

## Slots and formal regression

Final driver/context verification and its two cached production entries used
**slot 1**: `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`.
The Responses/HTTP verification used **slot 2** with
`/Users/xiangmin/worker/code/shiju/target/rust-slot-2`. Earlier red/diagnostic stages
used slots 4, 7 and 5 with the corresponding absolute rust-slot-N paths. All were
dynamically claimed by atomic mkdir; CARGO_BUILD_BUILD_DIR matched the held slot.
Own locks were released only after stage processes ended; caches were retained.

After the scheduler freezes all implementations, acquire a fresh shared slot,
set both absolute build-directory variables, prepend the pinned adapter's bin to
PATH, then run:

```sh
cargo test -p vw-agent --lib --all-features acp_model_source_codex -- --nocapture
cargo test -p vw-agent --lib --all-features acp_model_source_relay_responses -- --nocapture
python3 docs/vibewindow/acp-model-source/evidence/codex/verify_production.py --agent codex --mode session-model --manifest docs/vibewindow/acp-model-source/evidence/codex/manifest.json --output target/acp-model-source-evidence/codex/session-model-formal.json --offline-fixture
python3 docs/vibewindow/acp-model-source/evidence/codex/verify_production.py --agent codex --mode cli-local --manifest docs/vibewindow/acp-model-source/evidence/codex/manifest.json --output target/acp-model-source-evidence/codex/cli-local-formal.json --offline-fixture
cargo clippy --all-targets --all-features -- -D warnings
```

Task 33 still owns installer/shared capability publication. Only after the formal
checks pass should task 13 be deleted and support published for this verified
platform/version; this evidence does not assert 22/22 or other-platform success.
