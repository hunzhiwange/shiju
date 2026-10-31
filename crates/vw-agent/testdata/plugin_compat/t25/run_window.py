"""MIT. Run actual native forms and guarantee teardown before releasing a Cargo slot."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time

root = Path(__file__).resolve().parents[5]
build = Path(os.environ['CARGO_BUILD_BUILD_DIR']) / 'debug/deps'
def binary(prefix):
    candidates = sorted((p for p in build.glob(prefix + '-*') if p.is_file() and os.access(p, os.X_OK) and '.' not in p.name), key=lambda p: p.stat().st_mtime, reverse=True)
    assert candidates, (prefix, build)
    return candidates[0]
backend = binary('vw_agent')
window = binary('mcp_form_window')
name = 'gateway::api::handlers::mcp_forms::tests::codex_plugin_compat_t25_window_gateway_fixture'
listed = subprocess.run([str(backend), '--list'], capture_output=True, text=True, check=True).stdout
assert name in listed
out = root / 'docs/plugin-compat/evidence/t25' / sys.argv[1]
out.mkdir()
env = os.environ.copy()
env['VIBEWINDOW_CONFIG_DIR'] = str(out / 'config')
env['VW_MCP_FORM_WINDOW_ROOT'] = str(out)
server = subprocess.Popen([str(backend), name, '--exact', '--ignored', '--nocapture', '--test-threads=1'], cwd=root, env=env)
try:
    deadline = time.monotonic() + 30
    while not (out / 'ready.json').exists():
        assert server.poll() is None, 'fixture exited before readiness'
        assert time.monotonic() < deadline, 'fixture readiness timeout'
        time.sleep(0.02)
    port = json.loads((out / 'ready.json').read_text())['port']
    result = subprocess.run([str(window), str(port), str(out)], cwd=root, env=env)
    if result.returncode:
        sys.exit(result.returncode)
    values = json.loads((out / 'window-results.json').read_text())
    assert len(values) == 6
    print(json.dumps({'native_scenarios': len(values), 'output': str(out)}, ensure_ascii=False), flush=True)
finally:
    (out / 'stop').write_text('stop')
    try:
        server.wait(timeout=5)
    except subprocess.TimeoutExpired:
        server.terminate()
        try:
            server.wait(timeout=5)
        except subprocess.TimeoutExpired:
            server.kill()
            server.wait()
