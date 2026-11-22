"""Apply Linux isolation and tool consent to the reviewed Kiro V3 server."""
import hashlib
import json
from pathlib import Path
import sys


def apply(path):
    contract = json.loads(Path(__file__).with_name("linux.json").read_text())
    source = path.read_bytes()
    if hashlib.sha256(source).hexdigest() != contract["v2_server_sha256"]:
        raise SystemExit("Kiro Linux overlay requires the reviewed v2 server")
    managed = b"shijuKiroManaged"
    entry = b'import { createRequire as __createRequire } from "module";'
    terminal = b"return this.connection.sendRequest(rs.terminal_create,e,n=>zQt.create(n.terminalId,e.sessionId,this.connection))"
    auth = b"o0r=process.env.KIRO_API_KEY?.trim()"
    mcp_file = b"readConfigFile(e,n){if(!AAe.existsSync(e))return{};"
    consent = b"return{...y,result:NJr(y.result)}"
    protected = b'"shell","fs_write","mcp"'
    changes = {
        entry: (
            entry + b'\nconst shijuKiroManaged=process.env.VW_ACP_LAUNCH_REFERENCE!==undefined;'
            b'if(shijuKiroManaged)delete process.env.VW_ACP_LAUNCH_REFERENCE;\n'
        ),
        terminal: (
            b'if(' + managed + b'&&e.env){e={...e,env:e.env.filter(({name})=>'
            b'name!=="KIRO_API_KEY"&&name!=="VW_ACP_LAUNCH_REFERENCE")}}' + terminal
        ),
        # The server passes the captured key to its explicit auth provider.
        # Remove it before KAS starts any tools with process.env.
        auth: auth + b',shijuKiroKeyConsumed=' + managed + b'&&delete process.env.KIRO_API_KEY',
        # ACP MCP remains available through session/new and session/load. Workspace
        # MCP JSON can spawn a command before an ACP tool approval exists.
        mcp_file: mcp_file.replace(b"if(!AAe.existsSync(e))", b"if(" + managed + b"||!AAe.existsSync(e))"),
        b"profileServers:r.profile,builtinServers:r.builtin,clientServers:r.client": (
            b"profileServers:" + managed + b"?[]:r.profile,builtinServers:r.builtin,clientServers:r.client"
        ),
        # Use the real upstream consent desk. Preserve all resolved denies and
        # prevent the classifier and local allow rules from granting these calls.
        consent: (
            b'const result=NJr(y.result);'
            b'const capability=f.capability??Yb(f.toolId,f.toolTags);'
            b'if(' + managed + b'&&result.effect!=="deny"&&[' + protected + b'].includes(capability))'
            b'return{...y,result:{...result,effect:"ask",isExplicitAsk:true}};'
            b'return{...y,result}'
        ),
        # Persistable native approvals must never substitute for a turn approval.
        b'...l?{}:{persistableConsent:!1,persistableConsentReason:HJu(r)}': (
            b'...l&&!(' + managed + b'&&[' + protected + b'].includes(a))'
            b'?{}:{persistableConsent:!1,persistableConsentReason:HJu(r)}'
        ),
    }
    patched = source
    for before, after in changes.items():
        if patched.count(before) != 1:
            raise SystemExit("Kiro Linux isolation boundary differs")
        patched = patched.replace(before, after)
    if hashlib.sha256(patched).hexdigest() != contract["server_sha256"]:
        raise SystemExit("Kiro Linux server checksum mismatch")
    path.write_bytes(patched)


if __name__ == "__main__":
    apply(Path(sys.argv[1]))
