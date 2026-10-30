# Task 7 discovery fixtures

The JSON fixtures and inline variations in `t07_tests.rs` are synthetic,
MIT-licensed test data authored for VibeWindow. They contain no upstream package
bytes, credentials or user content. Tests materialize them in temporary sources
and compare the original bytes after real discovery and installation calls.

| Fixture / behavior | Expected result | Frozen reference |
| --- | --- | --- |
| `native.json`: name, version, description | Existing native entry point remains supported | VibeWindow `load_plugin_manifest` |
| `legacy.json`: name | `.codex-plugin`, `.claude-plugin`, then `.cursor-plugin` entry points | `DISCOVERABLE_PLUGIN_MANIFEST_PATHS` |
| `portable.json`: schema, name | Recognized root `plugin.json` takes precedence over legacy; unknown versions fail without downgrade | `find_plugin_manifest_path`, `agent_plugin_schema_status` |
| Malformed selected package | Report its exact root and failure; retain unrelated valid packages | `load_plugins_from_layer_stack_with_scope`, `load_plugin` |
| Sources and identities | Workspace takes priority, same identity is stable, distinct marketplaces survive | Task 7 host source precedence; `PluginId::as_key` |
| Disabled marker and explicit sources | Exact disabled identity is filtered, no automatic Codex-home scan or package mutation | VibeWindow host policy |

Source snapshots are frozen in `docs/plugin-compat/baseline.json`. Task evidence
records their hashes. The native format remains ahead of legacy alternatives
when no recognized portable manifest exists, preserving the host entry point.
Source-root validation and cross-kind identity ambiguity remain explicit fatal
errors. Per-package failures are visible diagnostics and cannot enable that
package. No MCP execution, desktop or non-macOS acceptance is claimed here.
