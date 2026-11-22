"""Patch the fixed, privately extracted Copilot ACP boot options only."""
import argparse
import hashlib
import json
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('root', type=Path)
parser.add_argument('--linux', action='store_true')
args = parser.parse_args()
root = args.root
contract = json.loads((Path(__file__).with_name('patch.json')).read_text())
path = root / 'cli-main.js'
original = path.read_bytes()
if hashlib.sha256(original).hexdigest() != contract['original']:
    raise SystemExit('Copilot embedded source checksum mismatch')
text = original.decode()
helper = """function shijuCopilotSkillOptions(settings) {
  const root = process.env.SHIJU_COPILOT_SKILL_ROOT;
  if (root === undefined) return {};
  const project = process.env.SHIJU_COPILOT_PROJECT_SKILLS ? settings : undefined;
  const directories = [...(project?.skillDirectories ?? []), ...(root ? [root] : [])];
  return {
    enableConfigDiscovery: false,
    enableSkills: directories.length !== 0,
    skillDirectories: directories,
    disabledSkills: project?.disabledSkills,
    ignoredSkillsLocations: project?.ignoredSkillsLocations ?? [],
    includedBuiltinSkills: [],
    installedPlugins: [],
    pluginDirectories: []
  };
}
"""
anchor = '#!/usr/bin/env node\n'
if not text.startswith(anchor):
    raise SystemExit('Copilot source entry differs')
text = anchor + helper + text[len(anchor):]
anchor = 'homeDirectory:gmt.homedir()}}function U8n'
if text.count(anchor) != 1:
    raise SystemExit('Copilot ACP boot boundary differs')
text = text.replace(anchor, 'homeDirectory:gmt.homedir(),...shijuCopilotSkillOptions(r)}}function U8n')
patched = text.encode()
if hashlib.sha256(patched).hexdigest() != contract['patched']:
    raise SystemExit('Copilot patched source checksum mismatch')
if args.linux:
    # The ACP boot loader reads vendor MCP config before the SDK discovery
    # options take effect. Managed Linux only accepts host session MCP lists.
    anchor = 'async function B8n(e){'
    if text.count(anchor) != 1:
        raise SystemExit('Copilot ACP MCP boot boundary differs')
    guard = ('if(process.env.VW_ACP_LAUNCH_REFERENCE)'
             'return{mcpServers:{},disabledServers:S.githubMcpGetBuiltinServerNames()};')
    text = text.replace(anchor, anchor + guard)
    anchor = 'mcpServers:t?.mcpServers,additionalMcpConfig:e.additionalMcpConfig,'
    if text.count(anchor) != 1:
        raise SystemExit('Copilot ACP MCP config boundary differs')
    text = text.replace(anchor, 'mcpServers:t?.mcpServers,additionalMcpConfig:'
                        'process.env.VW_ACP_LAUNCH_REFERENCE?undefined:e.additionalMcpConfig,')
    patched = text.encode()
    linux = json.loads((Path(__file__).with_name('linux-artifacts.json')).read_text())
    if hashlib.sha256(patched).hexdigest() != linux['files']['embedded/cli-main.js']:
        raise SystemExit('Copilot Linux patched source checksum mismatch')
path.write_bytes(patched)
