"""Apply the reviewed Snow 0.9.14 session-only native skill scanner."""
import hashlib
import json
from pathlib import Path
import sys

spec = json.loads(Path(__file__).with_name("patch.json").read_text())
path = Path(sys.argv[1]) / "bundle/cli.mjs"
data = path.read_bytes()
if hashlib.sha256(data).hexdigest() != spec["original"]:
    raise RuntimeError("Snow upstream checksum mismatch")
text = data.decode()
for change in spec["changes"]:
    if text.count(change["old"]) != 1:
        raise RuntimeError("Snow patch anchor mismatch")
    text = text.replace(change["old"], change["new"])
if hashlib.sha256(text.encode()).hexdigest() != spec["patched"]:
    raise RuntimeError("Snow patched checksum mismatch")
path.write_text(text)
