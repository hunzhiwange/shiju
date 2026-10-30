# Task 3 legacy manifest fixture

`legacy.json` is a synthetic MIT-licensed fixture authored for this repository;
it contains no upstream package bytes, credentials, or user data.

Expected fields follow the frozen Codex source at
`codex-rs/core-plugins/src/manifest.rs` (`RawPluginManifest`,
`RawPluginManifestInterface`, `resolve_raw_plugin_manifest`, and
`resolve_default_prompts`), SHA-256
`cb6cc5e8e2004d1533141a3f11f3460a1aa8c2e0295c9583ae9726c6a75b2802`.
The missing version/description remain absent, websiteUrl accepts its camelCase
spelling, and defaultPrompt becomes one whitespace-normalized prompt.

Tests create isolated package copies and verify their original bytes remain
unchanged. The three legacy entry points follow
`DISCOVERABLE_PLUGIN_MANIFEST_PATHS` in
`codex-rs/exec-server-protocol/src/protocol.rs`.

Task 3 covers declaration parsing. Multiple skill directories and inline
MCP/Hooks declarations remain distinct values; their runtime consumers report
unsupported execution until the corresponding later task implements it.
The fixture never executes its illustrative command names.

The inline JSON samples in `t03_tests.rs` are authored under the same fixture
license and provenance. Their expectations are:

| Input family | Expected declaration | Frozen reference symbol |
| --- | --- | --- |
| Absent/blank name, optional metadata | Root basename fallback; version trimmed, description preserved | `resolve_raw_plugin_manifest` |
| Single/list skills | Ordered audited paths | `RawPluginManifestPaths`, `resolve_manifest_paths` |
| MCP path/object | Distinct file or unchanged inline server map | `resolve_manifest_mcp_servers` |
| Hooks path/list/object/object list | Ordered file or inline sources | `resolve_manifest_hooks` |
| Apps/onboarding | Root-bound resource paths; onboarding accepts optional `./` | `resolve_manifest_path`, `resolve_openai_onboarding_skill` |
| Interface URLs and dark logo | Both documented URL spellings; audited dark-logo resource | `RawPluginManifestInterface` |
| Prompt string/list | Whitespace collapsed; 128 Unicode characters; first three valid entries | `resolve_default_prompts` |
| Null interface lists | Explicit rejection for capabilities/screenshots | `RawPluginManifestInterface` |

VibeWindow retains its existing static audit policy: selected manifest entries
and component paths cannot traverse symlinks, and referenced paths must exist
inside the package. Unsafe/missing paths fail visibly rather than adopting
Codex's warning-and-ignore path behavior. These stricter host checks remain a
recorded compatibility difference for task 5 and final acceptance.
