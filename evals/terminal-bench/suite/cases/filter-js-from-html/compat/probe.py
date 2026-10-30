"""Bounded, synthetic browser lifecycle/error probe; never a score or an oracle.

Run under the verifier's sh/uv chain with the patched helper at argv[1].
Fault injection is limited to this separate probe, never the calibration path.
"""
import importlib.util
import json
from pathlib import Path
import signal
import sys
import tempfile
import time
from unittest.mock import patch


def processes():
    rows = []
    for directory in Path('/proc').glob('[0-9]*'):
        try:
            status = (directory / 'status').read_text()
            fields = dict(line.split(':', 1) for line in status.splitlines() if ':' in line)
            name = fields['Name'].strip()
            if 'chrom' in name:
                rows.append({'pid': int(directory.name), 'name': name,
                             'state': fields['State'].strip(), 'ppid': int(fields['PPid'])})
        except (OSError, KeyError, ValueError):
            continue
    return rows


def record(name):
    time.sleep(0.2)
    rows = processes()
    live = [row for row in rows if not row['state'].startswith('Z')]
    print(json.dumps({'probe': name, 'processes': rows, 'live_browser_processes': len(live),
                      'memory_current': Path('/sys/fs/cgroup/memory.current').read_text().strip(),
                      'memory_events': Path('/sys/fs/cgroup/memory.events').read_text()}), flush=True)
    assert not live, live


spec = importlib.util.spec_from_file_location('helper', sys.argv[1])
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)
with tempfile.TemporaryDirectory() as directory:
    html = Path(directory) / 'blank.html'
    html.write_text('<!doctype html><title>Lifecycle probe</title><p>plain text</p>')
    for n in range(2):
        assert helper.run_test_alert_file(html) is False
        record('normal-' + str(n))

    real_options = helper.Options
    def missing_binary():
        options = real_options()
        options.binary_location = '/nonexistent/vw-probe-browser'
        return options
    with patch.object(helper, 'Options', missing_binary):
        try:
            helper.run_test_alert_file(html)
        except Exception as error:
            print(json.dumps({'probe': 'startup', 'propagated': type(error).__name__}), flush=True)
        else:
            raise AssertionError('startup failure was swallowed')
    record('startup')

    factory = helper.webdriver.Chrome
    for mode in ('navigation', 'alarm'):
        class FaultDriver:
            def __init__(self, **kwargs):
                self.real = factory(**kwargs)
            def get(self, url):
                if mode == 'alarm':
                    signal.alarm(1)
                    time.sleep(2)
                raise OSError('synthetic navigation failure')
            def quit(self):
                self.real.quit()
        with patch.object(helper.webdriver, 'Chrome', FaultDriver):
            try:
                helper.run_test_alert_file(html)
            except (OSError, TimeoutError) as error:
                print(json.dumps({'probe': mode, 'propagated': type(error).__name__}), flush=True)
            else:
                raise AssertionError(mode + ' failure was swallowed')
        record(mode)
print('COMPAT_LIFECYCLE_PROBE_PASSED', flush=True)
