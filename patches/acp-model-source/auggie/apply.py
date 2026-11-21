"""Patch only the reviewed supplier bundle; keep the global installation intact."""
import hashlib
import json
from pathlib import Path
import shutil
import sys

root = Path(sys.argv[1])
assets = Path(__file__).parent
contract = json.loads((assets / 'patch.json').read_text())
entry = root / 'augment.mjs'
if hashlib.sha256(entry.read_bytes()).hexdigest() != contract['original_sha256']:
    raise RuntimeError('Auggie supplier bundle checksum mismatch')
source = entry.read_text()
# The second half contains embedded worker source strings, not the main runtime.
main, workers = source[:10_000_000], source[10_000_000:]
for needle, replacement, expected in [
    ('async chatStream(e,n){', 'async chatStream(e,n){if(process.env.SHIJU_AUGGIE_MODEL_CONFIG)return shijuAuggie.chatStream(n);', 2),
    ('async getModelConfig(){', 'async getModelConfig(){if(process.env.SHIJU_AUGGIE_MODEL_CONFIG)return shijuAuggie.modelConfig(oG(),NL());', 1),
]:
    if main.count(needle) != expected:
        raise RuntimeError('Auggie reviewed patch anchor mismatch')
    main = main.replace(needle, replacement)
# Use the native scanner and keep its native and plugin precedence and disabled modes.
source = main + workers
for needle, replacement in [
    (
        'return dmi(i,o,e,n)}function d2e',
        'let skills=dmi(i,o,e,n),root=process.env.SHIJU_AUGGIE_SKILL_VIEW;'
        'if(root&&process.env.SHIJU_AUGGIE_RESTRICTED==="1"){'
        'let files=G3({directories:[{path:root,source:"workspace"}],'
        'extensions:[".md"],createConfigFile:s=>s}).filter(Dfn);'
        'for(let skill of Wfn({discoveredFiles:files,transform:e}).transformedFiles)'
        'if(!o.has(skill.name))skills.push(skill)}return skills}function d2e',
    ),
    (
        'this.logger.debug(`Received session/new request: ${JSON.stringify(e)}`)',
        'this.logger.debug(process.env.SHIJU_AUGGIE_RESTRICTED==="1"'
        '?"Received restricted session/new request":`Received session/new request: ${JSON.stringify(e)}`)',
    ),
    (
        'o.length>0&&this.logger.info(`MCP servers: ${JSON.stringify(o,null,2)}`)',
        'o.length>0&&process.env.SHIJU_AUGGIE_RESTRICTED!=="1"'
        '&&this.logger.info(`MCP servers: ${JSON.stringify(o,null,2)}`)',
    ),
    (
        'toolCallId:r,title:s.title,rawInput:o,kind:s.kind',
        'toolCallId:r,title:process.env.SHIJU_AUGGIE_RESTRICTED==="1"'
        '&&JSON.parse(process.env.SHIJU_AUGGIE_MCP_TOOLS??"[]").includes(e)'
        '?"mcp__auggie__"+e:s.title,rawInput:o,kind:s.kind',
    ),
    (
        'let fe=Q?void 0:N,{mcpServers:we,warns:Ee}=await F5e(i.mcpServers,this.config.tools.mcpConfig,this.logger,l,H,fe),Se=[...we,...I]',
        'let restricted=process.env.SHIJU_AUGGIE_RESTRICTED==="1",'
        'fe=Q?void 0:N,{mcpServers:we,warns:Ee}=await F5e(restricted?{}:i.mcpServers,'
        'this.config.tools.mcpConfig,this.logger,l,restricted?[]:H,fe),Se=restricted?we:[...we,...I]',
    ),
]:
    if source.count(needle) != 1:
        raise RuntimeError('Auggie restricted runtime anchor mismatch')
    source = source.replace(needle, replacement)
# shebang must stay first.
needle = 'async checkAndRequestIndexingPermission(e,n){'
if source.count(needle) != 1:
    raise RuntimeError('Auggie indexing anchor mismatch')
source = source.replace(needle, needle + 'if(process.env.SHIJU_AUGGIE_MODEL_CONFIG)return "deny";')
newline = source.index('\n') + 1
source = source[:newline] + 'import * as shijuAuggie from "./session-model.mjs";\n' + source[newline:]
entry.write_text(source)
shutil.copy2(assets / 'session-model.mjs', root / 'session-model.mjs')
