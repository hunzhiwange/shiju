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
