# VibeWindow Demo Plugin

This plugin is a compact reference package for the plugin surfaces currently read by VibeWindow.

It includes:

- `.vibewindow-plugin/plugin.json` with required metadata and interface presentation fields.
- `skills/demo-plugin-guide/SKILL.md` for plugin-packaged skill discovery.
- `.mcp.json` with a local stdio MCP server.
- `scripts/demo_mcp_server.py` implementing `initialize`, `tools/list`, `tools/call`, `resources/list`, `resources/read`, and `prompts/list`.
- `hooks/hooks.json` plus `scripts/demo_hook.py` for lifecycle hook declarations.
- `assets/` referenced by the interface block.

Runtime notes:

- VibeWindow discovers workspace plugins from `plugins/<name>` and `.vibewindow/plugins/<name>`.
- The MCP server command is written for this repository as the active workspace. If you copy this plugin into a global plugin directory, update `.mcp.json` to point at the copied `scripts/demo_mcp_server.py`.
- Hooks are declared here, but execution still requires the existing plugin hook trust flow.
- App manifests are not included because the current VibeWindow plugin loader does not read an `apps` field.

Useful checks:

```bash
cargo run -p vw-cli -- plugins audit plugins/vibewindow-demo-plugin
```
