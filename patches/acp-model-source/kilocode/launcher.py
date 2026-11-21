"""Run the reviewed Kilo entry inside Seatbelt with the existing MCP bridge."""
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
    if args != ["acp", "--pure"]:
        raise ValueError("Invalid restricted Kilocode arguments")
    executable = Path(executable).resolve(strict=True)
    profile = Path(profile_name).resolve(strict=True)
    workspace = Path.cwd().resolve(strict=True)
    home = profile / "home"
    config = json.loads(os.environ["KILO_CONFIG_CONTENT"])
    provider_id, _ = config["model"].split("/", 1)
    provider = config["provider"][provider_id]
    options = provider["options"]
    endpoint = urlsplit(options["baseURL"])
    if (list(config["provider"]) != [provider_id]
            or provider["npm"] != "@ai-sdk/openai-compatible"
            or endpoint.scheme != "http" or endpoint.netloc != f"127.0.0.1:{endpoint.port}"
            or not endpoint.port or endpoint.path != "/v1" or endpoint.query or endpoint.fragment):
        raise ValueError("Restricted Kilocode requires the owned Chat relay")

    root = os.environ["SHIJU_KILO_SKILL_ROOT"]
    sources = json.loads(os.environ["SHIJU_KILO_SKILL_SOURCES"])
    token = os.environ["VW_KILO_RELAY_TOKEN"]
    state = {name: Path(os.environ[name]).resolve(strict=True) for name in (
        "XDG_DATA_HOME", "XDG_CACHE_HOME", "XDG_STATE_HOME", "XDG_CONFIG_HOME",
    )}
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        internal_port = reservation.getsockname()[1]

    env = {
        "PATH": "/usr/bin:/bin",
        "HOME": str(home),
        "TMPDIR": str(home),
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_CONFIG_GLOBAL": "/dev/null",
        "KILO_DISABLE_PROJECT_CONFIG": "1",
        "KILO_DISABLE_MODELS_FETCH": "1",
        "KILO_DISABLE_AUTOUPDATE": "1",
        "KILO_DISABLE_LSP_DOWNLOAD": "1",
        "KILO_DISABLE_SKILL_SHELL": "1",
        "KILO_DISABLE_CODEBASE_INDEXING": "vscode-no-workspace",
        "KILO_SERVER_PASSWORD": secrets.token_hex(32),
        "VW_KILO_RELAY_TOKEN": token,
        "SHIJU_KILO_RESTRICTED": "1",
        "SHIJU_KILO_SKILL_ROOT": root,
        **{name: str(path) for name, path in state.items()},
    }
    for name in (
        "KILO_DISABLE_CLAUDE_CODE",
        "KILO_DISABLE_CLAUDE_CODE_PROMPT",
        "KILO_DISABLE_CLAUDE_CODE_SKILLS",
        "KILO_DISABLE_EXTERNAL_SKILLS",
    ):
        if name in os.environ:
            env[name] = os.environ[name]
    reads = " ".join(f"(subpath {quote(path)})"
                     for path in [profile, *state.values(), *([root] if root else [])])
    writes = " ".join(f"(subpath {quote(path)})" for path in [home, *state.values()])
    denied = " ".join(f"(subpath {quote(workspace / name)})"
                      for name in (".git", ".shiju", ".env", ".vwacprc.json"))
    selected = " ".join(f"(subpath {quote(path)})" for path in sources)

    def configure(port):
        options["baseURL"] = f"http://127.0.0.1:{port}/v1"
        env["KILO_CONFIG_CONTENT"] = json.dumps(config)

    def command(port):
        selection = f"(allow file-read-data {selected})" if selected else ""
        policy = f"""(version 1)
(allow default)
(deny file-read-data)
(allow file-read-data (literal "/") (subpath "/System") (subpath "/usr/lib") (subpath "/usr/share")
  (literal "/dev/null") (literal "/dev/urandom") (literal "/private/etc/hosts") (literal "/private/etc/localtime"))
(deny file-read-data (subpath "/System/Volumes/Data"))
(allow file-read-data (subpath {quote(workspace)}) (literal {quote(executable)})
  (literal "/usr/bin/git") (literal "/usr/bin/xcrun")
  (subpath "/Library/Developer/CommandLineTools/usr/lib") {reads})
(deny file-read-data {denied})
{selection}
(deny file-write*)
(allow file-write* {writes} (literal "/dev/null"))
(deny process-exec)
(allow process-exec (literal {quote(executable)}) (literal "/usr/bin/git"))
(deny network*)
(allow network-bind network-inbound (local ip "localhost:{internal_port}"))
(allow network-outbound (remote ip "localhost:{port}") (remote ip "localhost:{internal_port}"))
"""
        return ["/usr/bin/sandbox-exec", "-p", policy, str(executable), *args,
                "--hostname", "127.0.0.1", "--port", str(internal_port)]

    bridge = runpy.run_path(str(profile / "omp-mcp-bridge.py"))
    stdio = runpy.run_path(str(profile / "omp-mcp-stdio.py"))
    raise SystemExit(asyncio.run(bridge["run"](command, env, workspace, endpoint, token,
                                               configure, stdio["Stdio"], stdio["allowed_tool"])))


if __name__ == "__main__":
    main()
