# Kilocode 7.8.18 session skill scanner (Darwin arm64)

The reviewed npm platform binary contains a Bun module graph with UTF-16 entry
source, not opaque agent bytecode. `apply.py` validates the complete upstream
executable and entry-source SHA-256, applies the two exact source edits in
`patch.json`, adjusts graph pointers and the Mach-O section length within existing
zero padding, then ad-hoc signs the private copy with a fixed identifier and no
timestamp. The resulting executable SHA-256 is pinned too. Other embedded modules
are checked byte-for-byte; native code, tools, providers and assets are retained.
No external Bun installation or source rebuild is needed. Other platforms fail
explicitly. Two independent repacks produced the same signed executable hash.

The source edits implement this behavior:

- Without `SHIJU_KILO_SKILL_ROOT`, preserve the upstream discovery and built-ins.
- With an empty value, discover no skill directories and register no built-ins.
- With an absolute existing root, call the **upstream scanner and markdown
  parser** only for that root, marking it untrusted. Resolve each selected skill directory through the
  native filesystem service and use that directory as the source-read and file
  substitution boundary, so audited session-view links work without granting
  trust or allowing reads outside that selected skill. Do not discover global, project, URL or plugin skills.
- In this mode, do not seed built-in skills. The upstream ACP directory snapshot
  derives its skill slash commands from this same skill service, so it cannot
  reintroduce excluded built-ins or removed skills.

This changes neither provider routing nor native permission evaluation, and does
not grant skill shell-injection trust. The Rust adapter validates the session
view and full patched installation before setting the root. Installation copies
and patches only the private runtime; it never changes the user's CLI or HOME.

The fixed upstream source is available at
`https://github.com/Kilo-Org/kilocode/tree/v7.8.18`, notably
`packages/opencode/src/skill/index.ts` and `packages/opencode/src/acp/service.ts`.
The downloaded tag archive SHA-256 is
`05a640377d8de1dba008992ea3f3b54d558de3f7eda91ceb1fe8879eb27c96e7`.
The repack uses the exact npm binary's embedded source, rather than assuming a
fresh build from a tag would match the release executable.
