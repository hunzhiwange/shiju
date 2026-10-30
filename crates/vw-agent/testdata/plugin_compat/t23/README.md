# Task 23 capability negotiation fixtures

`server.py` is an original MIT-licensed JSON-RPC fixture. It runs through the
production owned stdio transport, captures the actual initialize request and
asks roots, sampling and elicitation requests while a real tools/call is pending.
It does not access a network, host files, credentials or model providers.

Frozen reference: `codex-rs/codex-mcp/src/rmcp_client.rs`,
`mcp_initialize_request_params`, requests `2025-06-18`; roots and sampling are not
advertised. `codex-rs/rmcp-client/src/logging_client_handler.rs` only implements
elicitation, not roots or sampling. `codex-rs/codex-mcp/src/client_capabilities.rs`
projects host-declared extensions and removes privileged verification/network
capabilities from ordinary plugin services. No such handler or trusted host
extension is supplied to the VibeWindow task 23 connection, so none is advertised.

The task's anticipated roots implementation is conditional on actual reference
support; the inspected snapshot has none. We retain explicit roots refusal and
never expose the process current directory, workspace or another session's root.
Actual elicitation and UI handlers belong to later tasks. The reference's
`2026-07-28` shared-connection commentary does not make that version implemented
by this client. A negotiated known fallback is retained separately from the
requested version; unsupported versions must fail before catalog/tool execution.

`http_server.py` is an original MIT-licensed loopback server. It returns JSON or
SSE initialize responses and requires the negotiated `MCP-Protocol-Version` on
subsequent POST, GET and DELETE requests. The JSON-RPC request still asks for the
frozen version while a known server fallback is kept for HTTP headers. This
checks connection-local state rather than a global version or the request's
preferred version. It logs only methods and protocol versions to an isolated
fixture directory; it receives no credentials.
