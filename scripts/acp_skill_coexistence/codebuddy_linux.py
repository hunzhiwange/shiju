"""Exercise the fixed Linux CodeBuddy installation and chat through the gateway."""

import json
import os
from pathlib import Path
import platform
import sys
import tempfile
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from verify_acp_approval import environment, gateway, request
from verify_acp_integration import require
from verify_acp_model_source import Unavailable
from verify_acp_skill_coexistence import run


def verify(binary):
    require(sys.platform == "linux" and os.geteuid() != 0,
            "CodeBuddy acceptance requires a non-root Linux container")
    with tempfile.TemporaryDirectory(prefix="shiju-codebuddy-platform-") as directory:
        root = Path(directory).resolve()
        env = environment(root)
        # Keep an old gateway's generic npm fallback inside the test directory.
        env["npm_config_prefix"] = str(root / "npm")
        env["npm_config_ignore_scripts"] = "true"
        config = Path(env["VIBEWINDOW_CONFIG_DIR"])
        runtime = config / "acp-runtime/codebuddy-2.179.3-model-source-v2"
        with gateway(str(binary), root, env) as url:
            request(url, "/desktop/acp-install/codebuddy", method="POST")
            spec = json.loads(request(url, "/global/config/acp/defaults"))["codebuddy"]
            entry = Path(spec["command"])
            require(entry.is_absolute() and spec.get("args") == ["--acp"],
                    "gateway did not select the reviewed ACP entry")
            expected = runtime / "node_modules/@tencent-ai/codebuddy-code/bin/codebuddy"
            require(entry.resolve(strict=True) == expected.resolve(strict=True),
                    "gateway did not select the fixed CodeBuddy package")
            arch = {"x86_64": "x64", "aarch64": "arm64"}[platform.machine()]
            helper = expected.parents[1] / f"vendor/ripgrep/{arch}-linux/rg"
            original = helper.read_bytes()
            try:
                helper.write_bytes(b"modified native helper")
                fallback = json.loads(request(url, "/global/config/acp/defaults"))["codebuddy"]
                require(Path(fallback["command"]).resolve() != expected.resolve(),
                        "gateway reused CodeBuddy after the native helper changed")
            finally:
                helper.write_bytes(original)
        report = {"turns": []}
        with patch.dict(os.environ, CODEBUDDY_COEXISTENCE_RUNTIME=str(runtime)):
            run("codebuddy", binary, report)
        require(report["artifact"]["version"] == "2.179.3",
                "gateway probe selected the wrong version")
        require(report["artifact"]["entry"] == "bin/codebuddy",
                "gateway probe selected the wrong entry")
        mcp = [row["mcp_calls"] for row in report["turns"]]
        require(mcp == [0, 1, 0, 0], "MCP execution count changed after rejection or removal")
        print(json.dumps({
            "target": f"linux-{platform.machine()}",
            "entry": report["artifact"]["entry"],
            "version": report["artifact"]["version"],
            "model_endpoint": "/v1/chat/completions",
            "model_requests": [row["model_requests"] for row in report["turns"]],
            "mcp_calls": mcp,
        }))


if __name__ == "__main__":
    try:
        verify(Path(sys.argv[1]).resolve(strict=True))
    except AssertionError as error:
        sys.exit(str(error))
    except (OSError, Unavailable, ValueError, KeyError, TypeError) as error:
        sys.exit(f"CodeBuddy platform probe failed at {type(error).__name__}")
