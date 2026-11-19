"""Create a patched Dirac entry inside an isolated 0.5.24 dist directory."""
from pathlib import Path
import hashlib
import shutil
import sys

if len(sys.argv) != 2:
    raise SystemExit("usage: apply.py <private-dirac-cli-dist>")

dist = Path(sys.argv[1]).resolve(strict=True)
entry = dist / "cli.mjs"
upstream = entry.read_bytes()
if hashlib.sha256(upstream).hexdigest() != "c88e043f590a520da3345050e32bbe6930a68cefad60945ef4a7d14648217429":
    raise RuntimeError("Dirac 0.5.24 upstream checksum mismatch")

output = dist / "shiju-dirac.mjs"
helper = dist / "session-mcp.mjs"
if output.exists() or output.is_symlink() or helper.exists() or helper.is_symlink():
    raise RuntimeError("Dirac patch outputs already exist")

s = upstream.decode()
pairs = [
    ('const require = _createRequire(import.meta.url);',
     'import * as shijuMcp from "./session-mcp.mjs";\nconst require = _createRequire(import.meta.url);'),
    ('async newSession(e){let r=crypto.randomUUID(),i=this.createStartupSessionOverrides();',
     'async newSession(e){let mounts=shijuMcp.validate(e.mcpServers);let r=crypto.randomUUID(),i=this.createStartupSessionOverrides();'),
    ('this.sessionStates.set(r,h),{sessionId:r,modes:this.sessionConfig',
     'this.sessionStates.set(r,h),shijuMcp.save(u,mounts),{sessionId:r,modes:this.sessionConfig'),
    ('async prompt(e){let r=this.sessions.get(e.sessionId),i=this.sessionStates.get(e.sessionId);',
     'async prompt(e){return shijuMcp.prompt(this.sessions.get(e.sessionId),()=>this.promptNative(e),(call,options)=>this.requestPermission(e.sessionId,call,options),update=>this.emitSessionUpdate(e.sessionId,update))}async promptNative(e){let r=this.sessions.get(e.sessionId),i=this.sessionStates.get(e.sessionId);'),
    ('async cancel(e){let r=this.sessions.get(e.sessionId);',
     'async cancel(e){let r=this.sessions.get(e.sessionId);shijuMcp.stop(r);'),
    ('async loadSession(e){let r=e.sessionId,i=this.sessions.get(r);',
     'async loadSession(e){if(e.mcpServers?.length||shijuMcp.hasMounts(this.sessions.get(e.sessionId)))throw new Error("Dirac MCP prototype supports only new sessions");let r=e.sessionId,i=this.sessions.get(r);'),
    ('async unstable_forkSession(e){let r=e.sessionId,i=crypto.randomUUID()',
     'async unstable_forkSession(e){if(e.mcpServers?.length)throw new Error("Dirac MCP prototype supports only new sessions");let r=e.sessionId,i=crypto.randomUUID()'),
    ('async releaseSessionResources(e,r=!1){let i=this.sessions.get(e);',
     'async releaseSessionResources(e,r=!1){let i=this.sessions.get(e);shijuMcp.stop(i);'),
    ('async requestPermission(e,r,i){if(!this.permissionHandler)',
     'async requestPermission(e,r,i){if(shijuMcp.hasMounts(this.sessions.get(e))&&this.pendingPermissionResolvers.has(e))return{outcome:{outcome:"cancelled"}};if(!this.permissionHandler)'),
    ('p=this.mergeTools(u,c).filter(x=>hT(x.spec)&&(this.options.isToolAvailable?.(x)??!0)),h=',
     'p=shijuMcp.expose(this.mergeTools(u,c).filter(x=>hT(x.spec)&&(this.options.isToolAvailable?.(x)??!0))),h='),
]
for original, replacement in pairs:
    if s.count(original) != 1:
        raise RuntimeError("Dirac patch anchor mismatch: " + original)
    s = s.replace(original, replacement, 1)
with helper.open("xb") as stream:
    with Path(__file__).with_name("session-mcp.mjs").open("rb") as source:
        shutil.copyfileobj(source, stream)
with output.open("x") as stream:
    stream.write(s)
