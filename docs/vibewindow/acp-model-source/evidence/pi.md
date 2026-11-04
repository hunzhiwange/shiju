# Pi model source acceptance

Verified on **2026-11-05, Darwin arm64**, Node 26.13.0 / npm 11.20.0.
No macOS x86_64, Linux or Windows runtime result is inferred. Registry capability
publication and installation of the reviewed artifacts remain task 33.

## Supplier identity and patch

- Actual `pi-acp@0.0.22`, source commit
  `8be5f22769f71d759aa48d798416c56e8be101ef`, MIT.
- Actual `@mariozechner/pi-coding-agent@0.73.1`, npm gitHead
  `781152fc24841dc54b22284514604048ebe5e2c9`.
- [Fixed patch, build instructions, source/archive identities and SHA256 checks](../../../../patches/acp-model-source/pi/README.md).
  The patch only adds absolute `PI_ACP_STATE_DIR` to `getPiAcpDir()`; unset keeps
  the original user-directory behavior for cli_local. There is no replacement Pi agent.
- The driver rejects an unpatched/different adapter and a different native CLI
  executable before launch. The verifier independently checks genuine package
  metadata, both executable digests and native Pi's actual version output.
  pi-acp itself has no --version output; no fake version wrapper is used.

## Production boundary

`drivers/pi.rs::configure` creates separate private runtime models.json/settings.json
and a short-lived relay token in the child environment. No bearer token appears
in argv, model configuration or persistent history. Runtime configuration is
unique to each profile, so concurrent issue/reissue cannot overwrite active files.

Native session files and ACP session-map are separate persistent 0700 directories,
scoped to tenant/project/session and the session_model source. settings.json points
Pi's native sessionDir at the history directory. Model/token rotation retains
history while other source/session anchors remain separate. Symlinks and unsafe
owners/permissions are rejected. HOME is never replaced; conflicting explicit
HOME/USERPROFILE/PATH/Pi/Node configuration fails. Known ambient model-account and
proxy variables are cleared, and managed Pi disables resource downloads/version checks.

`AcpLaunchProfile::session_model_id` carries the real `session-provider/<model>` ID.
The production request applies it with ACP session/set_model before **every**
prompt, including restored sessions and native compaction. It replaces a stale
saved selection; project defaults cannot override it. The relay still owns
Provider credentials and protocol conversion. Missing credentials or a newly
disabled model do not fall back to a native account.

cli_local bypasses the profile and preserves the supplied command, arguments and
environment. Acceptance supplies an explicit private native configuration; changing
the client model still leaves native Pi's selected model unchanged.

Pi's built-in auxiliary path verified here is native `/compact`, including history
and split-turn summarization. A private fixture project sets keepRecentTokens=1 to
exercise it on a small session. HTTP observation recognizes the native summarizer
system prompt plus conversation envelope with no tools. The application also
requires the native `Compaction completed.` output, so a normal prompt containing
a marker cannot masquerade as compaction. Third-party extension subagents are not
claimed as verified.

## Current real CLI results

Both commands use the public manifest and genuine reviewed executables, located
through `target/acp-model-source-evidence/pi/runtime/node_modules/.bin` on PATH.
No global installation, HOME substitution, account login or paid upstream is used.
The local fixture only simulates the external model service.

| Source | Effective Rust tests / exit | HTTP requests | Native compaction requests | set_model RPC | Fixture errors |
| --- | --- | --- | --- | --- | --- |
| session_model | 1 / 0 | 14 | 3 | 7: primary 5, switched 2 | 0 |
| cli_local | 1 / 0 | 13 | 4 | 0 | 0 |

Every model request was authenticated and used `/v1/chat/completions`.
Managed requests used `acp-fixture-primary` and `acp-fixture-switched`; native
requests only used `acp-fixture-primary`, including after the client selection changed.
Both passed new session, second turn, request-started cancellation, process
restart/load, switch and isolation assertions. Both models' managed compaction
requests were observed. Managed acceptance deliberately supplied a forbidden
native model/credential, a conflicting project default, and a forbidden saved
model selection before the second turn; none reached the model service or model RPC.

The optional session/resume method returns -32601 on this adapter; the existing
client successfully uses session/load. Protocol diagnostics retain that fact;
zero errors in the table means zero model-fixture errors, not zero optional-method
negotiation errors.

Actual final shared slot: **2**.
`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`;
`CARGO_BUILD_BUILD_DIR` was the same absolute directory because this repository
also configures a separate Cargo build-dir. The slot was atomically claimed,
recorded with the holder PID, and released after all children ended; cache retained.

Recorded commands (inside that held slot):

```sh
cargo fmt --all
cargo test -p vw-agent --lib --all-features acp_model_source_pi -- --nocapture
cargo test -p vw-agent --lib --all-features acp_model_source_routing -- --nocapture
python3 -m unittest discover -s scripts -p 'acp_model_source*tests.py'
python3 scripts/verify_acp_model_source.py --agent pi --mode session-model --manifest docs/vibewindow/acp-model-source/cli-versions.json --output target/acp-model-source-evidence/pi/session-model --offline-fixture
python3 scripts/verify_acp_model_source.py --agent pi --mode cli-local --manifest docs/vibewindow/acp-model-source/cli-versions.json --output target/acp-model-source-evidence/pi/cli-local --offline-fixture
cargo clippy --all-targets --all-features -- -D warnings
```

Focused Pi tests **5 passed**, routing tests **10 passed**, script tests **33 passed**;
all exits 0. Final Clippy exit 0 (14.02s after the non-Unix unused-parameter guard). Pi behavior tests cover a real child-process
HTTP request using generated configuration, token revocation/rotation and history,
config/executable/symlink rejection, and production missing-credential/disabled-model
checks. They remain profile tests, distinct from the real CLI results above.

TDD evidence: the original driver failed one named test with its explicit
unverified-launch error (slot 3, exit 101). The integrated real test then failed at
the RPC-binding assertion (one effective test, Cargo exit 101 / script exit 1,
slot 2): native defaults could make valid model requests but emitted no set_model
RPC. Adding the launch binding produced the seven observed RPCs and a green test.
Earlier independent profile stages used slots 3 and 4; current final verification
used slot 2. All target directories were the corresponding absolute rust-slot-N paths.

Patch state-directory verification also passed. The initial upstream npm test run
was 46/48 passing, with two auth-dependent fixture failures; no clean upstream
unit-suite claim is made. Rust test linking can emit the repository's existing
large __eh_frame warning on macOS; final Clippy produced no warnings.

Evidence JSON/logs are under `target/acp-model-source-evidence/pi/`. No raw supplier
stdout/stderr, request bodies or bearer tokens are persisted by the verifier.
Next: task 33 must install the exact pinned patch/native engine and wire registry
capabilities; task 35 must run the complete supported-platform matrix.
