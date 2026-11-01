# MCP hook fixture

Original repository-authored Python standard-library HTTP JSON-RPC server; repository license applies.
No credentials or user data. `initialize` returns protocol 2024-11-05 and tools capability;
`tools/list` advertises read-only `echo`; `tools/call` records exact arguments outside the
plugin package and returns a text JSON hook output with `additionalContext: fixture-context`.

Frozen reference: codex-rs/hooks/src/engine/mcp_runner.rs `run_mcp_tool`,
`expand_mcp_argument_template`; engine/discovery.rs SessionEnd handling.
The local HTTP endpoint is a real MCP transport fixture, not public deployment acceptance.
Host source validation and normal executor approval remain active; no bundled allowlist.
