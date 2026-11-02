# Anonymous public MCP acceptance

Task 62 uses the unchanged MIT Context7 original archive and file hashes locked in `crates/vw-agent/testdata/plugin_compat/t61/public/sources.json`. Dependency task 61 is completed.

Endpoint: `https://mcp.context7.com/mcp`, Streamable HTTP, no credentials. Official configuration and rate-limit guidance: https://context7.com/docs/resources/troubleshooting (checked 2026-11-02; recommends adding a key for higher limits). The remote service itself is mutable; this locks the plugin, not the deployed server.

The registered t62 tests install the original, merge server configuration, discover `resolve-library-id`, call it with Tokio/Rust query and assert Tokio output, then disable/remove it without changing source bytes. Separate tests reject unreachable transport and prove alpha removal preserves beta bytes. No mock is used for public acceptance. Existing behavior passed; no production red was obtained or manufactured.

Exact commands, exit codes, counts and slot are in `evidence/t62.json`; raw logs are in `evidence/t62/`. This covers macOS runtime only, not desktop rendering or other platforms.
