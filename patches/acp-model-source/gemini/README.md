# Gemini CLI 0.66.0 managed history patch

The patch targets the published Apache-2.0 `@google/gemini-cli@0.66.0`
bundle. Exact original and resulting SHA-256 values are in `hashes.json`.
It adds a hidden managed startup flag and a separate project state root.
Managed model configuration starts from the selected client model instead of
native Gemini thinking/topK presets, and the resolved model remains locked for
primary, helper and compression requests. The transport removes the CLI-inserted
`skip_thought_signature_validator` sentinel. It preserves opaque provider
signatures and fails explicitly for unsupported cross-protocol signatures.

Native 0.66.0 stores resumable ACP conversations below `GEMINI_CLI_HOME` together
with credentials/settings. A request-owned launch directory is removed when its
relay lease ends. Managed launches therefore require a separate private history
root keyed by tenant/project/session/provider/model/generation. The new
`--shiju-model-source` flag is rejected by an unpatched CLI; the patched CLI
requires `SHIJU_GEMINI_HISTORY_ROOT`. The driver owns and validates that directory.
The patch redirects only project temporary state, not OAuth credentials or global
settings. `cli_local` receives neither the flag nor the history environment.

Apply only to a disposable installation, never to the user's global CLI:

```sh
npm install --prefix target/acp-gemini-runtime --ignore-scripts --no-audit --no-fund --save-exact @google/gemini-cli@0.66.0
patch -p1 -d target/acp-gemini-runtime/node_modules/@google/gemini-cli < patches/acp-model-source/gemini/gemini-cli-0.66.0-history.patch
```

Verify each file against `hashes.json` before/after applying. Use that
installation's `node_modules/.bin` for offline acceptance. The managed registry
installs this artifact in its versioned revision directory.

The managed v3 scanner appends `SHIJU_GEMINI_SKILL_ROOT` after the native
project discovery roots, inside Gemini's existing trusted-workspace branch.
An empty selection keeps native discovery. A selected name that collides with
a native skill fails discovery explicitly. Gemini retains its native skill
settings, activation and file reader.

In `session_model`, an `activate_skill` call that requires native confirmation
offers one-time approval and rejection. The host accepts this structured
confirmation only from the verified bundle during the active session. Gemini's
policy engine still decides whether to allow, deny or request confirmation.

The managed v2 runtime adds structured ACP permission metadata for native MCP
tool invocations in `session_model` and offers only single-call allow and reject
options. ShiJu accepts the metadata only from a verified managed executable, for
a server mounted in the active session. MCP tools retain their generic ACP tool
kind, so a tool name or a server-provided read-only hint cannot activate
read-only auto-approval. Native workflow confirmations keep their existing
policy.

Gemini must already trust the workspace before its native MCP discovery runs.
An operator can point the managed process at an explicit native trust file using
`GEMINI_CLI_TRUSTED_FOLDERS_PATH`. The installer does not create trust entries or
change a user's existing Gemini settings.
