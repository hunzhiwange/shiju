---
name: demo-plugin-guide
description: Use when demonstrating or verifying VibeWindow local plugin packaging, including plugin skills, MCP servers, hook declarations, and interface metadata.
---

# Demo Plugin Guide

This skill explains how the demo plugin is wired.

## Surfaces

| Surface | File | Purpose |
| --- | --- | --- |
| Manifest | `.vibewindow-plugin/plugin.json` | Declares the plugin package and component paths. |
| Skill | `skills/demo-plugin-guide/SKILL.md` | Shows plugin-packaged skill discovery. |
| MCP | `.mcp.json` | Registers a local stdio MCP server. |
| Hooks | `hooks/hooks.json` | Declares lifecycle hooks that require trust before runtime use. |
| Interface | `assets/*` and `interface` metadata | Provides display name, prompts, brand color, and local assets. |

## Quick Checks

Run plugin audit:

```bash
cargo run -p vw-cli -- plugins audit plugins/vibewindow-demo-plugin
```

Use the plugin settings surface:

```text
System Settings -> Skills -> Plugins -> Refresh
```

## MCP Demo Tools

The bundled stdio MCP server exposes:

- `demo_echo`: returns the supplied message.
- `demo_sum`: returns the sum of numeric values.

## Hook Demo

The hook script accepts JSON payloads on stdin and writes compact diagnostics to stderr. It does not mutate files or call the network.
