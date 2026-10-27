"""Private, per-attempt native execution gates; no global plugin trust changes."""
import hashlib
import json
from pathlib import Path, PurePosixPath

PLUGIN = 'suite-budget'
EVENTS = {'before_llm_call': 'model', 'before_tool_call': 'tool'}
REQUIRED = [f'plugin:{PLUGIN}:{event}' for event in EVENTS]


def bundle(canonical_remote, attempt):
    """Return files and public config for an already canonicalized remote root.

    Counters are initialized only during fresh install, never during resume.
    The lock inode survives atomic state replacement; missing/corrupt state denies.
    """
    root = PurePosixPath(canonical_remote)
    if not root.is_absolute() or '..' in root.parts or not attempt.isalnum():
        raise ValueError('invalid isolated budget identity')
    plugin = root / 'budget/plugins' / PLUGIN
    state = root / 'budget/state'
    files = {'budget/state': f'attempt {attempt}\nmodels 0\ntools 0\n', 'budget/state.lock': ''}
    hooks, trust = [], []
    for event, kind in EVENTS.items():
        hook = {'event': event, 'command': str(root / 'runtime/vw-budget-gate'),
                'args': [str(state), kind, attempt], 'timeout_secs': 5, 'working_dir': '.'}
        hooks.append(hook)
        identity = {'plugin_name': PLUGIN, 'event': event, 'command': hook['command'],
                    'args': hook['args'], 'working_dir': str(plugin),
                    'manifest_path': str(plugin / 'hooks/hooks.json')}
        # Mirror the pinned Rust json! insertion order (preserve_order feature).
        trust.append(hashlib.sha256(json.dumps(identity, sort_keys=False,
                           separators=(',', ':'), ensure_ascii=False).encode()).hexdigest())
    files['budget/plugins/suite-budget/.vibewindow-plugin/plugin.json'] = json.dumps({
        'name': PLUGIN, 'version': '1.0.0', 'description': 'Frozen isolated attempt budget',
        'hooks': './hooks/hooks.json'})
    files['budget/plugins/suite-budget/hooks/hooks.json'] = json.dumps({'hooks': hooks})
    files['config/plugin-hook-trust.json'] = json.dumps({'version': 1, 'trusted_hashes': sorted(trust)})
    config = {'plugins': {'enabled': [PLUGIN], 'sources': [str(plugin.parent)]},
              'hooks': {'enabled': True, 'plugins': {'enabled': True}, 'required': REQUIRED},
              'reliability': {'provider_retries': 0}}
    return files, config


def identity():
    paths = [Path(__file__), Path(__file__).with_name('budget_gate.rs')]
    return {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in paths}
