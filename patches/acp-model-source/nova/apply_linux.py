"""Overlay the Linux GNU Nova runtime after applying the reviewed v3 patch."""
import hashlib
import shutil
from pathlib import Path
import sys


def rewrite(path, expected, replacements):
    original = path.read_bytes()
    if hashlib.sha256(original).hexdigest() != expected:
        raise SystemExit(f"Unreviewed Nova Linux patch input: {path.name}")
    result = original
    for old, new in replacements:
        if result.count(old) != 1:
            raise SystemExit(f"Nova Linux patch context mismatch: {path.name}")
        result = result.replace(old, new)
    return result


root = Path(sys.argv[1])
wrapper = root / "scripts/bin-wrapper.mjs"
cli = root / "dist/cli.js"
extension = root / "dist/shiju-session-mcp.mjs"

# A platform executable resolved by Node may live outside the reviewed prefix.
# Keep the wrapper's environment and stdio handling and pin it to its JS bundle.
patched_wrapper = rewrite(wrapper,
    "82f4ea07714bc0227d9c7206420acc1fdb41cd2f4a2886d1146db47af456119b",
    [(b"const binaryPath = findBinary();", b"const binaryPath = process.env.SHIJU_NOVA_MODEL_CONFIG ? null : findBinary();")])

# Skills that use shell expansion or fork escape the native tool approval loop.
# On Linux, keep managed skill activation inline, including native skills.
patched_cli = rewrite(cli,
    "ee96b92927b6c1e524b6c20a25f00428b5579692ba298b254725aa66f5dabf5f", [
        (b"if(shijuNovaIsMountedSkill(_0x5d5232) &&",
         b"if((shijuNovaBound || shijuNovaIsMountedSkill(_0x5d5232)) &&"),
        (b"Mounted Nova skills require inline context",
         b"Managed Nova skills require inline context"),
        (b"_0x2265ff=shijuNovaIsMountedSkill(_0x5d5232)?",
         b"_0x2265ff=(shijuNovaBound || shijuNovaIsMountedSkill(_0x5d5232))?"),
    ])

# Linux can only use request-owned projections. Keep the original inputs for
# v3 verification, then remove the local process path from the Linux extension.
patched_extension = rewrite(extension,
    "4dc6b7a014f47022e6915e0790cc0ae859863d3c1039d64727d4abe0992a4202", [
        (b"import { execFile } from 'node:child_process';\n", b""),
        (b"import { fileURLToPath } from 'node:url';",
         b"import { selectProjectedServers, projectedOperation } from './shiju-linux-mcp.mjs';"),
        (b"const helper = fileURLToPath(new URL('./shiju-mcp.py', import.meta.url));\n", b""),
        (b"process.platform !== 'darwin' || process.arch !== 'arm64'",
         b"process.platform !== 'linux' || !['x64', 'arm64'].includes(process.arch)"),
        (b"process.platform !== 'darwin' || servers.length > 16",
         b"process.platform !== 'linux' || !['x64', 'arm64'].includes(process.arch)"),
        (b"    const selected = Array.isArray(servers) && servers.length > 0;",
         b"    if (servers !== undefined && !Array.isArray(servers)) throw invalid('Invalid Nova MCP selection');\n"
         b"    const selected = Boolean(servers?.length);"),
        (b"    state = { managed: true };",
         b"    const projected = selectProjectedServers(servers, bound.endpoint,\n"
         b"      process.env.SHIJU_NOVA_RELAY_TOKEN, process.env.VW_ACP_LAUNCH_REFERENCE);\n"
         b"    state = { managed: true };"),
        (b"      state.workspace = await realpath(params.cwd);\n", b""),
        (b"async function discover(servers, workspace, allowed, signal)",
         b"async function discover(servers, allowed, signal)"),
        (b"state.tools = await discover(servers, state.workspace,",
         b"state.tools = await discover(projected,"),
        (b"if ((allowed && !allowed.has(policyId)) || deny.includes(descriptor.name) || deny.includes(policyId) ||\n"
         b"          (allow.length && !allow.includes(descriptor.name) && !allow.includes(policyId))) continue;",
         b"if (allowed && !allowed.has(policyId)) continue;"),
        (b"await operation({ operation: 'call', workspace: state.workspace,\n"
         b"    server: target.server, name: target.name, arguments: arguments_",
         b"await projectedOperation({ operation: 'call',\n"
         b"    server: target.server, name: target.name, arguments: arguments_, callId"),
    ])
operation = patched_extension.index(b"function operation(request, signal)")
discovery = patched_extension.index(b"async function discover", operation)
patched_extension = patched_extension[:operation] + patched_extension[discovery:]
start = patched_extension.index(b"  const names = new Set();", operation)
end = patched_extension.index(b"    for (const descriptor", start)
patched_extension = (
    patched_extension[:start]
    + b"  const tools = new Map();\n"
    + b"  for (const server of servers) {\n"
    + b"    const { name } = server;\n"
    + b"    const catalog = await projectedOperation({ operation: 'list', server }, signal);\n"
    + patched_extension[end:]
)
for path, source in [(wrapper, patched_wrapper), (cli, patched_cli), (extension, patched_extension)]:
    path.write_bytes(source)
shutil.copyfile(Path(__file__).with_name("linux-mcp.mjs"), root / "dist/shiju-linux-mcp.mjs")
