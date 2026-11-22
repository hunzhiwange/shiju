"""Exercise the reviewed Linux Dirac installation and session through the gateway."""

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

REVISION = "dirac-0.5.24-model-source-v3"
BIN = Path("node_modules/dirac-cli/dist/shiju-dirac.mjs")
RG = Path("node_modules/dirac-cli/dist/shiju-rg")


def verify(binary):
    require(sys.platform == "linux" and os.geteuid() != 0,
            "Dirac acceptance requires a non-root Linux container")
    with tempfile.TemporaryDirectory(prefix="shiju-dirac-platform-") as directory:
        root = Path(directory).resolve()
        env = environment(root)
        env["npm_config_prefix"] = str(root / "npm")
        env["npm_config_ignore_scripts"] = "true"
        runtime = Path(env["VIBEWINDOW_CONFIG_DIR"]) / "acp-runtime" / REVISION
        with gateway(str(binary), root, env) as url:
            request(url, "/desktop/acp-install/dirac", method="POST")
            spec = json.loads(request(url, "/global/config/acp/defaults"))["dirac"]
            entry = runtime / BIN
            require(Path(spec["command"]).resolve(strict=True) == entry.resolve(strict=True)
                    and spec["args"] == ["--acp"],
                    "gateway did not select the reviewed Linux Dirac entry")
            helper = runtime / RG
            original = helper.read_bytes()
            try:
                helper.write_bytes(b"modified native helper")
                fallback = json.loads(request(url, "/global/config/acp/defaults"))["dirac"]
                require(Path(fallback["command"]).resolve() != entry.resolve(),
                        "gateway selected Dirac after the native helper changed")
            finally:
                helper.write_bytes(original)

        report = {"turns": []}
        with patch.dict(os.environ, SHIJU_DIRAC_COEXISTENCE_RUNTIME=str(runtime)):
            run("dirac", binary, report)
        artifact = report["artifact"]
        require(artifact["version"] == "0.5.24" and artifact["revision"] == REVISION,
                "unexpected Dirac runtime in the model and skill probe")
        require(artifact["model_endpoint"] == "/v1/chat/completions",
                "unexpected model protocol in the Dirac probe")
        require(artifact["target"] == platform.machine()
                and artifact["entry"] == "dist/shiju-dirac.mjs",
                "unexpected Dirac architecture or entry in the model probe")
        require(all(turn["model_requests"] >= len(turn["calls"]) + 2 for turn in report["turns"]),
                "Dirac model did not receive the native search continuation")
        require([turn["mcp_calls"] for turn in report["turns"]] == [0, 1, 0, 0],
                "Dirac MCP audit changed after rejection or removal")
        print(json.dumps(report))


if __name__ == "__main__":
    try:
        verify(Path(sys.argv[1]).resolve(strict=True))
    except AssertionError as error:
        sys.exit(str(error))
    except (OSError, Unavailable, ValueError, KeyError, TypeError) as error:
        sys.exit(f"Dirac Linux probe failed at {type(error).__name__}")
