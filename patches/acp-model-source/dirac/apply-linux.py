"""Add the fixed Linux search helper and restore guards to a staged Dirac v2."""

import hashlib
import io
import json
from pathlib import Path
import platform
import sys
import tarfile
import urllib.request


def replace(source, before, after):
    if source.count(before) != 1:
        raise RuntimeError("Dirac Linux patch anchor mismatch")
    return source.replace(before, after, 1)


def patch_sources(source, bridge):
    source = replace(
        source,
        'import * as shijuMcp from "./session-mcp.mjs";',
        'import * as shijuMcp from "./session-mcp.mjs";'
        'import * as shijuSession from "./linux-session.mjs";',
    )
    source = replace(
        source,
        'validate(e.mcpServers);let r=crypto.randomUUID(),i=this.createStartupSessionOverrides();',
        'validate(e.mcpServers);let r=crypto.randomUUID();'
        'if(selected||mounts.length)shijuSession.recordMount(r);'
        'let i=this.createStartupSessionOverrides();',
    )
    for method in ("loadSession", "unstable_forkSession"):
        anchor = f"async {method}(e){{shijuMcp.rejectSkillRestore"
        source = replace(
            source, anchor,
            f"async {method}(e){{shijuSession.rejectMountedRestore(e.sessionId);shijuMcp.rejectSkillRestore",
        )
    source = replace(source, 'import{execFileSync as GSp}from"node:child_process";', "")
    source = replace(
        source,
        'import{rgPath as hei}from"@vscode/ripgrep";',
        'import{fileURLToPath as shijuFileURLToPath}from"node:url";',
    )
    start = 'async function Aei(t){'
    end = 'var HSp,A2s,mei,oI,eK='
    if source.count(start) != 1 or source.count(end) != 1:
        raise RuntimeError("Dirac Linux ripgrep anchor mismatch")
    first = source.index(start)
    last = source.index(end, first)
    source = source[:first] + (
        'async function Aei(t){if(t!=="rg")throw new Error("Unsupported Dirac search helper");'
        'const path=shijuFileURLToPath(new URL("./shiju-rg",import.meta.url));'
        'qSp(path,m2s.X_OK);return path}'
    ) + source[last:]
    source = replace(
        source,
        'cwd:e.cwd,mcpServers:e.mcpServers??[]})}async captureForkSnapshot',
        'cwd:e.cwd,mcpServers:e.mcpServers??[],_meta:e._meta})}async captureForkSnapshot',
    )
    source = replace(
        source,
        'if(e.mcpServers?.length)throw new Error("Dirac MCP prototype supports only new sessions");let r=e.sessionId,i=crypto.randomUUID()',
        'if(e.mcpServers?.length||shijuMcp.hasMounts(this.sessions.get(e.sessionId)))'
        'throw new Error("Dirac MCP prototype supports only new sessions");let r=e.sessionId,i=crypto.randomUUID()',
    )
    bridge = replace(
        bridge,
        "          await update('in_progress');\n          const result = await client.callTool(",
        "          await update('in_progress');\n          controller.signal.throwIfAborted();\n"
        "          const result = await client.callTool(",
    )
    bridge = replace(
        bridge,
        "    running.delete(session);\n    await Promise.allSettled(clients.map(client => client.close()));",
        "    await Promise.allSettled(clients.map(client => client.close()));\n    running.delete(session);",
    )
    bridge = replace(
        bridge,
        "const decision = await requestPermission({ ...fields, rawInput }, [",
        "const argumentsSnapshot = structuredClone(rawInput);\n"
        "          const decision = await requestPermission({ ...fields, kind: 'other', rawInput: argumentsSnapshot }, [",
    )
    bridge = replace(
        bridge,
        "client.callTool({ name: tool.name, arguments: rawInput }, undefined, {",
        "client.callTool({ name: tool.name, arguments: argumentsSnapshot,\n"
        "            _meta: { shijuCallId: fields.toolCallId } }, undefined, {",
    )
    return source.encode(), bridge.encode()


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: apply-linux.py <staged-dirac-cli-package>")
    manifest = json.loads(Path(__file__).with_name("linux-artifacts.json").read_text())
    target = manifest["targets"].get(platform.machine())
    if sys.platform != "linux" or target is None:
        raise RuntimeError("Unsupported Dirac Linux target")

    package = Path(sys.argv[1]).resolve(strict=True)
    entry = package / manifest["bin"]
    helper = package / "dist/session-mcp.mjs"
    rg = package / manifest["rg"]
    session = Path(__file__).with_name("linux-session.mjs").read_bytes()
    if any(not path.is_file() or path.resolve() != path for path in (entry, helper)):
        raise RuntimeError("Dirac Linux patch requires regular v2 files in the staged package")
    original_entry = entry.read_bytes()
    original_helper = helper.read_bytes()
    if (hashlib.sha256(original_entry).hexdigest()
            != "abbc5f72264b5bf4da8692a93bdcc27e4490d95a9a230632885d77da8fed2096"
            or hashlib.sha256(original_helper).hexdigest()
            != "4e2362bfa6661564693b06ddd0eb85bc4d5edc1e6720ea758589c1234e6455ad"):
        raise RuntimeError("Dirac Linux patch requires the reviewed v2 files")

    source, bridge = patch_sources(original_entry.decode(), original_helper.decode())
    files = {
        manifest["bin"]: source,
        "dist/session-mcp.mjs": bridge,
        "dist/linux-session.mjs": session,
    }
    for name, data in files.items():
        if hashlib.sha256(data).hexdigest() != manifest["files"][name]:
            raise RuntimeError("Dirac Linux patched file checksum mismatch")
    with urllib.request.urlopen(target["url"], timeout=30) as response:
        archive = response.read(16 * 1024 * 1024)
    if hashlib.sha256(archive).hexdigest() != target["archive_sha256"]:
        raise RuntimeError("Dirac ripgrep archive checksum mismatch")
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as bundle:
        member = bundle.getmember("rg")
        if not member.isfile():
            raise RuntimeError("Dirac ripgrep archive has no regular executable")
        binary = bundle.extractfile(member).read()
    if hashlib.sha256(binary).hexdigest() != target["sha256"]:
        raise RuntimeError("Dirac ripgrep binary checksum mismatch")
    with rg.open("xb") as stream:
        stream.write(binary)
    rg.chmod(0o755)
    with (package / "dist/linux-session.mjs").open("xb") as stream:
        stream.write(session)
    entry.write_bytes(files[manifest["bin"]])
    helper.write_bytes(files["dist/session-mcp.mjs"])


if __name__ == "__main__":
    main()
