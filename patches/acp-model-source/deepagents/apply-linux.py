"""Add the Linux projection client to a staged, reviewed DeepAgents v5 package."""
import hashlib
from pathlib import Path
import shutil
import sys

V5 = {
    "cli.js": "55aace3a7b5eb077e8cb6314465f6fb965d8ee844d134e7f361bbca6f5396d4c",
    "shiju-mcp.mjs": "c3ca54774614df9574ad064bda29c02e6185b0823283022f44dd8d02d4cd11cf",
}


def replace(source, old, new):
    if source.count(old) != 1:
        raise RuntimeError("DeepAgents Linux patch context mismatch")
    return source.replace(old, new)


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: apply-linux.py <staged-deepagents-package>")
    dist = Path(sys.argv[1]).resolve(strict=True) / "dist"
    for name, digest in V5.items():
        path = dist / name
        if path.is_symlink() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise RuntimeError("DeepAgents Linux patch requires the reviewed v5 files")
    entry = dist / "cli.js"
    mcp = dist / "shiju-mcp.mjs"
    cli_source = replace(
        entry.read_text(),
        "title: formatToolCallTitle(toolCall.name, toolCall.args),\n\t\t\t\t\tkind:",
        'title: process.platform === "linux" && session.mcpTools?.has(toolCall.name)\n'
        "\t\t\t\t\t\t? toolCall.name : formatToolCallTitle(toolCall.name, toolCall.args),\n"
        "\t\t\t\t\tkind:",
    )
    cli_source = replace(
        cli_source,
        'if (optionId === "allow-always") {',
        'if (optionId === "allow-always") {\n'
        '\t\t\t\tif (process.platform === "linux" && session.mcpTools?.has(toolCall.name)) return "reject";',
    )
    mcp_source = 'import { linuxSelection, runProjectedMcp } from "./shiju-linux-mcp.mjs";\n' + mcp.read_text()
    mcp_source = replace(mcp_source, "if (!servers.length) return [];",
                         'if (!servers.length) return [];\n  if (process.platform === "linux") return linuxSelection(servers);')
    mcp_source = replace(mcp_source, "  signal?.throwIfAborted();",
                         "  signal?.throwIfAborted();\n  if (request.server.url) return runProjectedMcp(request, signal);")
    mcp_source = replace(mcp_source, "const server = { name, command, args };",
                         "const server = policy.url ? policy : { name, command, args };")
    mcp_source = replace(mcp_source, 'const nameForModel = server.name + "__" + name;',
                         'const nameForModel = (server.url ? "mcp__" : "") + server.name + "__" + name;')
    mcp_source = replace(mcp_source, "if (!allowedTool(policy, name, nameForModel)) continue;",
                         "if (!server.url && !allowedTool(policy, name, nameForModel)) continue;")
    mcp_source = replace(mcp_source, "name, arguments: args }, config.signal);",
                         "name, arguments: args, callId: config.toolCall.id }, config.signal);")
    entry.write_text(cli_source)
    mcp.write_text(mcp_source)
    shutil.copyfile(Path(__file__).with_name("linux-mcp.mjs"), dist / "shiju-linux-mcp.mjs")


if __name__ == "__main__":
    main()
