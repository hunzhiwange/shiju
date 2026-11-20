"""Patch the fixed Nova bundle only inside a managed installation."""
import hashlib, shutil, sys
from pathlib import Path
root = Path(sys.argv[1]); entry = root / 'dist/cli.js'
if hashlib.sha256(entry.read_bytes()).hexdigest() != 'ee9d26976cda1beda9b86a0f05aea0e04a77665ae89d8cfbe5395a11ca6e9935':
    raise SystemExit('Unreviewed Nova bundle; refusing patch')
source = entry.read_text()
def replace(old, new):
    global source
    if source.count(old) != 1: raise SystemExit('Nova patch context mismatch')
    source = source.replace(old, new)
replace("function r0(){return i6n(_0x4195fe['homedir']());}", "function r0(){return shijuNovaStateDirectory || i6n(_0x4195fe['homedir']());}")
replace("function lD(_0x2a74ae=process['cwd']()){return i6n(_0x2a74ae);}", "function lD(_0x2a74ae=process['cwd']()){return shijuNovaProjectDirectory(_0x2a74ae) || i6n(_0x2a74ae);}")
replace('function yu(_0x1c61ea){', 'function yu(_0x1c61ea){if(shijuNovaBound)return shijuNovaBound.model;')
replace('function Ns(_0x594918){', 'function Ns(_0x594918){if(shijuNovaBound)return shijuNovaModel();')
replace('function Q5r(_0x1b44d9,_0x369ca9){', 'function Q5r(_0x1b44d9,_0x369ca9){if(shijuNovaBound && _0x1b44d9.maxTokens===undefined)_0x1b44d9.maxTokens=Math.min(kh,_0x369ca9.maxOutputTokens);')
replace('function MC(){', 'function MC(){if(shijuNovaBound)return [shijuNovaModel()];')
replace("async['_doInitialize'](){", "async['_doInitialize'](){if(shijuNovaBound){this.currentModel=shijuNovaBound.model;this.apiKey=process.env.SHIJU_NOVA_RELAY_TOKEN;this.providerApiKeys.set('anthropic',this.apiKey);this.providerBaseUrls.set('anthropic',shijuNovaBound.endpoint);this.initialized=true;return true;}")
replace(
    "async function Yhi(_0x2261bf,_0x4df01f,_0x3ed63d,_0x527f17,_0x5861d1){",
    "async function Yhi(agent,method,params,conn,delegate){"
    "return shijuNovaDispatchSkills(conn,method,params,current=>shijuNovaDispatchMcp(conn,method,current,next=>shijuNovaDispatch(agent,method,next,conn,delegate)));}"
    "async function shijuNovaDispatch(_0x2261bf,_0x4df01f,_0x3ed63d,_0x527f17,_0x5861d1){",
)
replace(
    "async['requestHostTool'](_0x2604e4){",
    "async['requestHostTool'](_0x2604e4){"
    "if(shijuNovaHasSelectedMcp(this))return shijuNovaCallMcp(this,_0x2604e4);",
)
# Extend the native invocation catalog after its normal precedence and scope filters.
replace(
    "return _0x2ce5f9===_0x44ec67['all']?_0x44ec67['active']:IDr(_0x2ce5f9['map'](_0x2d187f=>({..._0x2d187f})))['active'];",
    "return shijuNovaAppendSkills(B7,yd()?.sessionId,_0x2ce5f9===_0x44ec67['all']?_0x44ec67['active']:IDr(_0x2ce5f9['map'](_0x2d187f=>({..._0x2d187f})))['active'],osn);",
)
# Mounted content is read inline. Nova's shell substitutions run outside the
# tool approval loop, and its fork context creates a separate execution scope.
replace(
    "async function dwe(_0x5d5232,_0x23679c){",
    "async function dwe(_0x5d5232,_0x23679c){"
    "if(shijuNovaIsMountedSkill(_0x5d5232) && (_0x5d5232.frontmatter.context??'inline')!=='inline')"
    "throw new Error('Mounted Nova skills require inline context');",
)
replace(
    "_0x2265ff=await csn(_0x2265ff,_0x5d5232['frontmatter']['allowed-tools'])",
    "_0x2265ff=shijuNovaIsMountedSkill(_0x5d5232)?_0x2265ff:await csn(_0x2265ff,_0x5d5232['frontmatter']['allowed-tools'])",
)
replace(
    "'mode':'inline','content':_0x10f39b['processedContent']",
    "'mode':'inline',...(shijuNovaIsMountedSkill(_0x448d80)?{directory:_0x448d80.directoryPath}:{}),'content':_0x10f39b['processedContent']",
)
source = source.replace("#!/usr/bin/env node\n", "", 1)
source = "#!/usr/bin/env node\nimport { bound as shijuNovaBound, model as shijuNovaModel, stateDirectory as shijuNovaStateDirectory, projectDirectory as shijuNovaProjectDirectory } from './shiju-session-model.mjs';\n" + source
source = source.replace(
    "#!/usr/bin/env node\n",
    "#!/usr/bin/env node\nimport { dispatchSkills as shijuNovaDispatchSkills, appendSkills as shijuNovaAppendSkills, isMountedSkill as shijuNovaIsMountedSkill, dispatchMcp as shijuNovaDispatchMcp, hasSelectedMcp as shijuNovaHasSelectedMcp, callMcp as shijuNovaCallMcp } from './shiju-session-mcp.mjs';\n",
    1,
)
entry.write_text(source)
shutil.copy2(Path(__file__).with_name('session-model.mjs'), root / 'dist/shiju-session-model.mjs')

shutil.copy2(Path(__file__).with_name('session-mcp.mjs'), root / 'dist/shiju-session-mcp.mjs')
