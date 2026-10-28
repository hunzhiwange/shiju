"""Calibrate exact task bytes before freezing all 89 parent/diagnostic groups."""
import asyncio
import hashlib
from importlib.metadata import distribution, version
import json
from pathlib import Path
import subprocess
import sys
import tomllib
import uuid
from urllib.parse import unquote, urlparse

from suite import diagnostics, variants
from suite.selection import selected_parents, merge_execution_configs
from suite.runtime import verify_runtime
from suite.scoring import harbor_trial, summarize, checked_startup_override
from suite.state import MODEL, load, digest, atomic_json, tree, execution_identity

CATALOG = Path(__file__).with_name('catalog.json')

def checked_runtime(path):
    value = load(path)
    for name in ('historical_root', 'source_root', 'cases_root'):
        if not Path(value[name]).is_absolute():
            raise ValueError('runtime paths must be absolute')
    identity = verify_runtime(value['historical_root'])
    if identity.get('generation') != 2:
        raise ValueError('suite scheduling requires the calibrated budget runtime generation')
    return value, identity


def harbor_identity():
    import harbor
    if version('harbor') != '0.24.0':
        raise ValueError('Harbor version drift')
    direct = json.loads(distribution('harbor').read_text('direct_url.json'))
    source = Path(unquote(urlparse(direct['url']).path))
    revision = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD']).decode().strip()
    if revision != '6099b8cf2bfbbb3e7ff1eb2163bfc4524dbd9ba5':
        raise ValueError('Harbor source revision drift')
    installed = Path(harbor.__file__).parent
    subprocess.run(['git', '-C', str(source), 'diff', '--quiet', 'HEAD', '--', 'src/harbor'], check=True)
    names = subprocess.check_output(['git', '-C', str(source), 'ls-tree', '-r', '--name-only',
                                     'HEAD', '--', 'src/harbor']).decode().splitlines()
    files = {str(Path(name).relative_to('src/harbor')): digest(source / name)
             for name in names if name.endswith('.py')}
    actual = {str(p.relative_to(installed)): digest(p) for p in installed.rglob('*.py')}
    if actual != files:
        raise ValueError('installed Harbor differs from pinned source')
    return {'revision': revision, 'version': '0.24.0',
            'sha256': hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()}


def adapter_identity():
    from suite.agent import source_identity
    return source_identity()


def case_settings(runtime, parent):
    settings = {}
    for name in [parent] + [parent + '-m' + str(n) for n in range(1, 5)]:
        matches = [t for t in runtime['tasks'] if t['taskId'] == name]
        if len(matches) > 1 or (name == parent and not matches):
            raise ValueError('runtime task missing or duplicate')
        value = matches[0] if matches else {'workingDirectory': '/app', 'agentTimeoutSec': 900}
        settings[name] = {k: v for k, v in value.items()
                          if k in ('workingDirectory', 'agentTimeoutSec', 'recovery', 'startup_override', 'environment_variant')}
        if not Path(settings[name]['workingDirectory']).is_absolute():
            raise ValueError('working directory must be absolute')
        if 'environment_variant' in settings[name]:
            if name != parent:
                raise ValueError('environment variant applies only to its authorized parent')
            settings[name]['environment_variant'] = variants.resolve(parent, settings[name]['environment_variant'])
        if 'startup_override' in settings[name]:
            checked_startup_override(name, settings[name]['startup_override'])
        recovery = settings[name].get('recovery')
        if recovery and recovery['mode'] == 'artifact':
            settings[name]['recovery'] = {**recovery, 'calibrated': True, 'calibrate': False}
    return settings


def inputs(catalog_path, runtime_path, parent, checked=None):
    if digest(catalog_path) != digest(CATALOG):
        raise ValueError('catalog differs from frozen official catalog')
    catalog = load(catalog_path)
    runtime, identity = checked or checked_runtime(runtime_path)
    matches = [task for task in catalog['tasks'] if task['id'] == parent]
    if len(matches) != 1:
        raise ValueError('unknown parent')
    settings = [task for task in runtime['tasks'] if task['taskId'] == parent]
    if len(settings) != 1 or settings[0]['agentTimeoutSec'] != matches[0]['agent']['timeout_sec']:
        raise ValueError('runtime must retain original agent timeout')
    if not Path(settings[0]['workingDirectory']).is_absolute():
        raise ValueError('working directory must be absolute')
    official = Path(runtime['source_root']) / 'tasks' / parent
    expected = {name: {'sha256': value['sha256'], 'executable': value['mode'] == '100755'}
                for name, value in matches[0]['files'].items()}
    if tree(official) != expected:
        raise ValueError('official task drift')
    case = Path(runtime['cases_root']) / parent / 'case.json'
    diagnostic = diagnostics.validate(case)
    # Ensure validation.json represents the complete calibration, not just syntax.
    calibrated = load(case.with_name('validation.json'))
    definition, _ = diagnostics.load_case(case)
    for child in definition['children']:
        diagnostic['checks'] += [
            {'child_id': child['id'], 'fixture': name, 'exit_code': code}
            for name, code in [('gold', 0), ('nop', 1)] + [(n['id'], 1) for n in child['negatives']]]
    if diagnostic != calibrated:
        raise ValueError('diagnostic calibration missing or stale')
    receipt = {'parent_id': parent, 'catalog_sha256': digest(catalog_path),
                     'case_settings': case_settings(runtime, parent),
                     'runtime_sha256': hashlib.sha256(json.dumps({'settings': case_settings(runtime, parent), 'runtime': identity,
                          'source_root': runtime['source_root'], 'cases_root': runtime['cases_root']},
                          sort_keys=True).encode()).hexdigest(), 'runtime_identity': identity,
                     'adapter': adapter_identity(), 'scheduler_sha256': execution_identity(),
                     'harbor': harbor_identity(),
                     'official': str(official), 'official_files': expected,
                     'diagnostic': str(case), 'diagnostic_receipt': calibrated}
    variant = receipt['case_settings'][parent].get('environment_variant')
    if variant is not None:
        if receipt['case_settings'][parent].get('recovery') or receipt['case_settings'][parent].get('startup_override'):
            raise ValueError('variant cannot combine with verifier recovery or startup overrides')
        receipt['environment_variant'] = variant
        receipt['execution_files'] = variants.execution_files(official, variant)
    return runtime, receipt


def preflight(catalog, runtime, parent, output):
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    runtime_value, receipt = inputs(catalog, runtime, parent)
    diagnostics.materialize(receipt['diagnostic'], output / 'diagnostics')
    parent_task = Path(receipt['official'])
    if 'environment_variant' in receipt:
        parent_task = variants.materialize(parent_task, output / 'variants' / parent, receipt['environment_variant'])
    tasks = [parent_task] + sorted((output / 'diagnostics').iterdir())
    receipt['calibration'] = []
    for task in tasks:
        for agent, expected in (('oracle', 1), ('nop', 0)):
            recovery = receipt['case_settings'][task.name].get('recovery')
            if recovery and recovery['mode'] == 'artifact':
                recovery = {**recovery, 'calibrate': True, 'calibrated': True}
            result = asyncio.run(harbor_trial(task, output / 'calibration',
                                             task.name + '-' + agent, agent, recovery=recovery,
                                             **({'startup_override': receipt['case_settings'][task.name]['startup_override']}
                                                if 'startup_override' in receipt['case_settings'][task.name] else {}),
                                             **({'environment_variant': receipt['environment_variant']}
                                                if task == parent_task and 'environment_variant' in receipt else {})))
            summary = summarize(result)
            scoring = output / 'calibration' / (task.name + '-' + agent) / 'scoring'
            receipt['calibration'].append({'case_id': task.name, 'agent': agent,
                                           'result': result, 'summary': summary,
                                           'first_score': load(scoring / 'first.json') if (scoring / 'first.json').exists() else {},
                                           'recovery_score': load(scoring / 'recovered.json')
                                               if (scoring / 'recovered.json').exists() else None,
                                           'evidence_root': str(scoring), 'evidence_files': tree(scoring)})
            if task == parent_task and 'environment_variant' in receipt:
                evidence = output / 'calibration' / (task.name + '-' + agent) / 'environment-variant.json'
                variants.check_start(load(evidence), receipt['environment_variant'])
                receipt['calibration'][-1]['environment_variant_start'] = {
                    'path': str(evidence.resolve()), 'sha256': digest(evidence)}
            atomic_json(output / 'preparation.json', receipt)
            if summary['score_status'] != 'scored' or summary['reward'] != expected:
                raise ValueError('calibration/infrastructure failure; no model run permitted')
            if 'error_type' in load(scoring / 'first.json'):
                raise ValueError('original scorer calibration failed')
    atomic_json(Path(runtime_value['cases_root']) / parent / 'preparation.json', receipt)
    return receipt


def freeze(catalog, cases_root, runtime_path, output):
    return _freeze(catalog, cases_root, runtime_path, output)


def freeze_ready_subset(catalog, cases_root, runtime_paths, output, selection):
    if selection is None:
        raise ValueError('ready-subset requires explicit selection')
    return _freeze(catalog, cases_root, runtime_paths, output, selection)


def _freeze(catalog, cases_root, runtime_path, output, scope=None):
    catalog_value = load(catalog)
    if catalog_value['task_count'] != 89 or len(catalog_value['tasks']) != 89:
        raise ValueError('freeze requires exactly 89 official parents')
    parents = selected_parents(catalog_value, scope)
    if isinstance(runtime_path, (list, tuple)):
        runtime = merge_execution_configs(runtime_path)
        _, identity = checked_runtime(runtime_path[0])
    else:
        runtime, identity = checked_runtime(runtime_path)
    if Path(cases_root).resolve() != Path(runtime['cases_root']).resolve():
        raise ValueError('case root differs from runtime')
    if scope is None:
        diagnostics.validate_all(catalog, cases_root, verify_receipts=True)
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    tasks = []
    receipts = {}
    prepared_receipts = {}
    for parent in parents:
        _, current = inputs(catalog, runtime_path, parent, (runtime, identity))
        path = Path(cases_root).resolve() / parent / 'preparation.json'
        recovery = next(t for t in runtime['tasks'] if t['taskId'] == parent).get('recovery')
        verify_preparation(path, current, recovery)
        prepared_receipts[parent] = {'path': str(path), 'sha256': digest(path)}
        receipts[parent] = current
        package = output / 'packages' / parent
        diagnostics.materialize(current['diagnostic'], package)
        official_settings = [t for t in runtime['tasks'] if t['taskId'] == parent]
        if len(official_settings) != 1:
            raise ValueError('runtime task missing or duplicate')
        parent_task = Path(current['official'])
        if 'environment_variant' in current:
            parent_task = variants.materialize(parent_task, output / 'variants' / parent, current['environment_variant'])
        paths = [parent_task] + sorted(package.iterdir())
        for index, path in enumerate(paths):
            settings = current['case_settings'][path.name]
            tasks.append({'taskId': path.name, 'parent_id': parent,
                          'kind': ('compatibility_variant' if 'environment_variant' in current else 'official') if index == 0 else 'diagnostic',
                          'path': str(path), 'files': tree(path), **settings})
    manifest = {'schema_version': 1, 'experiment_id': uuid.uuid4().hex, 'model': MODEL,
                'tasks': tasks, 'receipts': receipts, 'preparation_receipts': prepared_receipts,
                'catalog_sha256': digest(catalog), 'runtime_root': runtime['historical_root'],
                'runtime': {'binarySha256': identity['binary_sha256'],
                            'requiredSidecars': identity['sidecars']},
                'adapter': adapter_identity(), 'scheduler_sha256': execution_identity()}
    if scope is not None:
        manifest['scope'] = scope
    manifest['harbor'] = harbor_identity()
    atomic_json(output / 'manifest.json', manifest)
    return manifest


def verify_preparation(path, expected, recovery):
    prepared = load(path)
    calibration = prepared.pop('calibration', [])
    parent = expected['parent_id']
    official = tomllib.loads((Path(expected['official']) / 'task.toml').read_text())
    official_name = ('terminal-bench/' + expected['environment_variant']['id']
                     if 'environment_variant' in expected else official.get('task', {}).get('name') or parent)
    cases = [parent] + [parent + '-m' + str(n) for n in range(1, 5)]
    wanted = [(case, agent, reward) for case in cases for agent, reward in (('oracle', 1), ('nop', 0))]
    if prepared != expected or [(c['case_id'], c['agent'], c['summary']['reward'])
                               for c in calibration] != wanted:
        raise ValueError('preparation receipt drift or incomplete calibration')
    for c in calibration:
        if c['case_id'] == parent and 'environment_variant' in expected:
            evidence = c.get('environment_variant_start', {})
            if (set(evidence) != {'path', 'sha256'} or digest(evidence['path']) != evidence['sha256']
                    or c['result'].get('config', {}).get('environment', {}).get('import_path') != 'suite.docker:SuiteDockerEnvironment'):
                raise ValueError('missing standard pre-agent variant calibration')
            variants.check_start(load(evidence['path']), expected['environment_variant'])
        if (summarize(c['result']) != c['summary'] or c['summary']['score_status'] != 'scored'
                or c['result']['agent_info']['name'] != c['agent']
                or c['result']['task_name'] != (official_name if c['case_id'] == parent else c['case_id'])
                or tree(c['evidence_root']) != c['evidence_files']
                or load(Path(c['evidence_root']) / 'first.json') != c['first_score']
                or (c['recovery_score'] is not None and
                    load(Path(c['evidence_root']) / 'recovered.json') != c['recovery_score'])
                or c['first_score'].get('rewards', {}).get('reward') != c['summary']['reward']
                or (c['recovery_score'] is not None and
                    c['recovery_score'].get('rewards', {}).get('reward') != c['summary']['reward'])):
            raise ValueError('invalid or changed scorer calibration evidence')
        if (expected['case_settings'][c['case_id']].get('recovery', {}).get('mode') == 'artifact'
                and c['recovery_score'] is None):
            raise ValueError('artifact verifier image requires oracle/nop calibration')


def validate_manifest(path):
    manifest = load(path)
    if 'scope' in manifest and manifest['scope'] is None:
        raise ValueError('invalid explicit scope')
    scheduler = execution_identity()
    adapter = adapter_identity()
    if (manifest['schema_version'] != 1 or manifest['catalog_sha256'] != digest(CATALOG)
            or manifest['model'] != MODEL or manifest['adapter'] != adapter
            or manifest['scheduler_sha256'] != scheduler or manifest['harbor'] != harbor_identity()):
        raise ValueError('frozen execution policy drift')
    parents = selected_parents(load(CATALOG), manifest.get('scope'))
    expected_ids = [case for parent in parents for case in
                    [parent] + [parent + '-m' + str(n) for n in range(1, 5)]]
    if ([t['taskId'] for t in manifest['tasks']] != expected_ids
            or set(manifest['receipts']) != set(parents)
            or set(manifest['preparation_receipts']) != set(parents)):
        raise ValueError('frozen manifest requires complete parent groups; full scope requires 89')
    identity = verify_runtime(manifest['runtime_root'])
    if (identity.get('generation') != 2 or manifest['runtime'] !=
            {'binarySha256': identity['binary_sha256'], 'requiredSidecars': identity['sidecars']}):
        raise ValueError('runtime drift')
    for parent in parents:
        receipt = manifest['receipts'][parent]
        if (receipt['runtime_identity'] != identity or receipt['scheduler_sha256'] != scheduler
                or receipt['adapter'] != adapter or receipt['catalog_sha256'] != digest(CATALOG)):
            raise ValueError('preparation identity drift')
        prepared = manifest['preparation_receipts'][parent]
        if digest(prepared['path']) != prepared['sha256']:
            raise ValueError('preparation receipt changed')
        task = next(t for t in manifest['tasks'] if t['taskId'] == parent)
        verify_preparation(prepared['path'], receipt, task.get('recovery'))
    for task in manifest['tasks']:
        receipt = manifest['receipts'][task['parent_id']]
        parent_variant = receipt.get('environment_variant')
        kind = ('compatibility_variant' if parent_variant else 'official') if task['taskId'] == task['parent_id'] else 'diagnostic'
        if kind == 'compatibility_variant':
            if task.get('environment_variant') != parent_variant or variants.check_package(task['path'], parent_variant) != parent_variant:
                raise ValueError('frozen environment variant drift')
            if tree(receipt['official']) != receipt['official_files'] or tree(task['path']) != receipt['execution_files']:
                raise ValueError('frozen variant or original source drift')
        elif 'environment_variant' in task:
            raise ValueError('variant cannot impersonate official or diagnostic task')
        if (task['kind'] != kind or task['parent_id'] not in parents
                or tree(task['path']) != task['files']
                or {k: task[k] for k in ('workingDirectory', 'agentTimeoutSec', 'recovery', 'startup_override', 'environment_variant') if k in task}
                    != manifest['receipts'][task['parent_id']]['case_settings'][task['taskId']]):
            raise ValueError('frozen task drift')
    return manifest
