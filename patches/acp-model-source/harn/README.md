# Harn session-model runtime

The system installer pins Harn 0.10.201 for macOS arm64 and preserves the
supplier's license files. v4 used the official binary archive. v5 verifies
the fixed source archive and patch before building the native CLI. Other
platforms fail explicitly.

`agent.harn` consumes the ACP `prompt` ambient value through Harn's own
`std/agent/loop`. It exposes read/write tools through `harness.tools.invoke`, never
through direct native filesystem calls. The small `launcher.py` transport adapter
maps these two `builtin_call` requests to standard ACP `fs/read_text_file` and
`fs/write_text_file` requests. ShiJu remains responsible for session identity,
permissions, workspace boundaries, and the actual file effect. Unsupported host
tools fail closed. The adapter does not perform model inference or file operations.

The driver writes a private provider overlay pointing only to the request-owned
loopback relay, selects the original provider/model through ACP, and rejects
custom arguments, custom environment, and project provider overrides. Only the
relay credential is granted to Harn's in-process model calls. Child commands and
MCP processes do not inherit it. The launcher removes unrelated gateway environment
values. Native CLI-local model configuration is deliberately unavailable for this
managed runtime.

The host must declare a tool-capable model. The pipeline's native tool contract
uses Harn's documented override reason because the request-private relay provider
is not a shipped Harn catalog entry; the relay validates the selected model's
protocol and tool capabilities before launch. This does not grant filesystem or
process permissions.

Run the native boundary checks explicitly with a checksum-verified binary:

```sh
python3 patches/acp-model-source/harn/installer_test.py
HARN_TEST_BINARY=/absolute/path/to/harn python3 patches/acp-model-source/harn/pipeline_test.py
HARN_TEST_BINARY=/absolute/path/to/harn python3 patches/acp-model-source/harn/host_bridge_test.py
```

These use isolated state and a local provider fixture. They verify natural-language
parsing, the original model on the wire, successful authorized writing, and host
write denial. Real supplier/model acceptance is recorded separately in ACP验收.md.

The v4 managed launcher accepts explicit ACP stdio mounts for the current
workspace bound by the host at launch. It discovers tools in the same read-only, environment-filtered
sandbox used by the reviewed Cortex adapter. The host-resolved allow and deny
lists control tool registration. Harn's pipeline uses the existing host tool
callback, and the launcher asks the ACP host for one-time permission with the
original call ID and arguments before dispatch. The Harn engine stays in its
ordinary permission mode. A new, loaded or resumed session takes its MCP
selection from that request, and prompt cancellation revokes outstanding
approvals and closes the tool subprocesses.

For the focused native check, pass the installed v4 launcher, a checksum-verified
RustCodeGraph and a private indexed RueJS copy:

```sh
python3 patches/acp-model-source/harn/mcp_test.py /absolute/path/to/harn /absolute/path/to/rustcodegraph /absolute/path/to/ruejs
```

## Source-built skill registry

Managed revision v5 uses the pinned 0.10.201 source commit and
`skills.patch`. Install with Rust 1.99.1 and the upstream lockfile. The
installer verifies the source and patch before invoking Cargo and records
the engine digest alongside those inputs. It installs beside v4. Use the
v5 engine for the pipeline smoke checks above.

The managed request owns a private directory with a snapshot of the selected
bundles and a separate snapshot of project `.harn/skills` manifests and detached
signatures. Harn gives the native catalog precedence on duplicate names.
The ACP runtime installs the same metadata-only loader and policy fetcher
used by its CLI, with only those two roots. It resets the binding when the
prompt leaves the VM. Skills that disable model invocation continue to do so.
The ACP bridge translates attachment reads back to the workspace or the scoped
skill lease. File permission, tool policy and one-time MCP approval stay with
the existing host routes.

Run the installer and launcher boundary checks with:

```sh
python3 patches/acp-model-source/harn/installer_test.py
python3 patches/acp-model-source/harn/skills_test.py
```
