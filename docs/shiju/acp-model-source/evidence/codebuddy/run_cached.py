"""Run the unchanged acceptance entry in the just-validated production test binary."""
import json
import os
from pathlib import Path
import sys
sys.path.insert(0, str(Path('scripts').resolve()))
import verify_acp_model_source as v
binary = Path(os.environ['CODEBUDDY_ACCEPTANCE_BINARY']).resolve()
slot = Path(os.environ['CARGO_TARGET_DIR']).resolve()
if binary.parent != slot / 'debug/deps' or not binary.name.startswith('vw_agent-'):
    raise SystemExit('binary must belong to the held Cargo slot')
original = v.execute_cargo
commands = []
def execute(command, *, env, timeout=300):
    actual = [str(binary), command[command.index('--') - 1], *command[command.index('--') + 1:]]
    commands.append(actual)
    return original(actual, env=env, timeout=timeout)
v.execute_cargo = execute
result = v.main()
output = Path(sys.argv[sys.argv.index('--output') + 1])
report = json.loads(output.read_text())
report['evidence_kind'] = 'cached-production-binary'
if commands:
    report['command'] = commands[-1]
    report['binary_exit_code'] = report.pop('cargo_exit_code', None)
output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
raise SystemExit(result)
