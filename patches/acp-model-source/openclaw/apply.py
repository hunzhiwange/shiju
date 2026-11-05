"""Apply only the pinned OpenClaw Gateway session skill source contract."""
import hashlib
import json
from pathlib import Path
import sys
spec = json.loads(Path(__file__).with_name("patch.json").read_text())
p = Path(sys.argv[1]) / spec["file"]
s = p.read_text()
if hashlib.sha256(s.encode()).hexdigest() != spec["original"]:
    raise RuntimeError("OpenClaw upstream checksum mismatch")
for change in spec["changes"]:
    if s.count(change["old"]) != 1:
        raise RuntimeError("OpenClaw patch anchor mismatch")
    s = s.replace(change["old"], change["new"])
if hashlib.sha256(s.encode()).hexdigest() != spec["patched"]:
    raise RuntimeError("OpenClaw patched checksum mismatch")
p.write_text(s)
