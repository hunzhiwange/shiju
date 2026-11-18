#!/usr/bin/python3 -I
"""Run native Corust with a request-local model endpoint and workspace sandbox."""

import json
import os
from pathlib import Path
import stat
import sys
from urllib.parse import urlsplit

NATIVE = Path(__file__).resolve().with_name("corust-agent-native")
SANDBOX = """
(version 1)
(allow default)
(deny file-read-data (subpath "/Users") (subpath "/private/tmp")
    (subpath "/private/var/folders") (subpath (param "host-home")))
(allow file-read-data
    (subpath (param "workspace")) (subpath (param "runtime"))
    (literal (param "native")) (literal "/dev/null"))
(deny file-write*)
(allow file-write* (subpath (param "workspace")) (subpath (param "runtime"))
    (literal "/dev/null"))
(deny file-read-data file-write* (subpath (param "git")))
(deny process-fork process-exec)
(allow process-exec (literal (param "native")))
(deny network*)
(allow network-outbound (remote ip (param "relay")))
"""


def main():
    configuration = os.environ.get("SHIJU_CORUST_LAUNCH")
    if not configuration and "VW_ACP_LAUNCH_REFERENCE" not in os.environ:
        os.execv(NATIVE, [str(NATIVE), *sys.argv[1:]])
    if not configuration or sys.argv[1:]:
        raise SystemExit("Corust session-model launch requires a private configuration")
    path = Path(configuration)
    info = path.lstat()
    runtime = path.parent.resolve(strict=True)
    private = runtime.stat()
    if (
        not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) != 0o600
        or info.st_uid != os.geteuid() or private.st_uid != os.geteuid()
        or stat.S_IMODE(private.st_mode) != 0o700
    ):
        raise SystemExit("Insecure Corust launch configuration")
    settings = json.loads(path.read_text())
    workspace = Path(settings["workspace"]).resolve(strict=True)
    endpoint = urlsplit(settings["endpoint"])
    host_home = Path.home().resolve(strict=True)
    if (
        workspace != Path.cwd().resolve() or not workspace.is_dir() or workspace == workspace.parent
        or workspace == host_home or workspace in host_home.parents
        or endpoint.scheme != "http" or endpoint.hostname != "127.0.0.1"
        or not endpoint.port or endpoint.username or endpoint.password
        or endpoint.path != "/" or endpoint.query or endpoint.fragment
        or not settings["token"]
    ):
        raise SystemExit("Invalid Corust workspace or relay binding")
    sandbox = "/usr/bin/sandbox-exec"
    os.execve(sandbox, [
        sandbox, "-D", f"runtime={runtime}", "-D", f"workspace={workspace}",
        "-D", f"host-home={host_home}", "-D", f"native={NATIVE}", "-D", f"git={workspace / '.git'}",
        "-D", f"relay=localhost:{endpoint.port}", "-p", SANDBOX, str(NATIVE),
    ], {
        "PATH": "/usr/bin:/bin",
        "HOME": str(runtime),
        "TMPDIR": str(runtime),
        "CORUST_HOME": str(runtime / "corust"),
        "CORUST_API_KEY": settings["token"],
        "NO_PROXY": "127.0.0.1,localhost",
    })


if __name__ == "__main__":
    main()
