# Task 22 resource and prompt fixture

VibeWindow-authored Python JSON-RPC server, MIT (repository license). It runs locally through
production stdio connections; it is not an external-service acceptance substitute. Packages are
copied into isolated temporary workspaces and remain unchanged. Request journals are outside them.

Frozen reference: `codex-rs/codex-mcp/src/resource_client.rs` (`McpResourcePage`,
`McpResourceReadResult`, `McpResourceClient::list_resources`/`read_resource`) and
`codex-rs/codex-mcp/src/pagination.rs` (`collect_paginated_with_limit`). Ordinary catalogs cap at
100 pages, 2048 entries and 65536-byte cursors with a 30-second aggregate timeout. No hosted limits
or hosted event subscription authority are imported. Resource subscriptions remain unsupported.

The local rmcp 0.8.5 model supplies `GetPromptRequestParam`, `GetPromptResult`, `PromptMessage`
and the user/assistant role boundary. The task explicitly requires host prompts/list/get support;
resource support alone is not evidence of upstream Codex prompt integration. Prompt messages are
returned as untrusted structured content; their roles do not inject messages into the conversation.

Cases use a two-page resource/template/prompt catalog, byte-exact opaque URI forwarding, text/blob
contents with MIME and metadata, string prompt arguments, user/assistant messages and embedded
resources. Malformed roles/content, repeated/oversize cursors, catalog limits, missing capability,
timeouts, disabled servers and two plugins sharing an advertised server/URI are rejected or kept
isolated. `file:` URIs are sent to the selected MCP service; the host never opens those paths.
