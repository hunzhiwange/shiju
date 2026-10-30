# MCP session lifecycle fixtures

`server.py` and `http_server.py` are authored for VibeWindow and use the repository's license.
They contain no user content, credentials, external URLs, or copied third-party implementation.
Each test creates its own temporary directory and local process. Marker files stay outside plugin
packages. `starts` records only fixture process/session identifiers.

The inputs exercise the real rmcp stdio, Streamable HTTP, and existing host SSE transports. The
stdio fixture advertises MCP 2024-11-05 and can delay initialization, the initial tool catalog, or
a tool call. The HTTP fixture maintains an actual `Mcp-Session-Id`; the SSE fixture uses the
legacy endpoint event and POST response channel. Portable plugins still reject `type=sse`.

Expected results are one initialization per live session/configuration, private legacy connections
between chats, bounded startup/call cancellation, source/configuration retirement, and process
reaping after the last owner releases its connection. A retained tool binding must not keep an
ended chat's lease alive. Local HTTP/SSE tests do not count as public-service integration evidence.

Frozen reference symbols are `McpConnectionPool` and `ConnectionLease::release` in
`codex-rs/codex-mcp/src/connection_pool.rs`, and `McpServerConfig` in
`codex-rs/config/src/mcp_types.rs`. The per-file SHA-256 values and executed evidence are recorded
in `docs/plugin-compat/evidence/t18.json`; the repository's frozen baseline remains unchanged.

The installed rmcp 0.8.5 protocol does not negotiate the reference snapshot's MCP 2026-07-28
cross-chat connection sharing. Legacy connections stay private; this task does not impersonate
that newer protocol or claim cross-account sharing, interactive input, subscriptions, or a cached
catalog readiness path. With no validated catalog cache, both readiness declarations use a live
connection and initial tool catalog before succeeding.
