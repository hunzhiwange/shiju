# Task 6 identity fixtures

`plugin.json` and inline cases in `t06_tests.rs` are synthetic fixtures authored
for this repository under MIT. They contain no upstream package bytes, credentials
or user content. Tests copy native packages through the real installer, discover
actual installed directories and toggle/remove exact identities.

Frozen local sources (no Git revision asserted):

- `codex-rs/core-plugin-common/src/plugin_id.rs`, SHA-256
  `f044856953a080930ef6ce932c3d83e70c118cce192bae6739cc0e7c34614501`:
  `PluginId::new`, `PluginId::parse`, `PluginId::as_key`,
  `validate_plugin_segment`.
- `codex-rs/core-plugins/src/store.rs`: `PluginStore::plugin_base_root`,
  `PluginStore::install_with_version_and_manifest`, `PluginStore::uninstall`, SHA-256
  `ee77430b5dfcf3aa4dea129c423e29a5685b975cad8eea6f54abcddd5ef4e609`.
  This snapshot stores market and package separately; VibeWindow retains its
  existing flat installed-package roots, using `<plugin>@<marketplace>` for market
  packages and the unchanged manifest name for local packages.

| Input / operation | Expected outcome | Reference / boundary |
| --- | --- | --- |
| `sample.plugin` from `alpha` and `beta` | Independent stable keys, copies and discovery entries | `PluginId::as_key`; names alone cannot deduplicate markets |
| Plugin dots at start/end, repeated dots; invalid market ASCII/path segments | Explicit rejection before installation | `validate_plugin_segment`; both segments validate independently |
| Existing local `sample.plugin` | Retains `sample.plugin` identifier and directory | Host local compatibility; no new alias |
| Toggle/remove `sample.plugin@alpha` | Only that market package and state change | Store per-ID operations; package manifest is unchanged |
| Installation source and version | Persist in host sidecar metadata outside package | Store separates host state from original package |
| Local spelling colliding with a marketplace key across sources | Explicit ambiguity error; neither identity silently overrides the other | Host rejects ambiguous public keys while retaining existing local spelling |
| Invalid or mismatched host sidecar | Fail closed; do not infer another market or fall back to a name | Host identity/ownership boundary |

Task 6 covers materialized local package provenance and exact lifecycle targeting.
Network acquisition, marketplace catalog persistence, transactional upgrades,
skill/MCP/Hooks runtime names and real cross-platform acceptance remain their
mapped later tasks. No remote URL is fetched by these tests.
