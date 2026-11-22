# Kilocode 7.8.18 session skill scanner (Darwin arm64)

`apply.py` verifies the pinned npm platform executable and UTF-16 Bun entry
source, applies the four exact edits in `patch.json`, and adjusts the module
graph inside the existing Mach-O section padding. The private copy is ad-hoc
signed with a fixed identifier and no timestamp. The script checks its final
SHA-256 and compares every other embedded module byte for byte. The managed
installer uses revision `kilocode-7.8.18-model-source-v2`.

The scanner keeps the upstream native search order and configuration gates.
A nonempty `SHIJU_KILO_SKILL_ROOT` appends the validated session view using
the upstream discovery and Markdown parser. The empty value adds no paths.
Each selected skill retains its canonical directory as its file substitution
and source-read boundary and stays untrusted. Native skills take precedence
when a selected skill declares the same name. The managed launch continues
to omit built-ins while the variable is set. Without the variable, upstream
discovery and built-in registration are unchanged. ACP slash commands and the
model's native `skill` tool use the same catalog.

Restricted Darwin `session_model` launches use the request-owned Seatbelt
profile. The native `skill` tool samples files with Bun's glob iterator in
this profile, so it needs no ripgrep executable or download. The launcher
permits the reviewed entry and system Git, enables only its authenticated
native loopback endpoint and the existing OMP bridge, and exposes selected
MCP services through that bridge. The bridge checks the host catalog, binds
calls to the active prompt, and waits for one-time authorization before
running the isolated stdio call.

The patch retains the native provider and permission implementations. The
installer works on its private runtime, and the host still checks the selected
view and reviewed executable before passing the root.

The release sources for the scanner and ACP catalog are
[`skill/index.ts`](https://raw.githubusercontent.com/Kilo-Org/kilocode/v7.8.18/packages/opencode/src/skill/index.ts)
and [`acp/service.ts`](https://raw.githubusercontent.com/Kilo-Org/kilocode/v7.8.18/packages/opencode/src/acp/service.ts).

## Linux GNU release artifacts

`linux.json` pins the npm archive integrity, original ELF, embedded entry
source, and patched ELF for x86-64 baseline and AArch64. `apply_linux.py`
takes the manifest target (`x64` or `arm64`), the extracted `bin/kilo`,
and a separate reviewed destination inside the private installation. It
uses the release's existing ELF segment padding for the same four scanner
edits, checks the complete output digest, and publishes the executable
atomically. Both targets use the native `acp --pure` entry.

In revision 3, restricted ACP registration accepts only the anonymous
Streamable HTTP paths issued by the host bridge. The Linux launcher must
set `SHIJU_KILO_BRIDGE_PORT` from the bridge's bound port in the child
environment. The client rejects every supplied mount if the port is unset
or any entry falls outside that contract.

The patched ELF is an installation input. Restricted Linux use still
requires a launcher that enforces the workspace, execution, and loopback
boundaries and isolates the downstream MCP peers. The checked-in Seatbelt
launcher provides those boundaries on Darwin arm64.
