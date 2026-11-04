# Codex ACP 2.5.0 model-source patch

Fixed published adapter: `@agentclientprotocol/codex-acp@2.5.0`.
Fixed actual adapter-resolved engine: `@openai/codex@0.165.0`.
Original `dist/index.js` SHA-256:
`15de6b2dd5203876a48b12861378bcf87040d623ab2159e05052dc9d54c32b16`.
Patched bundle SHA-256:
`b47dda5f2a998dab3aaf97f7f7e48dc499bebf9906cd5c2bb103756ae5974c37`.

The adapter passes `CODEX_CONFIG`/`MODEL_PROVIDER` to ordinary sessions but chooses
`gpt-5.6-luna` for titles/suggestions and starts their threads without the session
config merge. Auxiliary turns now inherit their native thread's model. Managed
mode additionally binds new/resumed/forked threads and all turns, including
collaboration settings, to the selected model/provider/config. Permissions and
sandbox fields remain under their existing controls.

Managed routing config stays in a disposable private CODEX_HOME. The actual
app-server uses SHIJU_CODEX_HISTORY_HOME for resumable state; only non-secret
routing fields and the selected model_context_window go through its verified
native global `-c` entry. The driver creates history directories with owner/0700
checks and rejects symlinks. No account files are copied. Only the dedicated
short-lived relay token and OS runtime enter the engine's environment. Raw adapter
logging is disabled; a missing relay credential fails startup. Neither config
files nor process argv contain the token; generated native overrides are checked.

Managed launches use exactly `cli --shiju-model-source`. An unpatched adapter
forwards this to the fixed engine, which rejects it with exit 2. The patched
entry starts ACP. This prevents silent partial support on unpatched adapters.
Native cli_local receives no managed flag, model override, or history override;
its auxiliary turns inherit its own native model configuration.

Install only into an isolated repository-local directory and check the original
hash before applying the patch. Do not patch an ambient/global installation:

```sh
npm install --prefix target/acp-codex-inspection/runtime --no-audit --no-fund --ignore-scripts --save-exact @agentclientprotocol/codex-acp@2.5.0 @openai/codex@0.165.0
shasum -a 256 target/acp-codex-inspection/runtime/node_modules/@agentclientprotocol/codex-acp/dist/index.js
patch -p1 -d target/acp-codex-inspection/runtime/node_modules/@agentclientprotocol/codex-acp < patches/acp-model-source/codex/codex-acp-2.5.0-model-source.patch
node --check target/acp-codex-inspection/runtime/node_modules/@agentclientprotocol/codex-acp/dist/index.js
target/acp-codex-inspection/runtime/node_modules/.bin/codex-acp --shiju-model-source-check
```

The marker is `codex-acp 2.5.0 shiju-model-source-v1`. The private production
verifier checks both the exact patched hash and marker, resolves the engine
relative to the adapter, and probes that actual engine's version. A plain
upstream --version probe is insufficient. Task 33 must distribute this exact
artifact before advertising support; this task does not edit the installer or
shared capability manifest.

Direct real-adapter probes (only the model upstream is a fixture):

```sh
python3 docs/vibewindow/acp-model-source/evidence/codex/probe_adapter.py managed
python3 docs/vibewindow/acp-model-source/evidence/codex/probe_adapter.py cli-local
python3 docs/vibewindow/acp-model-source/evidence/codex/probe_adapter.py managed-missing-credential
```

Managed probes delete the old runtime directory, rotate the token/provider lease,
restore the original thread, and change models with a separate history root.
They also verify that history contains neither token's bytes. Title requests are
observed separately; suggestions use the same gate but are not individually
triggered. See the [evidence](../../../docs/vibewindow/acp-model-source/evidence/codex.md)
for actual production-path requests and coordinated regression instructions.
