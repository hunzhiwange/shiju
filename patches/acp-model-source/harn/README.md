# Harn session-model runtime

The system installer pins Harn 0.10.201 for macOS arm64, checks the official archive
and each runnable artifact, and preserves the supplier's license files. Other
platforms fail explicitly; no moving shell installer or same-name npm package is used.

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
