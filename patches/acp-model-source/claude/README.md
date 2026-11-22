# Claude 0.26.0 GNU model-only candidate

Use an isolated npm prefix with the pinned Claude ACP 0.26.0 adapter and
Claude Agent SDK 0.2.96. Apply `apply-model-only.py <package-directory>` to
that prefix. The script checks the upstream adapter digest and patches only
`dist/acp-agent.js`, alongside the new `dist/model-only.mjs` module. The
separate Rust verifier also checks the original adapter entry, SDK and engine.

The candidate consumes the existing private native Messages profile and the
host-verified `CLAUDE_CODE_EXECUTABLE` path. It starts Code in the private
settings directory, with a fixed model and an explicit environment. Its
SDK options disable built-in tools, project settings, hooks, skills, and MCP
discovery. It checks the SDK MCP status before accepting a session. The ACP
surface provides new, prompt, cancel and close. It rejects account
authentication, non-text input, slash commands, model or permission changes,
and history restoration. Ordinary account use keeps its own unmodified prefix.

Release wiring must verify this candidate with
`access_policy::claude::verified_linux_model_only_engine` and keep it separate
from general Claude capabilities. Exercise the offline native Messages route
in both GNU architectures before changing any published capability.

The next tier is one host-catalogued read-only MCP tool. The pinned SDK hook
exposes `tool_use_id`, but its SDK MCP control message carries a server name
and a JSON-RPC request without that tool-use ID. The native HTTP path likewise
has no Shiju call-ID contract. The GNU projection consumes grants by exact
name, arguments, and ID, so a final dispatch path must propagate the ID
before this tier can enter acceptance.

After Linux runtime verification is authorized, run the patch option
assertions with `node model-only.test.mjs` in this directory. Set
`SHIJU_TEST_CLAUDE_MODEL_ONLY_ENTRY` to the patched prefix's absolute npm bin
and run the exact identity filter `acp_platform_claude_contract_model_only_identity`
with `-- --ignored`. These checks supplement the offline request
acceptance through the actual host.
