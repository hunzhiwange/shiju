# siGit 1.7.10 host boundary

The fixed upstream source is built separately from the user's native installation.
`host-boundary.patch` removes failed filesystem/terminal callback fallbacks, asks
for host authorization on every tool invocation, and removes pre-approval file
previews, ambient tool/session hooks and automatic project instruction loading.
The built-in read/create/edit/multi-edit and foreground command tools use
host-backed execution. Explicitly selected stdio MCP tools can coexist with
them under the macOS read-only sandbox. Native fallback tools, automatic
skills/subagents and ambient MCP discovery stay disabled. Use the pinned
managed executable.

Build with the repository Cargo slot runner and `MISTRALRS_METAL_PRECOMPILE=0`.
This is the upstream-supported runtime Metal compilation path, not a disabled
permission check. Source and patch checksums are pinned in managed contracts.

Upstream v1.7.10 declares Apache-2.0 in its `LICENSE` and includes a `NOTICE`
with copyright and trademark attribution. Sources:
https://github.com/getsigit/sigit/blob/v1.7.10/LICENSE and
https://github.com/getsigit/sigit/blob/v1.7.10/NOTICE.
This is a modified, locally built managed artifact, not an unmodified official
release. The upstream protocol version remains 1.7.10; the separate source,
patch and binary digests in the build receipt identify the reviewed derivative.
Retain the upstream LICENSE and NOTICE when redistributing derived artifacts.

## Explicit stdio MCP on macOS

The managed installer keeps this revision beside the previous host-only
runtime. Built-in tools retain their existing approval and execution paths.
The explicitly selected service starts in the workspace with a cleared
environment, read-only access, and no network or subprocess creation.
Each MCP invocation uses the existing ACP host approval. An MCP error
becomes a failed tool event, and discovery with an empty tool catalog fails
session setup.

RustCodeGraph 1.2.10 requires usable SQLite WAL/SHM sidecars to open a
fresh index under the read-only policy. The private acceptance probe opens
a host-owned SQLite read transaction to prepare those sidecars.
