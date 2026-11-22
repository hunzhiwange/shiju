"""Patch the reviewed Auggie v2 bundle in a staged Linux installation."""

import hashlib
from pathlib import Path
import sys

V2_DIGESTS = {
    "augment.mjs": "c8d0c76c7ef7eaa1a4837f7cda2eec3eeecd9d8bf28da2db9697993d98300ba9",
    "session-model.mjs": "7d0afac3f9242e473a40a42f75aebf7bc48fd611aee50f375a04d92afc3fe162",
}
CALL = "y=()=>s.callTool({name:c,arguments:i},t.laxCallToolResultSchema,A);"
CALL_WITH_ID = (
    "y=()=>{let params={name:c,arguments:i};"
    'if(process.platform==="linux"&&process.env.SHIJU_AUGGIE_RESTRICTED==="1"){'
    'if(!JSON.parse(process.env.SHIJU_AUGGIE_MCP_TOOLS??"[]").includes(r)'
    '||typeof n!=="string"||!n)throw new Error("Auggie MCP call has no managed identity");'
    'params._meta={"shiju.auggie.toolCallId":n}}'
    "return s.callTool(params,t.laxCallToolResultSchema,A)};"
)


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: apply-linux.py <staged-auggie-package>")
    package = Path(sys.argv[1]).resolve(strict=True)
    for name, digest in V2_DIGESTS.items():
        path = package / name
        if path.is_symlink() or not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise RuntimeError("Auggie Linux patch requires the reviewed v2 files")
    entry = package / "augment.mjs"
    source = entry.read_text(encoding="utf-8")
    if source.count(CALL) != 1:
        raise RuntimeError("Auggie Linux MCP call anchor mismatch")
    entry.write_text(source.replace(CALL, CALL_WITH_ID, 1), encoding="utf-8")


if __name__ == "__main__":
    main()
