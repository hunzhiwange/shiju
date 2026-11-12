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
# shebang must stay first.
source = main + workers
needle = 'async checkAndRequestIndexingPermission(e,n){'
if source.count(needle) != 1:
    raise RuntimeError('Auggie indexing anchor mismatch')
source = source.replace(needle, needle + 'if(process.env.SHIJU_AUGGIE_MODEL_CONFIG)return "deny";')
newline = source.index('\n') + 1
source = source[:newline] + 'import * as shijuAuggie from "./session-model.mjs";\n' + source[newline:]
entry.write_text(source)
shutil.copy2(assets / 'session-model.mjs', root / 'session-model.mjs')
