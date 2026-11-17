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
source = source.replace("#!/usr/bin/env node\n", "", 1)
source = "#!/usr/bin/env node\nimport { bound as shijuNovaBound, model as shijuNovaModel, stateDirectory as shijuNovaStateDirectory, projectDirectory as shijuNovaProjectDirectory } from './shiju-session-model.mjs';\n" + source
entry.write_text(source)
shutil.copy2(Path(__file__).with_name('session-model.mjs'), root / 'dist/shiju-session-model.mjs')
