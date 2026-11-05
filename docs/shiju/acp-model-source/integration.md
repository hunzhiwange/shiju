# ACP model-source integration

Only the fixed releases below have real model-request evidence on Darwin arm64. Other platforms and releases remain unverified. Five upstream entries remain unsupported; this is not a 22/22 delivery.

| Driver | Evidence release | Managed launch | Evidence |
| --- | --- | --- | --- |
| auggie | 0.36.0 | Provider registration rejected by fixed upstream | [auggie](evidence/auggie.md) |
| claude | 0.26.0 | native-release | [claude](evidence/claude.md) |
| codex | 2.5.0 | reviewed-patched-runtime | [codex](evidence/codex.md) |
| codebuddy | 2.179.3 | native-release | [codebuddy](evidence/codebuddy.md) |
| goose | v1.57.0 | reviewed-patched-runtime | [goose](evidence/goose.md) |
| vibe | 3.0.4 | reviewed-patched-runtime | [vibe](evidence/vibe.md) |
| hermes | 0.19.0 | reviewed-patched-runtime | [hermes](evidence/hermes.md) |
| snow | 0.9.14 | native-release | [snow](evidence/snow.md) |
| copilot | 1.0.105 | native-release | [copilot](evidence/copilot.md) |
| cursor | 2026.10.22-27dcf5f | No verified arbitrary CLI Provider injection contract | Explicit refusal; upstream gate remains |
| droid | 1.3.0 | ACP bidirectional transport/configuration isolation remain unverified | Explicit refusal; upstream gate remains |
| gemini | 0.66.0 | reviewed-patched-runtime | [gemini](evidence/gemini.md) |
| iflow | 0.5.19 | reviewed-patched-runtime | [iflow](evidence/iflow.md) |
| kilocode | 7.8.18 | native-release | [kilocode](evidence/kilocode.md) |
| kimi | 2.5.0 | native-release | [kimi](evidence/kimi.md) |
| kiro | 3.0.0 | native-release | [kiro](evidence/kiro.md) |
| opencode | 1.19.5 | native-release | [opencode](evidence/opencode.md) |
| openclaw | 2026.10.10 | native-release | [openclaw](evidence/openclaw.md) |
| pi | 0.0.22 | reviewed-patched-runtime | [pi](evidence/pi.md) |
| qoder | 1.1.82 | Authorized personal-account login and Custom Provider entitlement required | Explicit refusal; upstream gate remains |
| qwen | 0.26.7 | reviewed-patched-runtime | [qwen](evidence/qwen.md) |
| trae | 0.121.3 | Authorized platform login and fixed-version runtime-root isolation required | Explicit refusal; upstream gate remains |

## Installation and identity

The existing gateway ACP install action calls `vw_acp::install_builtin_agent`. Codex, Gemini, iFlow, Qwen, Pi, Goose, Vibe and Hermes use independent versioned runtime directories. Global CLI installations are preserved. Registry resolution selects the reviewed runtime after validation. Explicit custom executable overrides also require the reviewed patched identity.

The installer embeds the reviewed patches and checks original and resulting artifacts. npm archive integrity is checked; iFlow uses the dependency lock from its real acceptance installation. Vibe and Hermes use checksum-pinned wheels in relocatable Python 3.12 environments. Pi builds the exact source commit and verifies the resulting adapter/native engine digests. Goose builds the checksum-pinned upstream source plus reviewed patch; its build receipt records source, patch and resulting executable digests. Cargo inherits the held repository slot during this build.

Patched support is published only after the reviewed runtime is present. Public options report the exact evidence release and platform; unknown custom driver names remain unsupported for `session_model`. The native `cli_local` path bypasses model overrides.

## Session lifecycle

A tenant/directory/session lock covers preparation, replacement, prompts and writes to native controls. Busy writes and overlapping prompts are rejected. Non-secret source, Provider/model, generation and recovery record are persisted in the scoped gateway settings store. Source/backend changes create a new generation and native history identity. Same-source changes validate a fresh profile before stopping the old process. Initialization/configuration/record failures preserve the old committed selection and recovery anchor; no expired credential is persisted or restored.

Managed options hide native model selectors, including model-category config controls. Mode/effort controls keep the backend-advertised choices. CLI-local sessions retain native model choices. Disconnected sessions still expose the committed client model identity. Request-owned clients retire even when option readers hold references; dropping a request removes its cache entry and schedules actor shutdown. Profile destruction revokes the relay before removing private files or owned Gateway processes.

Standalone vw-acp recovery cannot launch a persisted `vw-session-model:` reference: the existing process boundary requires reissue by the owning gateway. Queued gateway prompts use the same request lifecycle; no relay credentials are serialized to queue-owner state.

## Verification

All commands below exited 0. No unfiltered Cargo test suite was used. Detailed non-secret request observations: [integration.json](evidence/integration.json).


| Check | Effective tests | Shared slot |
| --- | ---: | --- |
| `cargo test -p vw-agent -p vw-acp -p vw-api-types --lib --all-features acp_model_source_session_controls -- --nocapture` | 7 passed; explicit installation test ignored here and run separately | 3 |
| `cargo test -p vw-agent --lib --all-features acp_model_source_launch_profile -- --nocapture` | 6 passed | 3 |
| `cargo test -p vw-agent --lib --all-features acp_session_options_follow_capabilities -- --nocapture` | 3 passed | 3 |
| `cargo test -p vw-agent --lib --all-features session::llm::acp::session::session_tests -- --nocapture` | 22 passed | 3 |
| `cargo test -p vw-acp --lib --all-features acp_model_source_session_controls_real_reviewed_installer_and_registry -- --ignored --nocapture --test-threads=1` | 1 passed; all 8 reviewed runtimes and registry entries | 2 |
| `cargo clippy --all-targets --all-features -- -D warnings` | Exit 0, no warnings | 2 |

Installation acceptance requires an explicit absolute `VW_ACP_INSTALL_ACCEPTANCE_ROOT`; this run used a repository target directory, never global vendor installations. Initial creation called the same public installer from the built test executable after its parent Cargo exited, allowing the actual Goose Cargo build to inherit the held slot. The final Cargo invocation above revalidated all installed artifacts and registry resolution. Goose was actually rebuilt from the pinned source; no substitute binary was used.

The behavior red light executed one named managed-model choice test and failed because a native CLI model selector was exposed. Follow-up regressions caught missing API initializers and a test-cache synchronization omission; both were fixed. An initial real Goose run rejected the new compiler output under an isolated config root; verification now accepts its checksum-validated fixed-source build receipt. The final real request runs below use this reviewed executable.

`cargo fmt --all` exited 0 before the final checks. Slots 2, 3 and 4 were dynamically acquired; both Cargo directory variables pointed at the held repository `target/rust-slot-N`. All owned locks were released and caches retained. The existing macOS large `__eh_frame` linker message can appear in test/native builds; Clippy itself completed without warnings.

| Runtime | Session-model requests | CLI-local requests | Effective Cargo tests |
| --- | ---: | ---: | ---: |
| codex | 9 | 7 | 2 |
| gemini | 9 | 7 | 2 |
| iflow | 9 | 7 | 2 |
| qwen | 9 | 15 | 2 |
| pi | 14 | 13 | 2 |
| vibe | 11 | 8 | 2 |
| hermes | 11 | 8 | 2 |
| goose | 11 | 8 | 2 |

All 16 real Cargo acceptance entries passed: **156 model requests**, authentication correct, zero fixture errors, and the required lifecycle checks true. The generic production entries additionally reject controls during a real active model request and preserve the committed model/recovery anchor after a disabled replacement is rejected. Model-service HTTP output alone is simulated; supplier CLIs, ACP transport, tools, source routing, relay, persistence and process retirement run normally.

Next: task 34 consumes the source/capability projection in the desktop; task 35 performs the complete release matrix. The five refused drivers remain explicit upstream gates and do not count as supported.
