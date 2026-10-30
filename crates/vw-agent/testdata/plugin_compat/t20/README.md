# Task 20 notification fixtures

The Rust `ServerHandler` in `plugins/compat/t20_tests.rs` is the real rmcp service.
`bridge.py` only copies newline-framed bytes between the production stdio transport
and that test-owned loopback service. It does not implement or substitute MCP.
Both files are project-authored MIT fixtures; no external account or user content
is used. Ports, processes and workspace directories are allocated by each test.

Expected behavior: unchanged advertised catalogs are cached per live connection;
tools/resources/prompts list-change notifications invalidate only their own
catalogs (resources include templates). Paginated refreshes never publish mixed
revisions, cyclic/churning catalogs fail, old frozen bindings cannot call changed
definitions or a replacement connection, admitted calls may finish, and shutdown
prevents further cache access. Servers without list-change advertisement are
listed afresh. Resource subscriptions remain explicitly unsupported.

Frozen reference: `codex-mcp/src/client_tool_catalog.rs` notification refresh and
`run_with_snapshot`; `codex-mcp/src/runtime.rs::invalidate_resource_caches`;
`codex-mcp/src/pagination.rs::collect_paginated_with_limit`.
The reference logging handler only logs resource/prompt list changes; this task's
explicit resource/prompt invalidation requirement is implemented at the host's
existing inspection boundary, without advertising resource subscriptions.
