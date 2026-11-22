"""Apply the reviewed Snow 0.9.14 session-scoped skill and MCP changes."""
import hashlib
import json
from pathlib import Path
import sys

def apply(data, name):
    spec = json.loads(Path(__file__).with_name(name).read_text())
    if hashlib.sha256(data).hexdigest() != spec["original"]:
        raise RuntimeError(f"Snow patch input checksum mismatch: {name}")
    text = data.decode()
    for change in spec["changes"]:
        if text.count(change["old"]) != 1:
            raise RuntimeError(f"Snow patch anchor mismatch: {name}")
        text = text.replace(change["old"], change["new"])
    data = text.encode()
    if hashlib.sha256(data).hexdigest() != spec["patched"]:
        raise RuntimeError(f"Snow patched checksum mismatch: {name}")
    return data


if len(sys.argv) < 2 or sys.argv[2:] not in ([], ["--linux"]):
    raise SystemExit("usage: apply.py package-directory [--linux]")
path = Path(sys.argv[1]) / "bundle/cli.mjs"
data = apply(path.read_bytes(), "patch.json")
if sys.argv[2:] == ["--linux"]:
    data = apply(data, "linux.json")
    projection = Path(__file__).with_name("linux-mcp.mjs").read_bytes()
    spec = json.loads(Path(__file__).with_name("linux.json").read_text())
    if hashlib.sha256(projection).hexdigest() != spec["projection_sha256"]:
        raise RuntimeError("Snow MCP projection checksum mismatch")
    path.with_name("shiju-mcp.mjs").write_bytes(projection)
path.write_bytes(data)
