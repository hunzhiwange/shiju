#!/usr/bin/env python3
"""Run the reviewed Stakpak binary with a request-private ACP workspace policy."""

import os
from pathlib import Path
import stat
import sys

NATIVE = Path(__file__).resolve().with_name("stakpak-native")
SANDBOX = """
(version 1)
(allow default)
(deny file-write*)
(allow file-write* (subpath (param "runtime")) (literal "/dev/null"))
(deny file-write* (subpath (param "skills")))
(deny process-exec)
(allow process-exec (literal (param "native")))
(deny network*)
(allow network-bind network-inbound (local ip "localhost:*"))
(allow network-outbound (remote ip "localhost:*"))
"""


def main():
    args = sys.argv[1:]
    runtime = os.environ.get("SHIJU_STAKPAK_RUNTIME")
    if "VW_ACP_LAUNCH_REFERENCE" not in os.environ and runtime is None:
        os.execv(NATIVE, [str(NATIVE), *args])
    if not runtime or args != ["acp"]:
        raise SystemExit("Stakpak session-model launch requires a private runtime and ACP")
    root = Path(runtime).resolve(strict=True)
    info = root.stat()
    if info.st_uid != os.geteuid() or stat.S_IMODE(info.st_mode) != 0o700:
        raise SystemExit("Stakpak session-model launch requires a private runtime")

    sandbox = "/usr/bin/sandbox-exec"
    env = {
        "PATH": "/usr/bin:/bin",
        "HOME": str(root),
        "TMPDIR": str(root),
        "SHIJU_STAKPAK_MANAGED_SKILLS": "1",
        "NO_PROXY": "127.0.0.1,localhost",
    }
    sys.dont_write_bytecode = True
    from mcp import run

    os.umask(0o077)
    try:
        run([
            sandbox, "-D", f"runtime={root}", "-D", f"native={NATIVE}",
            "-D", f"skills={root / '.stakpak/skills'}", "-p", SANDBOX, str(NATIVE), "acp",
        ], env, root)
    except EOFError:
        return
    except (OSError, ValueError):
        raise SystemExit("Stakpak managed ACP launch failed") from None


if __name__ == "__main__":
    main()
