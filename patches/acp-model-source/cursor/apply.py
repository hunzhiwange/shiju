"""Patch only the fixed Cursor supplier archive in an independent managed installation."""
import hashlib
import json
from pathlib import Path
import shutil
import sys

root = Path(sys.argv[1])
assets = Path(__file__).parent
linux = len(sys.argv) == 3 and sys.argv[2] in ('x86_64', 'aarch64')
if linux:
    manifest = json.loads((assets / 'linux-artifacts.json').read_text())
    contract = manifest['targets'][sys.argv[2]]
elif len(sys.argv) == 2:
    contract = json.loads((assets / 'patch.json').read_text())
else:
    raise RuntimeError('Unsupported Cursor patch target')
for name, digest in contract['original'].items():
    if hashlib.sha256((root / name).read_bytes()).hexdigest() != digest:
        raise RuntimeError('Cursor supplier checksum mismatch: ' + name)

def replace(source, needle, replacement):
    if source.count(needle) != 1:
        raise RuntimeError('Cursor reviewed patch anchor mismatch: ' + needle[:70])
    return source.replace(needle, replacement)

entry = root / 'index.js'
source = entry.read_text()
anchor = 'async run(t,e,r,s,i,c,u,l,d,m,p){const f='
source = replace(source, anchor,
    'async run(t,e,r,s,i,c,u,l,d,m,p){if(process.env.SHIJU_CURSOR_MODEL_CONFIG)return shijuCursor.run(shijuCursorApi,t,e,r,s,i,c,u,l,d,m,p);const f=')
agent_class, updates, exec_pb = ('$e', '_', 'v') if linux else ('Me', 'b', 'C')
client_anchor = f'class {agent_class}{{constructor(t,e,r={{}}){{this.client=t'
source = replace(source, client_anchor,
    f'const shijuCursorApi={{pb:g,context:h,updates:{updates},exec:o.n6O,ExecServerMessage:{exec_pb}.Ye,parseShell:r("../shell-exec/dist/api/shell-parser.js").UD}};' + client_anchor)
# Disable backend bootstrap/telemetry, using Cursor's existing authless startup path.
bootstrap = 'O' if linux else 'q'
bootstrap_anchor = f'void 0!==({bootstrap}={{localAgentRuntime:e,argv:i,env:process.env}}).localAgentRuntime&&'
source = replace(source, 'o=' + bootstrap_anchor, 'o=!!process.env.SHIJU_CURSOR_MODEL_CONFIG||' + bootstrap_anchor)
source = 'const shijuCursor=require("./session-model.cjs");\n' + source
entry.write_text(source)

entry = root / ('6136.index.js' if linux else '4119.index.js')
source = entry.read_text()
source = replace(source, 'const{isAuthenticated:M}=await(0,r.D)(a.credentialManager,{authToken:t.authToken})',
    'const{isAuthenticated:M}=process.env.SHIJU_CURSOR_MODEL_CONFIG?{isAuthenticated:!!shijuCursor.config()}:await(0,r.D)(a.credentialManager,{authToken:t.authToken})')
source = replace(source, 'P=await(0,d.A)(a.credentialManager,d.h.LOGIN)',
    'P=process.env.SHIJU_CURSOR_MODEL_CONFIG?false:await(0,d.A)(a.credentialManager,d.h.LOGIN)')
source = replace(source, 'initSharedServices:async()=>{if(x)return x;',
    'initSharedServices:async()=>{if(x)return x;if(process.env.SHIJU_CURSOR_MODEL_CONFIG)return x=await(0,w.Q)(e,t,a,y,{getAcpClientMetadata:()=>A});')
services_start = source.index('async function M(e,t,n,M,T){')
services_end = source.index('}(0,n.h)("shared-services")', services_start) + 1
native_services = source[services_start:services_end]
start = source.index('const _=await(0,S.Qy)(n.configProvider),R=')
end = source.index(',D=new c.xK7', start)
# The managed entry requires a private binding; the user's original CLI is untouched.
chunks = ('2240,8096,7923,1297,5504,5493,5164,3198,5557,8952,9494,1212' if linux
          else '2240,8096,7923,982,1989,8752,2699,1253,9966,7244,5738,8170')
replacement = (
    f'await Promise.all([{chunks}].map(id=>s.e(id)));'
    'const _=undefined,U=shijuCursor.unavailableServices(),E=U,L=new(s("../agent-client/dist/index.js").PW)(null,shijuCursor.config().endpoint)')
source = source[:start] + replacement + source[end:]
source = replace(source, 'await(0,c.W33)(e,{getManagedSkills:async()=>(await E.getManagedSkills(new h.X8g({}))).skills??[]});', '')
team_settings, disabled_store = ('Z', 'X') if linux else ('X', 'Z')
start = source.index(f'{team_settings}=new c.waD(')
end = source.index(f',{disabled_store}=new v.P', start)
source = source[:start] + f'{team_settings}=s("./src/shared/resources.ts").cn()' + source[end:]
source = replace(source, 'se=await w.P3.init(U,{configProvider:n.configProvider,initialModel:k}),ae=await ee.load(e)',
    'se=shijuCursor.modelManager(w.P3,s("../proto/dist/generated/agent/v1/agent_pb.js"),n.configProvider),ae=new d.i9({})')
source = replace(source, 'ce=(0,g.Np)(j)', 'ce=shijuCursor.unavailableServices()')
# The Agent Store scanner accepts additional roots without changing project rule discovery.
skill_loader, skill_scanner = ('Q', 'B') if linux else ('B', 'z')
source = replace(source, f'D,{skill_loader},void 0,W,void 0,"cli"),K=',
    f'D,{skill_loader},void 0,W,void 0,"cli",void 0,shijuCursor.skillRoots),K=')
services_start = source.index('async function M(e,t,n,M,T){')
services_end = source.index('}(0,n.h)("shared-services")', services_start) + 1
managed_services = source[services_start:services_end].replace('async function M(', 'async function shijuCursorShared(', 1)
managed_services = replace(managed_services, 'return{aiServerClient:U',
    f'await {skill_scanner}.agentStoreSkillsSettled();return{{aiServerClient:U')
native_services = native_services.replace('async function M(e,t,n,M,T){', 'async function M(e,t,n,M,T){if(process.env.SHIJU_CURSOR_MODEL_CONFIG)return shijuCursorShared(e,t,n,M,T);', 1)
source = source[:services_start] + native_services + managed_services + source[services_end:]
# Bind the managed shell default to the working directory of the ACP session.
source = replace(source, 'async initializeSessionState(e){this.sessions.set',
    'async initializeSessionState(e){if(process.env.SHIJU_CURSOR_MODEL_CONFIG)e.agentStore.setMetadata("workspacePath",e.cwd);this.sessions.set')
# Upstream ACP normally turns inference failures into successful text responses.
source = replace(source, 'catch(e){if(this.lastRequestId=M.d.getLastParentAgentRequestId(),e instanceof r.cc||t.canceled)return;',
    'catch(e){if(this.lastRequestId=M.d.getLastParentAgentRequestId(),e instanceof r.cc||t.canceled)return;if(process.env.SHIJU_CURSOR_MODEL_CONFIG)throw new Error("Cursor session model request failed",{cause:e});')
# ACP normally reports only a success flag for MCP, even when the server returned an error.
source = replace(source, 'case"mcpToolCall":{const e=t.value.result?.result;return"success"===e?.case?{success:!0}',
    'case"mcpToolCall":{if(process.env.SHIJU_CURSOR_MODEL_CONFIG)return t.value.result?.toJson();const e=t.value.result?.result;return"success"===e?.case?{success:!0}')
for tool_call, call_id in [('e', 't'), ('t', 'a')]:
    anchor = f'toolCallId:{call_id},status:"completed",content:this.toolCalls.extractToolCallContent({tool_call})'
    source = replace(source, anchor,
        f'toolCallId:{call_id},status:shijuCursor.toolCallStatus({tool_call}),content:this.toolCalls.extractToolCallContent({tool_call})')
entry.write_text('const shijuCursor=require("./session-model.cjs");\n' + source)
shutil.copy2(assets / 'session-model.cjs', root / 'session-model.cjs')
