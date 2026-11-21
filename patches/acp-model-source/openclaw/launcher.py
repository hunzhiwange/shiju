"""Own the confined OpenClaw Gateway and ACP bridge beside host-approved stdio MCP."""
import asyncio
import json
import os
from pathlib import Path
import re
import runpy
import secrets
import socket
import sys
import tempfile
from urllib.parse import urlsplit


def main():
    entry_name, profile_name, node_name, *args = sys.argv[1:]
    if args != ["acp"]:
        raise ValueError("Invalid restricted OpenClaw arguments")
    entry, profile, node = (Path(value).resolve(strict=True)
                            for value in (entry_name, profile_name, node_name))
    tempfile.tempdir = "/private/tmp"
    workspace = Path.cwd().resolve(strict=True)
    settings = json.loads((profile / "openclaw-launch.json").read_bytes())
    if settings["workspace"] != str(workspace):
        raise ValueError("OpenClaw workspace binding changed")
    config = settings["config"]
    config["agents"]["defaults"]["workspace"] = str(workspace)
    provider = config["models"]["providers"]["session-provider"]
    endpoint = urlsplit(provider["baseUrl"])
    if (endpoint.scheme != "http" or endpoint.netloc != f"127.0.0.1:{endpoint.port}"
            or not endpoint.port or endpoint.path != "/v1" or endpoint.query or endpoint.fragment):
        raise ValueError("OpenClaw requires the owned model relay")
    token = os.environ["VW_OPENCLAW_RELAY_TOKEN"]
    home = profile / "native"
    config_file = home / "openclaw.json"
    gateway_token = secrets.token_hex(32)
    token_file = home / "gateway-token"
    token_file.write_text(gateway_token)
    token_file.chmod(0o600)
    skill_root = settings["skill_root"]
    sources = settings["skill_sources"]
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        gateway_port = reservation.getsockname()[1]

    env = {
        "HOME": str(home), "TMPDIR": str(home), "PATH": "/usr/bin:/bin",
        "OPENCLAW_STATE_DIR": str(home), "OPENCLAW_CONFIG_PATH": str(config_file),
        "OPENCLAW_SKIP_CHANNELS": "1", "OPENCLAW_SKIP_CANVAS_HOST": "1",
        "SHIJU_OPENCLAW_SKILL_ROOT": skill_root,
        "SHIJU_OPENCLAW_NATIVE_SKILL_WORKSPACE": str(workspace),
        "VW_OPENCLAW_GATEWAY_TOKEN": gateway_token, "VW_OPENCLAW_RELAY_TOKEN": token,
    }
    quote = lambda value: json.dumps(str(value))
    paths = [entry.parents[2], workspace, home] + ([skill_root, *sources] if skill_root else [])
    reads = " ".join(f"(subpath {quote(path)})" for path in paths)
    selected = " ".join(f"(subpath {quote(path)})" for path in sources)
    selection = f"(allow file-read-data {selected})" if selected else ""
    denied = " ".join(f"(subpath {quote(workspace / name)})" for name in (
        ".git", ".env", ".env.local", ".env.development", ".env.production",
        ".env.test", ".vwacprc.json", ".mcp-probe",
    ))

    def configure(port):
        nonlocal sandbox
        sandbox = f"""(version 1)
(allow default)
(deny file-read-data)
(allow file-read-data (literal "/") (subpath "/System") (subpath "/usr/lib") (subpath "/usr/share"))
(deny file-read-data (subpath "/System/Volumes/Data"))
(allow file-read-data (literal "/dev/null") (literal "/dev/urandom") (literal "/dev/random")
  (literal "/private/etc/hosts") (literal "/private/etc/localtime") (literal {quote(node)}) {reads})
(deny file-read-data (subpath {quote(workspace / ".shiju")}))
{selection}
(deny file-read-data {denied})
(deny file-write*)
(allow file-write* (subpath {quote(home)}) (literal "/dev/null"))
(deny process-exec)
(allow process-exec (literal {quote(node)}))
(deny network*)
(allow network-bind network-inbound (local ip "localhost:{gateway_port}"))
(allow network-outbound (remote ip "localhost:{gateway_port}") (remote ip "localhost:{port}"))
"""
        provider["baseUrl"] = f"http://127.0.0.1:{port}/v1"

    def executable():
        return ["/usr/bin/sandbox-exec", "-p", sandbox, str(node), str(entry)]

    async def gateway(mounts, session):
        # Only advertise the approved schema produced by the host's stdio relay.
        tool_names = []
        servers = {}
        for mount in session.mounts.values():
            for tool in mount.tools:
                name = f"{mount.name}__{tool}"
                if (len(mount.name) > 30 or len(name) > 64 or name.lower() != name
                        or not re.fullmatch(r"[a-z][a-z0-9_]*__[a-z][a-z0-9_]*", name)
                        or name in tool_names):
                    raise ValueError("Unsupported OpenClaw MCP tool identity")
                tool_names.append(name)
        for mount in mounts:
            servers[mount["name"]] = {"url": mount["url"], "transport": "streamable-http"}
        if servers:
            config["mcp"] = {"servers": servers}
        config["tools"] = {"allow": ["read", *tool_names], "toolSearch": False,
                           "fs": {"workspaceOnly": True}}
        config["gateway"]["port"] = gateway_port
        config["session"]["store"] = str(home / "{agentId}" / "sessions.json")
        config_file.write_text(json.dumps(config))
        child = await asyncio.create_subprocess_exec(
            *executable(), "gateway", "run", "--port", str(gateway_port), "--bind", "loopback",
            cwd=workspace, env=env, stdin=asyncio.subprocess.DEVNULL,
            stdout=asyncio.subprocess.DEVNULL, stderr=asyncio.subprocess.DEVNULL)
        try:
            for _ in range(300):
                if child.returncode is not None:
                    break
                try:
                    _, writer = await asyncio.open_connection("127.0.0.1", gateway_port)
                except OSError:
                    await asyncio.sleep(.1)
                else:
                    writer.close()
                    await writer.wait_closed()
                    return child
            raise ValueError("Private OpenClaw Gateway did not start")
        except BaseException:
            if child.returncode is None:
                child.kill()
            await child.wait()
            raise

    def command(_port):
        return executable() + ["acp", "--url", f"ws://127.0.0.1:{gateway_port}",
                               "--token-file", str(token_file), "--no-prefix-cwd",
                               "--session", settings["session"]]

    # Both Node processes stay in the ACP launch process group. The stdio
    # executor retains the existing relay's per-call process ownership.
    sandbox = None
    bridge = runpy.run_path(str(profile / "omp-mcp-bridge.py"))
    stdio = runpy.run_path(str(profile / "omp-mcp-stdio.py"))
    return asyncio.run(bridge["run"](command, env, workspace, endpoint, token,
                                     configure, stdio["Stdio"], stdio["allowed_tool"],
                                     owned_gateway=gateway, owned_mounts=settings["mounts"]))


if __name__ == "__main__":
    sys.exit(main())
