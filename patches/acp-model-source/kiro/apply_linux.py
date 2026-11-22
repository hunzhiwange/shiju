"""Keep the managed Kiro relay credentials out of forwarded terminal requests."""
import hashlib
import json
from pathlib import Path
import sys


def apply(path):
    contract = json.loads(Path(__file__).with_name("linux.json").read_text())
    source = path.read_bytes()
    if hashlib.sha256(source).hexdigest() != contract["v2_server_sha256"]:
        raise SystemExit("Kiro Linux overlay requires the reviewed v2 server")
    before = b"return this.connection.sendRequest(rs.terminal_create,e,n=>zQt.create(n.terminalId,e.sessionId,this.connection))"
    after = (
        b'if(process.env.VW_ACP_LAUNCH_REFERENCE!==undefined&&e.env){'
        b'e={...e,env:e.env.filter(({name})=>name!=="KIRO_API_KEY"&&name!=="VW_ACP_LAUNCH_REFERENCE")}}'
        + before
    )
    if source.count(before) != 1:
        raise SystemExit("Kiro Linux terminal boundary differs")
    patched = source.replace(before, after)
    if hashlib.sha256(patched).hexdigest() != contract["server_sha256"]:
        raise SystemExit("Kiro Linux server checksum mismatch")
    path.write_bytes(patched)


if __name__ == "__main__":
    apply(Path(sys.argv[1]))
