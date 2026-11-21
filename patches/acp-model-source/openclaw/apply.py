"""Apply the pinned OpenClaw skill and MCP process ownership patches."""
import hashlib
import json
from pathlib import Path
import sys

spec = json.loads(Path(__file__).with_name("patch.json").read_text())
coexistence = json.loads(Path(__file__).with_name("coexistence-candidate.json").read_text())
for patch in (spec, spec["mcp_lifecycle"], coexistence):
    path = Path(sys.argv[1]) / patch["file"]
    source = path.read_text()
    if hashlib.sha256(source.encode()).hexdigest() != patch["original"]:
        raise RuntimeError("OpenClaw upstream checksum mismatch")
    for change in patch["changes"]:
        if source.count(change["old"]) != 1:
            raise RuntimeError("OpenClaw patch anchor mismatch")
        source = source.replace(change["old"], change["new"])
    if hashlib.sha256(source.encode()).hexdigest() != patch["patched"]:
        raise RuntimeError("OpenClaw patched checksum mismatch")
    path.write_text(source)
