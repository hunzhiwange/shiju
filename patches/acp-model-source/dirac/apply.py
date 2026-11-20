"""Create a patched Dirac entry inside an isolated 0.5.24 dist directory."""
from pathlib import Path
import hashlib
import json
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

s = upstream.decode().removeprefix("\n")
respond_instruction = (
    r"- Use \`respond\` for user-facing communication: \`progress\` for meaningful updates during longer work, "
    r"\`question\` when you need information from the user, \`plan\` for Plan-mode answers or proposals, "
    r"and \`complete\` for final Act-mode results. Do not send plain assistant text; every response must include a tool call."
)
respond_default = json.dumps(respond_instruction.replace(r"\`", "`"))
plain_instruction = json.dumps("- Use plain assistant text for the final answer. Use only the callable tools shown for this turn.")
pairs = [
    (respond_instruction, "${shijuMcp.plainTextMode() ? " + plain_instruction + " : " + respond_default + "}"),
    ('if(t.taskState.didAttemptCompletion)return t.taskState.completionCommitted=!1',
     'if(!a&&await shijuMcp.completePlainText(t,e.stopReason,e.assistantTextOnly))return !0;'
     'if(t.taskState.didAttemptCompletion)return t.taskState.completionCommitted=!1'),
    ('return await Ell(t,{assistantHasContent:K,stopReason:W,',
     'return await Ell(t,{assistantHasContent:K,assistantTextOnly:$,stopReason:W,'),
    ('await this.emitSessionUpdate(e,{sessionUpdate:r.content.isReasoning?"agent_thought_chunk":"agent_message_chunk",content:{type:"text",text:i}})',
     'if(r.content.isReasoning||!shijuMcp.plainTextMode())await this.emitSessionUpdate(e,{sessionUpdate:r.content.isReasoning?"agent_thought_chunk":"agent_message_chunk",content:{type:"text",text:i}})'),
    ('y&&await this.emitSessionUpdate(e,{sessionUpdate:E,content:{type:"text",text:C?',
     'y&&(i.content.isReasoning||!shijuMcp.plainTextMode())&&await this.emitSessionUpdate(e,{sessionUpdate:E,content:{type:"text",text:C?'),
    ('const require = _createRequire(import.meta.url);',
     'import * as shijuMcp from "./session-mcp.mjs";\nconst require = _createRequire(import.meta.url);'),
    ('async newSession(e){let r=crypto.randomUUID(),i=this.createStartupSessionOverrides();',
     'async newSession(e){let selected=shijuMcp.validateSkills(e._meta),mounts=shijuMcp.validate(e.mcpServers);let r=crypto.randomUUID(),i=this.createStartupSessionOverrides();'),
    ('this.sessionStates.set(r,h),{sessionId:r,modes:this.sessionConfig',
     'this.sessionStates.set(r,h),shijuMcp.save(u,mounts),shijuMcp.saveSkills(u,selected),{sessionId:r,modes:this.sessionConfig'),
    ('async prompt(e){let r=this.sessions.get(e.sessionId),i=this.sessionStates.get(e.sessionId);',
     'async prompt(e){let session=this.sessions.get(e.sessionId);return shijuMcp.prompt(session,()=>shijuMcp.withSkills(session,()=>this.promptNative(e)),(call,options)=>this.requestPermission(e.sessionId,call,options),update=>this.emitSessionUpdate(e.sessionId,update))}async promptNative(e){let r=this.sessions.get(e.sessionId),i=this.sessionStates.get(e.sessionId);'),
    ('async cancel(e){let r=this.sessions.get(e.sessionId);',
     'async cancel(e){let r=this.sessions.get(e.sessionId);shijuMcp.stop(r);'),
    ('async loadSession(e){let r=e.sessionId,i=this.sessions.get(r);',
     'async loadSession(e){shijuMcp.rejectSkillRestore(this.sessions.get(e.sessionId),e._meta);if(e.mcpServers?.length||shijuMcp.hasMounts(this.sessions.get(e.sessionId)))throw new Error("Dirac MCP prototype supports only new sessions");let r=e.sessionId,i=this.sessions.get(r);'),
    ('async unstable_forkSession(e){let r=e.sessionId,i=crypto.randomUUID()',
     'async unstable_forkSession(e){shijuMcp.rejectSkillRestore(this.sessions.get(e.sessionId),e._meta);if(e.mcpServers?.length)throw new Error("Dirac MCP prototype supports only new sessions");let r=e.sessionId,i=crypto.randomUUID()'),
    ('async releaseSessionResources(e,r=!1){let i=this.sessions.get(e);',
     'async releaseSessionResources(e,r=!1){let i=this.sessions.get(e);shijuMcp.stop(i);'),
    ('async requestPermission(e,r,i){if(!this.permissionHandler)',
     'async requestPermission(e,r,i){if(shijuMcp.hasMounts(this.sessions.get(e))&&this.pendingPermissionResolvers.has(e))return{outcome:{outcome:"cancelled"}};if(!this.permissionHandler)'),
    ('e.skills?.length??0,e.preferredLanguageInstructions',
     'e.skills?.length??0,shijuMcp.skillPromptCacheKey(),e.preferredLanguageInstructions'),
    ('return e.push(...Fri),e}function pIp(t)',
     'return e.push(...Fri),await shijuMcp.appendSkills(t,e,lIp)}function pIp(t)'),
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
output.chmod(0o755)
