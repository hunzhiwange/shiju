"""Apply the ACP boundary to the exact AutoHand 0.9.11 npm bundle."""

import hashlib
import json
from pathlib import Path
import sys


def apply(package, boundary, linux_boundary=None):
    # npm's ESM entry imports this specific ACP chunk. Leave other modes untouched.
    entry = package / "dist/index.js"
    adapter = package / "dist/acp-LDTU5B5V.js"
    agent = package / "dist/chunk-R73FESUM.js"
    metadata = json.loads((package / "package.json").read_text())
    if (metadata["name"], metadata["version"]) != ("autohand-cli", "0.9.11"):
        raise ValueError("unexpected AutoHand package")
    for file, digest in (
        (entry, "339bdb0c8f4f8b7f9076108434d910b2cd5e14f2593b48c0e5858e04e91950eb"),
        (agent, "421bb57004348974750964ace4305d21f19735db19375c05bf363c07bed30d5a"),
        (adapter, "3e5e617a5ca04bbc3f383a2299f0966badfb6b07cce5c28b691fa7044f4a72dc"),
    ):
        if hashlib.sha256(file.read_bytes()).hexdigest() != digest:
            raise ValueError(f"unexpected AutoHand bundle file {file.name}")

    source = adapter.read_text()
    replacements = (
        (
            "  const workspacePath = options.path ?? process.cwd();",
            '  const hostWorkspace = process.env.SHIJU_AUTOHAND_MANAGED === "1"\n'
            "    ? process.env.SHIJU_AUTOHAND_WORKSPACE : undefined;\n"
            "  const workspacePath = hostWorkspace || options.path || process.cwd();",
        ),
        (
            "  redirectConsoleToStderr();\n  process.env.AUTOHAND_CLIENT_NAME",
            "  if (hostWorkspace) process.chdir(hostWorkspace);\n"
            "  redirectConsoleToStderr();\n  process.env.AUTOHAND_CLIENT_NAME",
        ),
        (
            "  resolveWorkspaceRoot(cwd) {\n    let workspaceRoot = cwd;",
            "  resolveWorkspaceRoot(cwd) {\n"
            '    if (process.env.SHIJU_AUTOHAND_MANAGED === "1" && process.env.SHIJU_AUTOHAND_WORKSPACE\n'
            "        && cwd !== process.env.SHIJU_AUTOHAND_WORKSPACE) {\n"
            '      throw RequestError.invalidParams({ message: "AutoHand managed workspace mismatch" });\n'
            "    }\n"
            "    let workspaceRoot = cwd;",
        ),

        (
            "    const converted = this.convertAcpMcpServers(mcpServers);",
            "    agent.shijuMcpSelect?.(mcpServers);\n"
            "    const converted = this.convertAcpMcpServers(mcpServers);",
        ),
        (
            "    await agent.initializeForRPC(void 0, existingSessionId);\n",
            '    if (process.env.SHIJU_AUTOHAND_MANAGED === "1") {\n'
            "      agent.shijuPrepareSkillView = () => attachSkillView(\n"
            "        process.env.SHIJU_AUTOHAND_SKILL_ROOT, agent, files,\n"
            "        (root) => new FileActionManager(root));\n"
            "    }\n"
            "    await agent.initializeForRPC(void 0, existingSessionId);\n",
        ),
        (
            "    this.permissionBridges.set(managedSessionId, permBridge);",
            '    if (process.env.SHIJU_AUTOHAND_MANAGED === "1") {\n'
            "      agent.shijuMcpSelect = requireMcpApproval(agent, this.connection, managedSessionId);\n"
            "    }\n"
            "    this.permissionBridges.set(managedSessionId, permBridge);",
        ),
    )
    for before, after in replacements:
        if source.count(before) != 1:
            raise ValueError("unexpected AutoHand patch anchor")
        source = source.replace(before, after)
    agent_source = agent.read_text()
    before = (
        "  const [, session] = await awaitLifecycleStep(Promise.all([\n"
        "    existingSessionId ? Promise.resolve() : host.resetConversationContext(),"
    )
    if agent_source.count(before) != 1:
        raise ValueError("unexpected AutoHand native skill initialization anchor")
    agent_source = agent_source.replace(
        before, "  await host.shijuPrepareSkillView?.();\n" + before
    )
    if linux_boundary is not None:
        source, agent_source = apply_linux_mcp_boundary(source, agent_source)
        (adapter.parent / "shiju-linux-mcp-boundary.mjs").write_bytes(linux_boundary.read_bytes())
    agent.write_text(agent_source)
    (adapter.parent / "shiju-session-boundary.mjs").write_bytes(boundary.read_bytes())
    adapter.write_text(
        'import { attachSkillView, requireMcpApproval } from "./shiju-session-boundary.mjs";\n' + source
    )


def apply_linux_mcp_boundary(adapter, agent):
    def replace_once(source, before, after):
        if source.count(before) != 1:
            raise ValueError("unexpected AutoHand Linux MCP patch anchor")
        return source.replace(before, after)

    adapter = replace_once(
        adapter,
        '    if (process.env.SHIJU_AUTOHAND_MANAGED === "1") {\n'
        "      agent.shijuPrepareSkillView",
        '    if (process.env.SHIJU_AUTOHAND_MANAGED === "1") {\n'
        "      agent.shijuGuardMcp = guardMcpConnections;\n"
        "      agent.shijuPrepareSkillView",
    )
    adapter = replace_once(
        adapter,
        "      agent.shijuMcpSelect = requireMcpApproval(agent, this.connection, managedSessionId);",
        "      const approveMcp = requireMcpApproval(agent, this.connection, managedSessionId);\n"
        "      agent.shijuMcpSelect = (servers) => agent.shijuSelectMcpConnections(servers, approveMcp);",
    )
    agent = replace_once(
        agent,
        "  host.mcpManager = new McpClientManager();",
        "  host.mcpManager = new McpClientManager();\n"
        "  host.shijuSelectMcpConnections = host.shijuGuardMcp?.(host.mcpManager);",
    )
    return ('import { guardMcpConnections } from "./shiju-linux-mcp-boundary.mjs";\n' + adapter, agent)


if __name__ == "__main__":
    if len(sys.argv) not in (3, 4):
        sys.exit("usage: apply.py PACKAGE BOUNDARY [LINUX_BOUNDARY]")
    apply(*map(Path, sys.argv[1:]))
