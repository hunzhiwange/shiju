# siGit 1.7.10 host boundary

The fixed upstream source is built separately from the user's native installation.
`host-boundary.patch` removes failed filesystem/terminal callback fallbacks, asks
for host authorization on every tool invocation, and removes pre-approval file
previews, ambient tool/session hooks and automatic project instruction loading.
Only host-backed read/create/edit/multi-edit and foreground command tools are
advertised. Native fallback tools and automatic skills/subagents are unavailable;
ambient MCP discovery is disabled. Never substitute the unmodified npm binary.

Build with the repository Cargo slot runner and `MISTRALRS_METAL_PRECOMPILE=0`.
This is the upstream-supported runtime Metal compilation path, not a disabled
permission check. Source and patch checksums are pinned in managed contracts.
