"""Patch the fixed extracted Kiro V3 scanner, preserving native discovery and tools."""
import hashlib
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
contract = json.loads(Path(__file__).with_name('patch.json').read_text())
raw = path.read_bytes()
if hashlib.sha256(raw).hexdigest() != contract['original']:
    raise SystemExit('Kiro embedded source checksum mismatch')
text = raw.decode()
helper = ('function shijuKiroSkills(rows){const root=process.env.SHIJU_KIRO_SKILL_ROOT;'
          'if(root===undefined)return rows;return rows.filter(row=>{const item=row.document??row;'
          'return root!==""&&typeof item.id==="string"&&item.id.startsWith(root+"/")})}\n')
anchor = "#!/usr/bin/env node\n"
if not text.startswith(anchor):
    raise SystemExit("Kiro source entry differs")
text = anchor + helper + text[len(anchor):]
changes = {
    'createTerminal(e){return this.connection.sendRequest(rs.terminal_create,e,n=>zQt.create(n.terminalId,e.sessionId,this.connection))}':
        'createTerminal(e){if(process.env.SHIJU_KIRO_SKILL_ROOT!==undefined&&(!e.args||e.args.length===0)){'
        'const launch=QHe({command:e.command,cwd:e.cwd??"",baseEnv:{...process.env,...SMo(e.env)},platform:process.platform});'
        'e={...e,command:launch.command,args:launch.args,env:Y2t(launch.env)}}'
        'return this.connection.sendRequest(rs.terminal_create,e,n=>zQt.create(n.terminalId,e.sessionId,this.connection))}',
    'declaredProfileSkills(e,n,r,i){let o=Hh(e)?void 0:n.get(e);':
        'declaredProfileSkills(e,n,r,i){const root=process.env.SHIJU_KIRO_SKILL_ROOT;'
        'if(root!==undefined)return{agent:i.place,registeredItems:root?ggs([{path:root+"/*/SKILL.md"}],r[0],this.homeDir).map(skill=>({...skill,resource:Ya({resourceType:"skill",origin:{origin:"client"}})})):[]};'
        'let o=Hh(e)?void 0:n.get(e);',
    'async scanAll(e){let n=[];for(let o of this.cloudConfigBases())':
        'async scanAll(e){const root=process.env.SHIJU_KIRO_SKILL_ROOT;if(root!==undefined){'
        'if(root&&!DD.isAbsolute(root))throw new Error("Kiro skill root must be absolute");'
        'const byRoot=new Map;for(const workspace of e)byRoot.set(workspace,{skills:[],steering:await this.scanSteeringDirectory(DD.join(Kb(workspace),TT),workspace,{origin:"workspace",root:workspace})});'
        'return{global:[],byRoot}}'
        'let n=[];for(let o of this.cloudConfigBases())',
    'sharedSkills:i.skills(this.shared),rootSkills:i.skills(e.roots)':
        'sharedSkills:i.skills(shijuKiroSkills(this.shared)),rootSkills:i.skills(shijuKiroSkills(e.roots))',
    'helpDocs:e.helpDocs,declared:i.skills(e.declared)':
        'helpDocs:process.env.SHIJU_KIRO_SKILL_ROOT===undefined?e.helpDocs:[],declared:i.skills(shijuKiroSkills(e.declared))',
}
for before, after in changes.items():
    if text.count(before) != 1:
        raise SystemExit('Kiro scanner boundary differs')
    text = text.replace(before, after)
patched = text.encode()
if hashlib.sha256(patched).hexdigest() != contract['patched']:
    raise SystemExit('Kiro patched source checksum mismatch')
path.write_bytes(patched)
