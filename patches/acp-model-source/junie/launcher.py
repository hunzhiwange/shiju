#!/usr/bin/env -S python3 -I
"""Launch the pinned Junie bundle with a request-owned model and Seatbelt policy."""
import asyncio
import json
import os
from pathlib import Path
import re
import pwd
import runpy
import stat
import subprocess
import sys
from urllib.parse import urlsplit


def private(path, kind, mode):
    info = path.lstat()
    if not kind(info.st_mode) or info.st_uid != os.geteuid() or stat.S_IMODE(info.st_mode) != mode:
        raise ValueError("Insecure private Junie path")


def policy(native, app, root, state, workspace, port, skill_reads):
    quote = json.dumps
    user_home = Path(pwd.getpwuid(os.geteuid()).pw_dir).resolve()
    skill_paths = " ".join(f'(subpath {quote(str(path))})' for path in skill_reads)
    return (
        '(version 1)(allow default)'
        '(deny network*)(deny file-write*)(deny process-exec)'
        f'(deny file-read-data (subpath {quote(str(user_home))}) (subpath "/private/tmp"))'
        f'(allow file-read-data (subpath {quote(str(app))}) '
        f'(subpath {quote(str(root))}) (subpath {quote(str(workspace))}))'
        f'(allow network-outbound (remote tcp "localhost:{port}"))'
        f'(allow process-exec (literal {quote(str(native))}))'
        f'(allow file-write* (subpath {quote(str(state))}) (subpath {quote(str(workspace))}))'
        + (f'(allow file-read-data {skill_paths})(deny file-write* {skill_paths})' if skill_paths else '')
        + '(deny file-write* (regex #"/[.]git(/|$)"))'
    )


def main():
    app = Path(__file__).resolve().parent / "Applications/junie.app"
    native = (app / "Contents/MacOS/junie").resolve(strict=True)
    subprocess.run(
        ["/usr/bin/codesign", "--verify", "--deep", "--strict", str(app)],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True,
    )
    if sys.argv[1:] == ["--version"]:
        print("Junie version: 26.11.2 (4100.5)")
        return
    if sys.argv[1:] != ["--acp=true"]:
        raise ValueError("Unexpected Junie launch arguments")
    config = Path(os.environ["SHIJU_JUNIE_CONFIG"]).resolve(strict=True)
    root = config.parent
    private(root, stat.S_ISDIR, 0o700)
    private(config, stat.S_ISREG, 0o600)
    data = json.loads(config.read_text())
    reference = os.environ["VW_ACP_LAUNCH_REFERENCE"]
    if not re.fullmatch(r"vw-session-model:[0-9a-f]{64}", reference) or data["reference"] != reference:
        raise ValueError("Junie launch reference mismatch")
    workspace = Path(data["workspace"]).resolve(strict=True)
    endpoint = urlsplit(data["endpoint"])
    if (not workspace.is_dir() or workspace == workspace.parent
            or endpoint.scheme != "http" or endpoint.netloc != f"127.0.0.1:{endpoint.port}"
            or not endpoint.port or endpoint.path not in ("", "/") or endpoint.query or endpoint.fragment):
        raise ValueError("Invalid Junie workspace or relay")
    skill_locations = []
    project_skills = workspace / ".junie/skills"
    if project_skills.is_dir() and project_skills.resolve() == project_skills:
        skill_locations.append(project_skills)
    skill_reads = []
    selection = root / "junie-skills.json"
    if selection.exists() or selection.is_symlink():
        private(selection, stat.S_ISREG, 0o600)
        selected = json.loads(selection.read_text())
        view = Path(selected["root"])
        if view != view.resolve(strict=True):
            raise ValueError("Noncanonical Junie skill view")
        private(view.parent, stat.S_ISDIR, 0o700)
        if view.name != "skills" or not view.parent.name.startswith("shiju-session-skills-"):
            raise ValueError("Invalid Junie skill view")
        sources = selected["sources"]
        if not isinstance(sources, dict) or not sources or set(sources) != {p.name for p in view.iterdir()}:
            raise ValueError("Invalid Junie skill view entries")
        skill_reads.append(view)
        for name, path in sources.items():
            link = view / name
            source = Path(path)
            if (not link.is_symlink() or source != link.resolve(strict=True)
                    or not source.is_dir() or source == source.parent):
                raise ValueError("Invalid Junie skill source")
            skill_reads.append(source)
        skill_locations.append(view)
    state = root / "state"
    profiles = root / "models"
    state.mkdir(mode=0o700)
    profiles.mkdir(mode=0o700)
    def configure(port):
        with os.fdopen(os.open(profiles / "shiju.json", os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as file:
            json.dump({
                "id": data["model"],
                "baseUrl": f"http://127.0.0.1:{port}/v1/chat/completions",
                "apiType": "OpenAICompletion",
                "apiKey": data["token"],
                "maxContextLength": data["context"],
            }, file)
    env = {
        "PATH": "/usr/bin:/bin", "HOME": str(state), "JUNIE_HOME": str(state / "junie"),
        "TMPDIR": str(state), "NO_PROXY": "127.0.0.1",
    }
    os.chdir(workspace)
    def command(port):
        return [
            "/usr/bin/sandbox-exec", "-p", policy(native, app, root, state, workspace, port, skill_reads), str(native),
            "--acp=true", "--model=custom:shiju", "--model-location=" + str(profiles),
            "--model-default-locations=false", "--config-default-locations=false",
            "--mcp-default-locations=false", "--skill-default-locations=false",
            "--command-default-location=false", "--agent-default-location=false",
            "--extensions-default-location=" + str(state / "extensions"),
            "--skip-update-check", "--share-anonymous-statistics=false",
            *("--skill-location=" + str(path) for path in skill_locations),
        ]

    bridge = runpy.run_path(str(app.parent.parent / "mcp_bridge.py"))
    stdio = runpy.run_path(str(app.parent.parent / "mcp_stdio.py"))
    raise SystemExit(asyncio.run(bridge["run"](
        command, env, workspace, endpoint, data["token"], configure,
        stdio["Stdio"], stdio["allowed_tool"],
    )))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, ValueError, subprocess.CalledProcessError):
        raise SystemExit("Junie managed launch rejected") from None
