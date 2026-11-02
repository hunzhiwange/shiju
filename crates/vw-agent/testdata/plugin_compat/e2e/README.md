# Original-package lifecycle verification

The registered `compat::t61_tests` exercise production installation, discovery,
skill loading, MCP connections/catalog/calls, hook trust, CLI marketplace actions,
enabled-state persistence, transactional upgrade and removal. Source and installed
package SHA-256 maps must remain unchanged. Runtime state stays outside packages.

Public originals are under `t61/public`, as compressed archives of untouched package trees, with
retained MIT licenses. Tests unpack each archive into a temporary source directory; per-file hashes verify that storage compression did not change package bytes. `public/sources.json` pins upstream Git commits, archive
SHA-256 and every package file. Context7 files were also verified against Git blob
identities. No package was edited to adapt to this host.

| Class | Original | Observable result |
| --- | --- | --- |
| Skills | Superpowers 6.3.0 and 7.0.1 | SkillTool returns original verification instructions; disable hides the skill; upgrade publishes exact next-version bytes |
| stdio | Pi Agent Plugins smoke package | Official MCP Everything 2.1.0 is discovered and its echo tool returns the requested marker |
| HTTP | Context7 Codex package | Real public resolve-library-id call returns Tokio documentation candidates without a credential |
| Hooks | agent-harness-plugins notify | Untrusted hooks are absent; trusted UserPromptSubmit creates a session state directory in isolated XDG cache; removal preserves state |
| Marketplace | Superpowers development catalog | CLI registers the original catalog and installs, disables, enables and removes its package |

`t61/package` is additionally a repository-authored MIT local MCP fixture. Its
real subprocess writes a caller-selected temporary output file. It supplements,
and does not replace, the public-original evidence.

Acquire a shared Cargo slot and set both build directories as required by task 61.
Prepare the separately pinned runtime dependency (never install into a plugin):

```sh
npm install --prefix /tmp/shiju-t61-everything-deps --ignore-scripts --no-audit --no-fund @modelcontextprotocol/server-everything@2.1.0
XDG_CACHE_HOME=/tmp/shiju-t61-hook-cache \
XDG_CONFIG_HOME=/tmp/shiju-t61-hook-config \
T61_EVERYTHING_DEPS=/tmp/shiju-t61-everything-deps \
cargo test -p vw-agent --lib --all-features codex_plugin_compat_t61_ -- --include-ignored --skip codex_plugin_compat_t61_original_reports_missing_dependency --nocapture
```

Live tests are explicitly opt-in because they need public network access, an
external npm dependency and isolated XDG directories. The evidence run executes
them; a default run that ignores them is insufficient for acceptance. The missing
runtime scenario runs the same built test executable with `PATH=/usr/bin:/bin`,
filter `codex_plugin_compat_t61_original_reports_missing_dependency`, and `--ignored`.

These checks do not establish three-platform or desktop rendering acceptance.
The source acquisition and initial test authoring mistakes are not claimed as
production TDD red/green evidence. No production behavior was changed in task 61.
