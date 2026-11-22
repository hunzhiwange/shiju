"""Apply the Linux GNU overlay after the reviewed Qoder model-source patch."""

import hashlib
import json
import platform
from pathlib import Path
import sys


def replace_once(source, anchor, replacement):
    if source.count(anchor) != 1:
        raise RuntimeError("Qoder Linux overlay anchor mismatch")
    return source.replace(anchor, replacement, 1)


def main():
    root = Path(sys.argv[1]).resolve(strict=True)
    contract = json.loads(Path(__file__).with_name("linux.json").read_text())
    arch = {"x86_64": "x64", "aarch64": "arm64"}.get(platform.machine())
    if platform.system() != "Linux" or arch is None or platform.libc_ver()[0] != "glibc":
        raise RuntimeError("Qoder Linux overlay requires x64 or ARM64 GNU/Linux")
    rg = contract["ripgrep"][arch]
    node_modules = root.parent.parent
    entry = node_modules / rg["package"] / "bin/rg"
    packages = json.loads((node_modules.parent / "package-lock.json").read_text())["packages"]
    lock = packages["node_modules/" + rg["package"]]
    if lock.get("version") != contract["version"] or lock.get("integrity") != rg["integrity"]:
        raise RuntimeError("Qoder ripgrep archive integrity mismatch")
    if (entry.resolve(strict=True) != entry or not entry.is_file()
            or not entry.stat().st_mode & 0o111
            or hashlib.sha256(entry.read_bytes()).hexdigest() != rg["sha256"]):
        raise RuntimeError("Qoder ripgrep artifact mismatch")

    module = root / "bundle/shiju-session-model.mjs"
    binding = replace_once(
        module.read_text(),
        "process.platform !== 'darwin' || process.arch !== 'arm64'",
        "!supportedTarget",
    )
    binding = replace_once(
        binding, "private Darwin arm64 session binding", "private Linux GNU session binding",
    )
    binding = replace_once(binding, "import dns from 'node:dns';",
                           "import dns from 'node:dns';\nimport {fileURLToPath} from 'node:url';")
    binding = replace_once(
        binding,
        "let binding;",
        "const supportedTarget = active && process.platform === 'linux'"
        " && ['x64', 'arm64'].includes(process.arch)"
        " && !!process.report.getReport().header.glibcVersionRuntime;\nlet binding;",
    )
    binding += """
// The installer and host verify this architecture's exact npm helper.
export function ripgrep() {
  validate();
  return {
    mode: 'builtin',
    command: fileURLToPath(new URL(`../../qodercli-ripgrep-linux-${process.arch}/bin/rg`, import.meta.url)),
    args: [],
  };
}
"""
    bundle = root / "bundle/qodercli.js"
    source = replace_once(bundle.read_text(), "function FBn(){if(Fx)return Fx;",
                          "function FBn(){if(Fx)return Fx;if(shijuQoder.active)return Fx=shijuQoder.ripgrep();")
    for file, contents in ((module, binding), (bundle, source)):
        data = contents.encode()
        if hashlib.sha256(data).hexdigest() != contract["files"][str(file.relative_to(root))]:
            raise RuntimeError("Qoder Linux patched artifact checksum mismatch")
    module.write_text(binding)
    bundle.write_text(source)


if __name__ == "__main__":
    main()
