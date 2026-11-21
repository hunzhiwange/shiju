"""Start restricted OpenCode and the host-approved stdio MCP bridge."""
import asyncio
import json
import os
from pathlib import Path
import runpy
import secrets
import socket
import sys
from urllib.parse import urlsplit


def quote(path):
    return json.dumps(str(path))


def main():
    executable, profile_name, *args = sys.argv[1:]
    if args != ["acp"]:
        raise ValueError("Invalid restricted OpenCode arguments")
    profile = Path(profile_name).resolve(strict=True)
    workspace = Path.cwd().resolve(strict=True)
    home = profile / "home"
    executable = Path(executable).resolve(strict=True)
    config = json.loads(os.environ["OPENCODE_CONFIG_CONTENT"])
    provider = config["provider"]["session-provider"]
    options = provider["options"]
    endpoint = urlsplit(options["baseURL"])
    if (provider["npm"] != "@ai-sdk/openai-compatible"
            or endpoint.scheme != "http" or endpoint.netloc != f"127.0.0.1:{endpoint.port}"
            or not endpoint.port or endpoint.path != "/v1" or endpoint.query or endpoint.fragment):
        raise ValueError("Restricted OpenCode requires the owned Chat relay")
    root = os.environ["SHIJU_OPENCODE_SKILL_ROOT"]
    sources = json.loads(os.environ["SHIJU_OPENCODE_SKILL_SOURCES"])
    token = os.environ["VW_OPENCODE_RELAY_TOKEN"]
    state = {name: Path(os.environ[name]).resolve(strict=True)
             for name in ("XDG_DATA_HOME", "XDG_CACHE_HOME", "XDG_STATE_HOME")}
    reservation = socket.socket()
    reservation.bind(("127.0.0.1", 0))
    internal_port = reservation.getsockname()[1]
    env = {
        "PATH": "/usr/bin:/bin",
        "HOME": str(home),
        "TMPDIR": str(home),
        "OPENCODE_TEST_HOME": str(home),
        "OPENCODE_CONFIG_DIR": os.environ["OPENCODE_CONFIG_DIR"],
        "XDG_CONFIG_HOME": os.environ["XDG_CONFIG_HOME"],
        "OPENCODE_DISABLE_PROJECT_CONFIG": "1",
        "OPENCODE_DISABLE_MODELS_FETCH": "1",
        "OPENCODE_DISABLE_AUTOUPDATE": "1",
        "OPENCODE_DISABLE_LSP_DOWNLOAD": "1",
        "OPENCODE_SERVER_PASSWORD": secrets.token_hex(32),
        "VW_OPENCODE_RELAY_TOKEN": token,
        "SHIJU_OPENCODE_SKILL_ROOT": root,
        "VW_OC_RO": "1",
        **{name: str(path) for name, path in state.items()},
    }
    for name in ("OPENCODE_DISABLE_EXTERNAL_SKILLS", "OPENCODE_DISABLE_CLAUDE_CODE_SKILLS",
                 "OPENCODE_DISABLE_CLAUDE_CODE", "OPENCODE_PERMISSION"):
        if name in os.environ:
            env[name] = os.environ[name]
    config_scope = Path(env["XDG_CONFIG_HOME"]).resolve(strict=True) / "opencode"
    config_dir = Path(env["OPENCODE_CONFIG_DIR"]).resolve(strict=True)
    if config_scope.parent != profile / "xdg-config" or config_dir != profile / "config":
        raise ValueError("Invalid private OpenCode config directories")
    reads = " ".join(f"(subpath {quote(path)})"
                     for path in [profile, *state.values(), *sources, *([root] if root else [])])
    writes = " ".join(f"(subpath {quote(path)})" for path in [home, *state.values()])
    denied = " ".join(f"(subpath {quote(workspace / name)})"
                      for name in (".git", ".env", ".env.local", ".env.development",
                                   ".env.production", ".env.test", ".vwacprc.json", ".mcp-probe"))
    selected = " ".join(f"(subpath {quote(path)})" for path in sources)

    def configure(port):
        options["baseURL"] = f"http://127.0.0.1:{port}/v1"
        env["OPENCODE_CONFIG_CONTENT"] = json.dumps(config)

    def command(port):
        reservation.close()
        selection = f"(allow file-read-data {selected})" if selected else ""
        policy = f"""(version 1)
(allow default)
(deny file-read-data)
(allow file-read-data (literal "/") (subpath "/System") (subpath "/usr/lib") (subpath "/usr/share")
  (literal "/dev/null") (literal "/dev/urandom") (literal "/private/etc/hosts") (literal "/private/etc/localtime"))
(deny file-read-data (subpath "/System/Volumes/Data"))
(allow file-read-data (subpath {quote(workspace)}) (literal {quote(executable)}) {reads})
(deny file-read-data (subpath {quote(workspace / ".shiju")}))
{selection}
(deny file-read-data {denied})
(deny file-write*)
(allow file-write* {writes} (literal "/dev/null"))
(allow file-write-create (literal {quote(config_scope)}))
(allow file-write* (literal {quote(config_scope / ".gitignore")})
  (literal {quote(config_dir / ".gitignore")}))
(deny process-fork process-exec)
(allow process-exec (literal {quote(executable)}))
(deny network*)
(allow network-bind network-inbound (local ip "localhost:{internal_port}"))
(allow network-outbound (remote ip "localhost:{port}") (remote ip "localhost:{internal_port}"))
"""
        return ["/usr/bin/sandbox-exec", "-p", policy, str(executable), "acp", "--pure",
                "--hostname", "127.0.0.1", "--port", str(internal_port)]

    bridge = runpy.run_path(str(profile / "omp-mcp-bridge.py"))
    stdio = runpy.run_path(str(profile / "omp-mcp-stdio.py"))
    raise SystemExit(asyncio.run(bridge["run"](command, env, workspace, endpoint, token,
                                               configure, stdio["Stdio"], stdio["allowed_tool"])))


if __name__ == "__main__":
    main()
