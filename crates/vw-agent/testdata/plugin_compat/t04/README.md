# Task 4 portable manifest fixtures

`portable.json` and the inline JSON cases in `t04_tests.rs` are synthetic,
MIT-licensed fixtures authored for this repository. They contain no upstream
package bytes, credentials, or user data. Tests copy them into temporary package
roots and assert unchanged bytes after loading through the public loader.

The frozen reference is `codex-rs/core-plugins/src/agent_plugin_manifest.rs`
(`parse_agent_plugin_manifest_uri`, `apply_codex_agent_plugin_extension`, and
`is_valid_agent_plugin_name`), SHA-256
`31f9efd68ad2a94e45d1596123d49066886c4592aa7588935170913ced1cc660`.
Root recognition follows `agent_plugin_schema_status` and
`find_plugin_manifest_path` in `codex-rs/utils/plugins/src/plugin_namespace.rs`,
SHA-256 `1515279deb99c997b407b987abae681ee6b3b3b8a7c7cdd1b33dd5a2ec240bad`.

Expected behavior: the supported schema selects root `plugin.json`, fixes
`skills/` and `mcp.json`, trims optional display metadata, and prefers the entire
inline `com.openai` object over the legacy overlay. Unknown schema versions and
invalid declared types fail; unrelated npm metadata does not become a plugin.
The package stays read-only and each load remains rooted in its own directory.
These tests cover declarations, without executing MCP or Hooks.

| Fields / input | Expected result | Frozen reference symbol |
| --- | --- | --- |
| `$schema` | Only the exact 1.0.0 URI is supported; recognized unknown versions never fall back | `agent_plugin_schema_status`, `parse_agent_plugin_manifest_uri` |
| `name` | 1–64 bytes; lowercase ASCII/digits/dots/hyphens; alphanumeric ends; no `..` or `--` | `is_valid_agent_plugin_name` |
| `version`, `description` | Optional strings, trimmed; blank becomes absent; explicit null rejected | `parse_agent_plugin_manifest_uri` |
| `author.name/email/url` | Optional string members, no unknown members or explicit null; only trimmed name affects interface | `RawAgentPluginAuthor`, `parse_agent_plugin_manifest_uri` |
| `homepage` | Trimmed default interface website URL | `parse_agent_plugin_manifest_uri` |
| `repository`, `license` | Validate optional string type, without adding runtime behavior | `RawAgentPluginManifest` |
| `keywords` | Preserve ordered string array, including blank entries; reject null/non-string elements | `RawAgentPluginManifest` |
| Unknown root fields and namespaces | Warn for unknown root fields; no inferred skills/MCP/agents behavior | `AGENT_PLUGIN_FIELDS` |
| `skills/`, `mcp.json` | Fixed root-level conventions even if absent; loading never creates them | `parse_agent_plugin_manifest_uri` |
| `extensions.com.openai` | Object replaces entire file overlay, including an empty object; non-objects are ignored | `apply_codex_agent_plugin_extension` |
| Overlay `apps`, `hooks`, `interface` | Only these declarations plus onboarding are applied; interface replaces defaults without merging | `apply_codex_agent_plugin_extension` |
| Inline `onboardingSkill` | Read directly from namespace, optional `./`, audited inside package | `resolve_openai_onboarding_skill` |
| Root/overlay/explicit component symlinks | Reject without fallback or following target; an unused overlay is not read | `find_plugin_manifest_path` plus VibeWindow host audit policy |

The host keeps its stricter existing explicit-path audit: declared overlay
resources must exist inside the package and cannot traverse symlinks. Component
and onboarding paths reject parent components; interface assets retain the
existing root-bound audit. Read/metadata failures are visible errors, rather than silently
ignoring a selected file. Only the two implicit, fixed root-level conventions
may be absent at parse time; their runtime consumers must still validate them.
Tasks 5, 7, 9, 13 and 15 retain the separate path, discovery, skill and MCP
execution acceptance. No desktop, public-service or non-macOS evidence is
claimed by this fixture.
