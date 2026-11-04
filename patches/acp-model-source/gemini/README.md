# Gemini CLI 0.66.0 managed history patch

The patch targets the published Apache-2.0 `@google/gemini-cli@0.66.0`
bundle. Exact original and resulting SHA-256 values are in `hashes.json`.
It adds a hidden managed startup flag and a separate project state root.
Managed model configuration starts from the selected client model instead of
native Gemini thinking/topK presets, and the resolved model remains locked for
primary, helper and compression requests. ACP, tools, permissions and
authentication remain upstream code. The transport removes only the CLI-inserted
`skip_thought_signature_validator` sentinel; opaque provider signatures remain
intact and unsupported cross-protocol signatures still fail explicitly. Native cli_local behavior is unchanged.

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
installation's `node_modules/.bin` on PATH for offline acceptance. Task 33 must
verify and distribute this artifact before advertising managed Gemini support;
this task does not modify the shared installer or capability manifest.
