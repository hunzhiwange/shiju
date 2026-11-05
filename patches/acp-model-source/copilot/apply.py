"""Patch the fixed, privately extracted Copilot ACP boot options only."""
import hashlib
import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
contract = json.loads((Path(__file__).with_name('patch.json')).read_text())
path = root / 'cli-main.js'
original = path.read_bytes()
if hashlib.sha256(original).hexdigest() != contract['original']:
    raise SystemExit('Copilot embedded source checksum mismatch')
text = original.decode()
helper = ('function shijuCopilotSkillOptions(){const root=process.env.SHIJU_COPILOT_SKILL_ROOT;'
          'if(root===undefined)return{};return{enableConfigDiscovery:false,enableSkills:root!=="",'
          'skillDirectories:root?[root]:[],includedBuiltinSkills:[],installedPlugins:[],pluginDirectories:[],'
          'ignoredSkillsLocations:[]}};\n')
anchor = '#!/usr/bin/env node\n'
if not text.startswith(anchor):
    raise SystemExit('Copilot source entry differs')
text = anchor + helper + text[len(anchor):]
anchor = 'homeDirectory:gmt.homedir()}}function U8n'
if text.count(anchor) != 1:
    raise SystemExit('Copilot ACP boot boundary differs')
text = text.replace(anchor, 'homeDirectory:gmt.homedir(),...shijuCopilotSkillOptions()}}function U8n')
patched = text.encode()
if hashlib.sha256(patched).hexdigest() != contract['patched']:
    raise SystemExit('Copilot patched source checksum mismatch')
path.write_bytes(patched)
