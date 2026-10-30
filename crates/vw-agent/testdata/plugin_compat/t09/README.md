# Task 9 fixture

These synthetic, token-free fixtures are authored for VibeWindow under the repository MIT license. Tests materialize their bytes in real temporary plugin packages; no external network or provider is used.

- `plugin.json`: legacy `RawPluginManifest.skills` string-array shape; roots are sorted and deduplicated by `plugin_skill_roots` in the frozen `codex-rs/core-plugins/src/loader.rs`.
- `alpha/SKILL.md`, `beta/SKILL.md`: YAML `name` and nonempty `description` from `parse_skill_frontmatter_metadata` in frozen `codex-rs/skills/src/parser.rs`. Full loads retain the original instruction bytes and relative resource references.
- Tests add malformed frontmatter, scripts, symlinks, portable defaults and disabled markers to copies. The static audit remains enabled according to the supplied runtime policy; rejected input is never executed or rewritten.
- Task 9 covers skill root discovery and provenance. Command migration belongs to task 11, invocation rules to task 10 and `agents/openai.yaml` metadata to task 12.
