#!/usr/bin/env python3
"""Apply the reviewed model-source patch to an isolated npm iFlow 0.5.19 install."""
import hashlib
from pathlib import Path
import sys

package = Path(sys.argv[1])
paths = [package / 'bundle/iflow.js', package / 'bundle/entry.js']
expected = ['3bc5c8873d4e5691322de7fac1b5cd530c2f797107c3c9beb11afd8e99cedddc',
            '81de70778e7276b6410ae91a84e1481f88e4087984b95f882873e8e3024bc2c1']
texts = []
for path, digest in zip(paths, expected):
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != digest:
        raise SystemExit('Unrecognized iFlow bundle; refusing patch')
    texts.append(data.decode())
s, entry = texts

def replace(old, new, count=1):
    global s
    if s.count(old) != count:
        raise SystemExit('iFlow patch context count differs')
    s = s.replace(old, new)

# Reuse the native parser and Skill tool, with a request-owned scanner root.
# Presence with an empty value disables all levels; undefined preserves CLI behavior.
replace('async scan(){let e=new Map;',
        'async scan(){if(Object.prototype.hasOwnProperty.call(process.env,"SHIJU_IFLOW_SKILL_ROOT")){let root=process.env.SHIJU_IFLOW_SKILL_ROOT;return root?this.scanDirectory(root,!1):[]}let e=new Map;')
replace('async ensureDirectories(){try{await L4e.mkdir',
        'async ensureDirectories(){if(Object.prototype.hasOwnProperty.call(process.env,"SHIJU_IFLOW_SKILL_ROOT"))return;try{await L4e.mkdir')
replace('await this.skillConfigCache.getSkill(e.skill)||KU.get(e.skill)',
        'await this.skillConfigCache.getSkill(e.skill)||(process.env.SHIJU_IFLOW_SKILL_ROOT===undefined?KU.get(e.skill):undefined)', 2)
replace('for(let o of KU.getAll())n.has(o.name)||r.push(o);',
        'if(process.env.SHIJU_IFLOW_SKILL_ROOT===undefined)for(let o of KU.getAll())n.has(o.name)||r.push(o);')

replace('r(U4e,this),', 'process.env.SHIJU_IFLOW_SKILL_ROOT!==""&&r(U4e,this),')

# IFLOW_HOME already controls Storage, but upstream settings/plugins/hooks ignore it.
replace('Ght=y5.join($ht(),SI)', 'Ght=Tn()')
replace('function NMi(){return process.env.IFLOW_CLI_SYSTEM_SETTINGS_PATH?', 'function NMi(){if(process.env.IFLOW_HOME&&!process.env.IFLOW_CLI_SYSTEM_SETTINGS_PATH)return y5.join(Tn(),"system.json");return process.env.IFLOW_CLI_SYSTEM_SETTINGS_PATH?')
for namespace, host, count in [('NS', 'Vfr', 2), ('BS', 'ume', 3), ('cu', 'nw', 6)]:
    replace(f'{namespace}.join({host}.homedir(),".iflow",', f'{namespace}.join(Tn(),', count)
replace('getDefaultUserSettingsPath(){let e=process.env.HOME||process.env.USERPROFILE||"";return Ufr.join(e,ui(),"settings.json")}', 'getDefaultUserSettingsPath(){return Ufr.join(Tn(),"settings.json")}')
replace('...n$i(i$i.homedir())', '...n$i(Tn(),true)')
replace('function n$i(t){let e=C1e.join(t,iou);', 'function n$i(t,shijuRoot=false){let e=shijuRoot?C1e.join(t,"extensions"):C1e.join(t,iou);')
# Persist only credential-free ACP chat history, independently of rotating route settings.
replace('getSessionsDir(){return spe.join(Tn(),"acp","sessions")}',
        'getSessionsDir(){return process.env.SHIJU_IFLOW_HISTORY_HOME||spe.join(Tn(),"acp","sessions")}')
replace('getSessionPath(e){return spe.join(this.getSessionsDir(),`${e}.json`)}',
        'getSessionPath(e){if(!/^[a-zA-Z0-9_-]{1,128}$/.test(e))throw new Error("Invalid iFlow session anchor");return spe.join(this.getSessionsDir(),`${e}.json`)}')
replace('await Qy.writeFile(r,JSON.stringify(e,null,2))',
        'await Qy.writeFile(r,(()=>{let data=JSON.stringify(e,null,2);if(process.env.SHIJU_IFLOW_MODEL_LOCK&&data.includes(process.env.IFLOW_apiKey))throw new Error("Credential in iFlow history");return data})(),{mode:384})')
# Managed authentication never attempts native OAuth or accepts ACP methodInfo overrides.
replace('authMethods:r,agentCapabilities:', 'authMethods:process.env.SHIJU_IFLOW_MODEL_LOCK?r.filter(method=>method.id===Kt.OPENAI_COMPATIBLE):r,agentCapabilities:')
replace('async authenticate(e){let r=e.methodId,',
        'async authenticate(e){if(process.env.SHIJU_IFLOW_MODEL_LOCK){if(e.methodId!=="openai-compatible"||e.methodInfo)throw new Error("Managed iFlow authentication is fixed");}let r=e.methodId,')
replace('if(await BOe(r,this.config)){', 'if(!process.env.SHIJU_IFLOW_MODEL_LOCK&&await BOe(r,this.config)){')
# Enforce authentication for child/config refresh paths as well as ACP authenticate.
replace('async function KOt(t,e,r){',
        'async function KOt(t,e,r){if(process.env.SHIJU_IFLOW_MODEL_LOCK){if(e!=="openai-compatible")throw new Error("Managed iFlow requires OpenAI authentication");r={apiKey:process.env.IFLOW_apiKey,baseUrl:process.env.IFLOW_baseUrl,modelName:process.env.SHIJU_IFLOW_MODEL_LOCK};}')
replace('constructor(e){this.config=e.config,this.apiKey=e.apiKey||',
        'constructor(e){if(process.env.SHIJU_IFLOW_MODEL_LOCK)e.multimodalModelName=process.env.SHIJU_IFLOW_MODEL_LOCK;this.config=e.config,this.apiKey=e.apiKey||')
# Apply the same model to every primary, compression and subagent Chat request.
replace('async generateContentInternal(e,r,n,o,s=!1){let a=this.config?.getModel()||"unknown",',
        'async generateContentInternal(e,r,n,o,s=!1){let a=process.env.SHIJU_IFLOW_MODEL_LOCK||this.config?.getModel()||"unknown",')
# A new flag makes unpatched binaries fail argument parsing rather than silently launch.
marker = '''
const shijuFlag = process.argv.indexOf('--shiju-model-source');
if (shijuFlag !== -1) {
  if (process.argv.slice(2).join(' ') !== '--experimental-acp --shiju-model-source' ||
      !process.env.IFLOW_HOME || !process.env.IFLOW_apiKey ||
      !process.env.SHIJU_IFLOW_HISTORY_HOME ||
      !process.env.SHIJU_IFLOW_MODEL_LOCK ||
      process.env.SHIJU_IFLOW_MODEL_LOCK !== process.env.IFLOW_modelName ||
      process.env.IFLOW_selectedAuthType !== 'openai-compatible') {
    throw new Error('Invalid managed iFlow launch');
  }
  process.argv.splice(shijuFlag, 1);
} else if (process.env.SHIJU_IFLOW_MODEL_LOCK) {
  throw new Error('Managed iFlow requires explicit launch flag');
}
'''
# Place after the shebang, before entry's dynamic engine import.
position = entry.find('\n') + 1 if entry.startswith('#!') else 0
entry = entry[:position] + marker + entry[position:]
# Validate every context before mutating either file. Originals remain in the npm tarball.
for path, content in zip(paths, [s, entry]):
    path.write_text(content)
print('Patched isolated iFlow 0.5.19 bundles')
