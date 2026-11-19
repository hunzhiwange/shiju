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

Upstream v1.7.10 declares Apache-2.0 in its `LICENSE` and includes a `NOTICE`
with copyright and trademark attribution. Sources:
https://github.com/getsigit/sigit/blob/v1.7.10/LICENSE and
https://github.com/getsigit/sigit/blob/v1.7.10/NOTICE.
This is a modified, locally built managed artifact, not an unmodified official
release. The upstream protocol version remains 1.7.10; the separate source,
patch and binary digests in the build receipt identify the reviewed derivative.
Retain the upstream LICENSE and NOTICE when redistributing derived artifacts.

## Explicit stdio MCP prototype

`session-mcp.patch` layers on top of `host-boundary.patch` for siGit 1.7.10.
Apply it with `patch -p1` in the already patched upstream source tree and
build the `sigit` binary through the repository Cargo slot runner. The
managed installer still selects the existing reviewed patch.

The prototype confines each explicitly selected stdio service to a macOS
read-only sandbox, with the requested workspace as its working directory.
The service starts with a clean environment. Each invocation uses siGit's
existing ACP host approval, and MCP errors are reported as failed tool
events. A configured service that exposes no tools fails session setup.

RustCodeGraph 1.2.10 requires usable SQLite WAL/SHM sidecars to open a
fresh index under the read-only policy. The private probe uses a host-owned
SQLite read transaction to prepare those sidecars. The sandbox tests cover
real RustCodeGraph queries, isolated environment and denied writes, external
reads, network connections and process creation. Native ACP integration
requires a separate acceptance run before the managed installer can select
this patch.
