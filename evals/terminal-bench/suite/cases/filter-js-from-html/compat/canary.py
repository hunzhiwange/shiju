"""Unscored positive control: the unchanged browser must detect real JavaScript."""
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import time

spec = importlib.util.spec_from_file_location('helper', sys.argv[1])
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)
with tempfile.TemporaryDirectory() as directory:
    page = Path(directory) / 'canary.html'
    page.write_text('<!doctype html><title>Browser control</title><script>alert("vw-control")</script>')
    assert helper.run_test_alert_file(page) is True, 'real JavaScript alert was not detected'
time.sleep(0.2)
live = []
for path in Path('/proc').glob('[0-9]*/status'):
    try:
        fields = dict(line.split(':', 1) for line in path.read_text().splitlines() if ':' in line)
        if 'chrom' in fields.get('Name', '') and not fields['State'].strip().startswith('Z'):
            live.append({'pid': path.parent.name, 'name': fields['Name'].strip()})
    except OSError:
        continue
print(json.dumps({'alert_detected': True, 'live_browser_processes': live}), flush=True)
assert not live, live
print('COMPAT_ACTIVE_JS_CONTROL_PASSED', flush=True)
