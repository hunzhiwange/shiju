# Task 5 component path audit fixtures

`root_aliases.json` and the inline JSON in `t05_tests.rs` are synthetic fixtures
authored for this repository under MIT. They contain no upstream package bytes,
credentials or user content. Tests write them into temporary package roots and
load them through `load_plugin_manifest`, without executing declared commands.

Frozen sources (local snapshot, no Git revision asserted):

- `codex-rs/core-plugins/src/manifest.rs`, SHA-256
  `cb6cc5e8e2004d1533141a3f11f3460a1aa8c2e0295c9583ae9726c6a75b2802`:
  `resolve_manifest_path`, `resolve_manifest_paths`,
  `resolve_interface_asset_path`, `resolve_openai_onboarding_skill`.
- `codex-rs/codex-mcp/src/agent_plugin_config.rs`, SHA-256
  `08a59340a5264849033ffaf0945152cadf09fb8ca98ca7ee44727e7e7918954a`:
  `resolve_existing_path_prefix`, `resolve_contained_host_path`,
  `is_portable_path_suffix`. Runtime MCP root/data expansion is task 16;
  this task only audits manifest component resources.

| Input / source | Expected host audit | Reference / intentional difference |
| --- | --- | --- |
| `./` and aliases containing only `.` / separators | Reject a root-only component | `resolve_manifest_path`; aliases cannot stand in for a resource |
| Files, directories, arrays, interface assets, onboarding | Every explicit path remains inside its own canonical root | The manifest path resolution symbols above; onboarding still accepts an omitted `./` |
| Any `..`, including escape followed by re-entry | Reject before filesystem traversal | `resolve_manifest_path`; extends the same policy to native and interface paths |
| Absolute paths, Windows drive roots and backslashes | Reject with the field named in the error | Host rejects Windows-ambiguous spelling on every platform, rather than interpreting it as a POSIX filename |
| Existing, internal, external or broken symlinks | Reject each symlink component | Stricter existing host policy; Codex manifest resolution is lexical and MCP runtime canonicalization can follow contained symlinks |
| Missing explicit resource | Reject; never silently omit a declaration | Stricter existing host policy than Codex warning-and-skip / lexical resolution |
| Missing implicit portable `skills/` or `mcp.json` | Retain the fixed root-local convention without creating it | Task 4 convention; actual skill/MCP use remains tasks 9/15/16 |
| Inline MCP/Hooks objects | Retain as inline data, not a fabricated path; do not run commands | `resolve_manifest_mcp_servers` / `resolve_manifest_hooks`; execution and their own runtime paths remain later tasks |
| Parsing or rejection | Package bytes and adjacent packages remain unchanged | Real filesystem isolation assertions; no state or credentials are written into packages |

These are static admission checks. They do not grant runtime execution rights or
replace execution-time permissions. Cross-platform lexical cases exercised on
macOS do not substitute for actual Linux/Windows acceptance in task 64.
