# MCP result fixture

VibeWindow-authored fixture, MIT license (repository license). It runs a real local Python JSON-RPC process through the production stdio transport and returns deterministic, non-sensitive text, image/audio bytes, embedded text/blob resources, a resource link, annotations, result metadata and structured content.

Fields and expected preservation follow frozen `codex-rs/protocol/src/mcp.rs::CallToolResult` (`d0d4efa82f985b53c426c0b1026b3af8b8cd0c4041ce19dee9cf4789000d0f81`). The package files remain unchanged by calls. The bytes are protocol fixtures, not media rendering or public-service acceptance evidence. Remote names/status in `_meta` must never replace host provenance or tool failure status.

`http_server.py` exercises actual loopback HTTP JSON, HTTP response event streams and the legacy SSE endpoint. The stdio fixture also covers empty/explicit-null and 1.2 MB results, malformed content, extensions that resemble other response variants, an internal-carrier name collision, compatibility notifications, and the declared call timeout.

The host retains the complete response object in `ToolCallResult.data.mcp_result`, keeps existing content/structured-content aliases, and exposes protocol `_meta` only as host data. Model output retains mixed content and structured content together; an explicit null cannot fall back to host metadata. A pure unannotated text result keeps its existing text DTO. These checks do not claim native image/audio playback, external-service integration, resource subscriptions or other numbered tasks.
