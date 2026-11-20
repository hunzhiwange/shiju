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
changes = {
    'createTerminal(e){return this.connection.sendRequest(rs.terminal_create,e,n=>zQt.create(n.terminalId,e.sessionId,this.connection))}':
        'createTerminal(e){if(process.env.SHIJU_KIRO_SKILL_ROOT!==undefined&&(!e.args||e.args.length===0)){'
        'const launch=QHe({command:e.command,cwd:e.cwd??"",baseEnv:{...process.env,...SMo(e.env)},platform:process.platform});'
        'e={...e,command:launch.command,args:launch.args,env:Y2t(launch.env)}}'
        'return this.connection.sendRequest(rs.terminal_create,e,n=>zQt.create(n.terminalId,e.sessionId,this.connection))}',
    # Retain the native profile's placement and append the host's separately
    # placed session entries through the original resource parser.
    'declaredProfileSkills(e,n,r,i){let o=Hh(e)?void 0:n.get(e);return o===void 0?OHt:DHt(o,n,r,this.homeDir,this.configuration.sharedItems,i.place)}':
        'declaredProfileSkills(e,n,r,i){let o=Hh(e)?void 0:n.get(e);'
        'const native=o===void 0?OHt:DHt(o,n,r,this.homeDir,this.configuration.sharedItems,i.place);'
        'const root=process.env.SHIJU_KIRO_SKILL_ROOT;if(!root)return native;'
        'if(!DD.isAbsolute(root))throw new Error("Kiro skill root must be absolute");'
        'return{...native,registeredItems:[...native.registeredItems,'
        '...ggs([{path:root+"/*/SKILL.md"}],r[0],this.homeDir).map(skill=>({...skill,'
        'shijuSkillPlace:i.place,resource:Ya({resourceType:"skill",origin:{origin:"client"}})}))]}}',
    # A native declaration wins if both placements produce the same KRN.
    'declared(e,n){if(n===void 0)return[];let r=new Map;for(let i of e){let o=this.items.declared(i,n);r.set(o.krn,{document:i,placed:o})}':
        'declared(e,n){let r=new Map;for(let i of e){const place=i.shijuSkillPlace??n;'
        'if(place===void 0)continue;let o=this.items.declared(i,place);'
        'if(!i.shijuSkillPlace||!r.has(o.krn))r.set(o.krn,{document:i,placed:o})}',
    # Keep native discovery and activation precedence, including names of
    # native items the configuration has disabled. The selected view cannot
    # override a native entry even when the native catalog qualifies names.
    'helpDocs:e.helpDocs,declared:i.skills(e.declared)':
        'helpDocs:e.helpDocs,declared:i.skills(e.declared).filter(({document})=>!document.shijuSkillPlace||'
        '![...this.shared,...e.roots,...e.steering,...e.declared].some(({document:native})=>'
        '!native.shijuSkillPlace&&Yne(native.displayName)===Yne(document.displayName))&&'
        '!e.helpDocs.some(native=>Yne(native.displayName)===Yne(document.displayName)))',
}
for before, after in changes.items():
    if text.count(before) != 1:
        raise SystemExit('Kiro scanner boundary differs')
    text = text.replace(before, after)
patched = text.encode()
if hashlib.sha256(patched).hexdigest() != contract['patched']:
    raise SystemExit('Kiro patched source checksum mismatch')
path.write_bytes(patched)
