# siGit 1.7.10 host boundary

The fixed upstream source is built separately from the user's native installation.
`host-boundary.patch` removes failed filesystem/terminal callback fallbacks, asks
for host authorization on every tool invocation, and removes pre-approval file
previews, ambient tool/session hooks and automatic project instruction loading.
The built-in read/create/edit/multi-edit and foreground command tools use
host-backed execution. Explicitly selected stdio MCP tools can coexist with
them under the macOS read-only sandbox. Native fallback tools, automatic
subagents and ambient MCP discovery stay disabled. Use the pinned
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

For an indexed workspace, siGit automatically prepares missing SQLite
sidecars before launching an explicit RustCodeGraph stdio service. The
macOS system SQLite helper can write only the sidecars and exits before the
read-only service starts. Existing workspace MCP configuration is enough.

## Native skill discovery in managed session-model launches

On Darwin arm64, the managed session scanner reads bounded frontmatter from
the project's native `.sigit/skills` and `.claude/skills` directories. It
also checks the host's private session view and appends its validated sources.
Project roots keep siGit's existing name precedence. Files opened for
discovery must be regular files reached without symlinks. The session view's
host-created links are checked against their registered targets.

The native `skill` tool activates a name advertised for the current turn.
Activation and attachment reads use ACP filesystem callbacks and host
permission policy. Selected sources outside the project are eligible for
read callbacks with an active skill lease. User-global discovery stays
disabled in managed ACP. The stdio MCP sandbox and approval path are unchanged.
