"""Overlay the Linux GNU Nova runtime after applying the reviewed v3 patch."""
import hashlib
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

# The install overlay is restricted to Linux GNU by the host. The MCP gate
# remains closed on Linux because the stdio helper requires sandbox-exec.
patched_extension = rewrite(extension,
    "4dc6b7a014f47022e6915e0790cc0ae859863d3c1039d64727d4abe0992a4202",
    [(b"process.platform !== 'darwin' || process.arch !== 'arm64'",
      b"process.platform !== 'linux' || !['x64', 'arm64'].includes(process.arch)")])

for path, source in [(wrapper, patched_wrapper), (cli, patched_cli), (extension, patched_extension)]:
    path.write_bytes(source)
