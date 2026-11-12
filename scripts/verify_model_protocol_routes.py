#!/usr/bin/env python3
"""Real-service audit via saved, scoped gateway diagnostics.

Run after building shiju in an owned Cargo slot. No credentials, upstream bodies,
or account copies survive the private temporary directory. Public rows distinguish
research, selector evidence, and bounded real-service text/stream observations.
"""
import argparse
from collections import Counter
from contextlib import contextmanager
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import secrets
import signal
import sqlite3
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request

from model_protocol_acp import acp_result, run_acp

ROOT = Path(__file__).resolve().parents[1]
PROTOCOLS = ('chat', 'messages', 'responses', 'gemini')


def compile_relay(target):
    """Use the production library and its exact Cargo dependency fingerprints."""
    if (target.parent != ROOT / 'target' or target.name not in {'rust-slot-%d' % i for i in range(1, 21)}
            or not (ROOT / 'target/rust-slot-locks' / ('slot-' + target.name.rsplit('-', 1)[1] + '.lock')).is_dir()):
        raise ValueError('claim a repository Cargo slot before compiling the relay host')
    dependencies = target / 'debug/deps'
    library = max(dependencies.glob('libvw_agent-*.rlib'), key=lambda path: path.stat().st_mtime)
    tag = library.stem.rsplit('-', 1)[1]
    fingerprints = target / 'debug/.fingerprint'
    metadata = json.loads((fingerprints / ('vw-agent-' + tag) / 'lib-vw_agent.json').read_text())
    binary = target / 'debug/model-protocol-service-relay'
    command = ['rustc', '--edition=2024', str(ROOT / 'scripts/model_protocol_service_relay.rs'),
               '--crate-name', 'model_protocol_service_relay', '-L', 'dependency=' + str(dependencies),
               '-o', str(binary), '--extern', 'vw_agent=' + str(library)]
    wanted = {'vw_api_types', 'tokio', 'serde', 'serde_json', 'anyhow'}
    for _, name, _, digest in metadata['deps']:
        if name not in wanted: continue
        for entry in fingerprints.glob('*/lib-' + name):
            value = entry.read_text().strip()
            if len(value) != 16 or int.from_bytes(bytes.fromhex(value), 'little') != digest: continue
            artifact = dependencies / ('lib' + name + '-' + entry.parent.name.rsplit('-', 1)[1] + '.rlib')
            if artifact.is_file():
                command.extend(['--extern', name + '=' + str(artifact)]); wanted.remove(name); break
    if wanted: raise RuntimeError('matching production dependencies unavailable: ' + ', '.join(sorted(wanted)))
    compiled = subprocess.run(command, capture_output=True)
    if compiled.returncode:
        raise RuntimeError('production relay host compilation failed (exit %d): %s' %
                           (compiled.returncode, compiled.stderr.decode(errors='replace')[-4000:]))
    return binary


def private_json(path, value):
    descriptor = os.open(str(path), os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, 'w') as output:
        json.dump(value, output, ensure_ascii=False)


@contextmanager
def exclusive(path):
    try:
        path.mkdir(mode=0o700)
    except FileExistsError:
        raise RuntimeError('another service audit owns the lock; inspect owner before recovery')
    try:
        private_json(path / 'owner.json', {'pid': os.getpid()})
        yield
    finally:
        (path / 'owner.json').unlink(missing_ok=True)
        path.rmdir()


def service_result(value):
    report = value.get('report', {})
    outcome = report.get('outcome', {})
    if outcome.get('status') == 'failed':
        return {'status': outcome.get('error', {}).get('code', 'invalid_response'),
                'http_status': outcome.get('httpStatus'), 'phase': outcome.get('error', {}).get('phase')}
    evidence = outcome.get('evidence', {})
    valid = (report.get('source') == 'real_service' and outcome.get('status') == 'succeeded'
             and 200 <= evidence.get('httpStatus', 0) < 300 and bool(evidence.get('actualModelId')))
    if value.get('kind') == 'model':
        route = evidence.get('route', {})
        valid = valid and {'text', 'terminal_status', 'model'}.issubset(route.get('verifiedFields', []))
        valid = valid and len(route.get('endpointFingerprint', '')) == 64
    elif value.get('kind') == 'streaming':
        valid = valid and isinstance(evidence.get('firstEventMs'), int)
    else:
        valid = False
    return {'status': 'passed' if valid else 'invalid_response',
            'http_status': evidence.get('httpStatus'), 'evidence': evidence if valid else None}


def enabled_models(config):
    models = []
    for provider, value in config.get('providers', {}).items():
        for model, settings in value.get('models', {}).items():
            if settings.get('status') == 'active':
                models.append({'provider_id': provider, 'model_key': model,
                               'model_api_id': settings.get('id', model),
                               'endpoint': settings.get('api', value.get('api')),
                               'adapter': settings.get('adapter', value.get('adapter'))})
    return models


def identity(row):
    return tuple(row[key] for key in ('source', 'provider_id', 'model_key'))


def coverage(expected, actual):
    wanted, found = Counter(map(identity, expected)), Counter(map(identity, actual))
    if wanted != found or any(count != 1 for count in found.values()):
        raise ValueError('missing, duplicate or unexpected source records')
    return len(found)


def summarize(rows):
    return dict(sorted(Counter(row['status'] for row in rows).items()))


def ensure_public(text, credentials):
    if any(credential and credential in text for credential in credentials):
        raise ValueError('public report contains credential material; output refused')
    return text


def configured_accounts(roots):
    accounts = []
    for root in roots:
        database = root / 'database/vibe-window.sqlite3'
        if not database.exists():
            continue
        auth_path = root / 'auth.json'
        auth = json.loads(auth_path.read_text()) if auth_path.exists() else {}
        with sqlite3.connect('file:' + str(database) + '?mode=ro', uri=True) as connection:
            for platform, company, payload in connection.execute('SELECT platform_id,company_id,payload_json FROM system_configs'):
                configuration = json.loads(payload)
                for model in enabled_models(configuration):
                    credential = auth.get(model['provider_id'], {})
                    provider = configuration['providers'][model['provider_id']]
                    environment_key = next((os.environ[name] for name in provider.get('env', []) if os.environ.get(name)), None)
                    # Keep each configured tenant/deployment/account separate, including
                    # local fixture scopes: an unexpected binding must remain visible.
                    model.update(config_root=root.name, platform=platform, company=company,
                                 credential=environment_key or (credential.get('key') if credential.get('type') == 'api' else None),
                                 credential_state='environment' if environment_key else 'api' if credential.get('type') == 'api' else 'missing_or_non_api')
                    accounts.append(model)
    return accounts


def http(base, path, token, body=None, method=None):
    headers = {'x-agent-apps-token': token, 'Content-Type': 'application/json',
               'x-platform-id': '10000', 'x-company-id': '100000000'}
    request = urllib.request.Request(base + path, headers=headers,
                                    data=json.dumps(body).encode() if body is not None else None,
                                    method=method)
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    try:
        with opener.open(request, timeout=40) as response:
            return response.status, json.loads(response.read(2 * 1024 * 1024))
    except urllib.error.HTTPError as error:
        # Error bodies can contain credentials or untrusted upstream payloads.
        return error.code, None


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def diagnostic(base, token, provider, model, kind, protocol, private, number):
    query = '?projectId=service-audit&targetHostId=service-audit&appId=claude'
    path = '/v1/agent-apps/diagnostics/jobs'
    body = {'kind': kind, 'providerId': provider, 'modelId': model}
    if kind == 'model':
        body['protocol'] = protocol
    status, job = http(base, path + query, token, body)
    if status != 202:
        return {'status': 'gateway_rejected', 'gateway_http_status': status}
    deadline = time.monotonic() + 45
    while time.monotonic() < deadline:
        status, snapshot = http(base, path + '/' + job['taskId'] + query, token)
        if status != 200:
            return {'status': 'gateway_rejected', 'gateway_http_status': status}
        if snapshot['state']['status'] not in ('queued', 'running', 'cancelling'):
            private_json(private / ('diagnostic-%d.json' % number), snapshot)
            results = snapshot.get('results') or []
            actual = service_result(results[0]) if len(results) == 1 else {'status': 'invalid_response'}
            if len(results) == 1:
                row = results[0]
                route = row.get('report', {}).get('outcome', {}).get('evidence', {}).get('route', {})
                if (row.get('providerId') != provider or row.get('modelId') != model
                        or row.get('kind') != kind
                        or actual['status'] == 'passed' and kind == 'model'
                        and (route.get('sourceProtocol') != protocol or route.get('upstreamProtocol') != 'chat')):
                    actual = {'status': 'binding_mismatch'}
            actual.update(gateway_http_status=status, job_state=snapshot['state']['status'])
            return actual
        time.sleep(0.2)
    http(base, path + '/' + job['taskId'] + '/cancel' + query, token, {})
    raise RuntimeError('diagnostic deadline exceeded; gateway shutdown required')


@contextmanager
def gateway(binary, private, accounts):
    token = secrets.token_urlsafe(32)
    home = private / 'home'; home.mkdir(mode=0o700)
    config = home / '.shiju-harness-dev'; config.mkdir(mode=0o700)
    (config / 'database').mkdir(mode=0o700)
    db = config / 'database/vibe-window.sqlite3'
    with sqlite3.connect(str(db)) as connection:
        connection.execute('CREATE TABLE gateway_settings (platform_id TEXT NOT NULL, company_id TEXT NOT NULL, namespace TEXT NOT NULL, config_key TEXT NOT NULL, payload_json TEXT NOT NULL, updated_at_ms INTEGER NOT NULL, PRIMARY KEY(platform_id,company_id,namespace,config_key))')
        connection.execute('CREATE TABLE system_configs (platform_id TEXT NOT NULL, company_id TEXT NOT NULL, payload_json TEXT NOT NULL, updated_at_ms INTEGER NOT NULL, PRIMARY KEY(platform_id,company_id))')
        connection.execute('INSERT INTO system_configs VALUES (?,?,?,?)', ('10000', '100000000', json.dumps({'gateway': {'host': '127.0.0.1', 'port': 0, 'auth_enabled': False}}), int(time.time() * 1000)))
        for index, account in enumerate(accounts):
            # Explicit snapshot of the configured destination, never an inferred URL.
            row = {'name': 'Private audit', 'protocol': 'anthropic', 'baseUrl': account['endpoint'],
                   'apiKey': account['credential'] or 'unavailable', 'model': account['model_api_id'],
                   'meta': {'apiFormat': 'openai_chat'}}
            connection.execute('INSERT INTO gateway_settings VALUES (?,?,?,?,?,?)',
                               ('10000', '100000000', 'app_providers.records', '"claude":audit-%d' % index, json.dumps(row), int(time.time() * 1000)))
    os.chmod(db, 0o600)
    private_json(config / 'shiju.json', {'gateway': {'host': '127.0.0.1', 'port': 0, 'auth_enabled': False}})
    policy = private / 'access.json'
    private_json(policy, {'principals': [{'id': 'service-audit', 'token_sha256': hashlib.sha256(token.encode()).hexdigest(),
                  'grants': [{'platform_id': '10000', 'company_id': '100000000', 'host_id': 'service-audit',
                              'provider_config_read': True, 'routing_manage': True,
                              'routing_apps': [{'project_id': 'service-audit', 'app_id': 'claude'}]}]}],
                 'hosts': [{'id': 'service-audit', 'binding': 'local', 'online': True, 'home': str(home),
                            'applications': str(home), 'search_paths': [], 'credential_root': str(home)}]})
    environment = os.environ.copy()
    environment.update(VIBEWINDOW_TEST_ROOT=str(home), VIBEWINDOW_CONFIG_DIR=str(config),
                       SHIJU_AGENT_APPS_ACCESS_FILE=str(policy))
    # SQLite snapshot and source credentials are private; CLI output is private too.
    log = private / 'gateway.log'
    descriptor = os.open(str(log), os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, 'w') as output:
        process = subprocess.Popen([str(binary), '--config-dir', str(config), 'gateway', '--host', '127.0.0.1', '-p', '0'],
                                   cwd=str(private), env=environment, stdout=output, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 120
            base = None
            while time.monotonic() < deadline:
                text = log.read_text(errors='replace')
                import re
                match = re.search(r'Gateway listening on http://(127\.0\.0\.1:\d+)', text)
                if match:
                    base = 'http://' + match.group(1); break
                if process.poll() is not None:
                    raise RuntimeError('private gateway exited before listening (exit %s)' % process.returncode)
                time.sleep(0.2)
            if base is None:
                raise RuntimeError('private gateway startup deadline exceeded')
            yield base, token, {'address': base, 'pid': process.pid, 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest()}
        finally:
            process.terminate()
            try:
                process.wait(timeout=20)
            except subprocess.TimeoutExpired:
                process.kill(); process.wait()


def service_records(rows, observations):
    records = []
    for row in rows:
        configured = [i for i, account in enumerate(observations) if account['provider_id'] == row['provider_id'] and account['model_key'] == row['model_key']]
        endpoint = row.get('source_declarations', {}).get('api')
        fingerprint = hashlib.sha256(endpoint.encode()).hexdigest() if isinstance(endpoint, str) else None
        matches = [i for i in configured if observations[i].get('model_api_id') == row.get('model_api_id')
                   and fingerprint and observations[i].get('endpoint_fingerprint') == fingerprint]
        overrides = [i for i in configured if i not in matches]
        records.append({key: row.get(key) for key in ('source', 'provider_id', 'model_key', 'model_api_id', 'canonical_model_id', 'applicability', 'research_conclusion', 'research_evidence_ids', 'research_reason')} | {
            'evidence_type': 'official_research_with_separate_service_observations',
            'service_observation_indices': matches,
            'configured_override_observation_indices': overrides,
            'service_status': 'observed' if matches else 'configured_binding_only' if overrides else 'not_configured_enabled',
            'unverified_reason': None if matches else 'Configured API ID or endpoint differs from the source binding; evidence applies only to that configuration.' if overrides else 'No configured active model/account binding in inspected local configurations; retain research conclusion, not a protocol rejection.'})
    return records


def run_service(args):
    inventory = json.loads(args.inventory.read_text())
    offline = json.loads((ROOT / 'docs/dev/model-protocols/offline-matrix.json').read_text())
    from audit_model_protocols import check_coverage
    errors = check_coverage(inventory, offline)
    if errors:
        raise ValueError('offline inventory validation failed: ' + '; '.join(errors[:3]))
    coverage(inventory['records'], offline['records'])
    for source, expected in offline['source_hashes'].items():
        if hashlib.sha256((ROOT / source).read_bytes()).hexdigest() != expected:
            raise ValueError('catalog source hash changed: ' + source)
    roots = args.config_root or [Path.home() / '.shiju-harness', Path.home() / '.shiju-harness-dev']
    accounts = configured_accounts(roots)
    binary = args.binary or Path(os.environ['CARGO_TARGET_DIR']) / 'debug/shiju'
    if not binary.is_file():
        raise ValueError('freshly built gateway binary is missing')
    source_rows = {(r['provider_id'], r['model_key']): r for r in offline['records']}
    for account in accounts:
        declared = source_rows.get((account['provider_id'], account['model_key']), {}).get('source_declarations', {})
        account['adapter'] = account['adapter'] or declared.get('adapter') or declared.get('npm')
        account['endpoint'] = account['endpoint'] or declared.get('api')
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=str(ROOT), text=True).strip()
    worktree_diff = subprocess.check_output(['git', 'diff', 'HEAD', '--', 'crates', 'Cargo.toml', 'Cargo.lock'], cwd=str(ROOT))
    observations = []
    rate_limited_accounts = {}
    lock = ROOT / 'target/model-protocol-service.lock'
    with exclusive(lock), tempfile.TemporaryDirectory(prefix='shiju-protocol-service-') as directory:
        private = Path(directory); os.chmod(private, 0o700)
        relay_binary = compile_relay(binary.parent.parent.resolve())
        with gateway(binary.resolve(), private, accounts) as (base, token, binding):
            for index, account in enumerate(accounts):
                public = {key: value for key, value in account.items() if key not in ('credential', 'endpoint')}
                public['account_binding'] = hashlib.sha256(json.dumps({k: account[k] for k in ('config_root', 'platform', 'company', 'provider_id', 'endpoint')}, sort_keys=True).encode()).hexdigest()
                public.pop('platform'); public.pop('company')
                public['endpoint_fingerprint'] = hashlib.sha256((account.get('endpoint') or '').encode()).hexdigest()
                catalog_row = source_rows.get((account['provider_id'], account['model_key']))
                modalities = (catalog_row or {}).get('capabilities', {}).get('modalities', {})
                credential_id = hashlib.sha256((account['credential'] or '').encode()).hexdigest()
                if modalities.get('output') and 'text' not in modalities['output']:
                    public.update(status='not_applicable_text', attempts=[])
                elif credential_id in rate_limited_accounts:
                    public.update(status='deferred_rate_limit', attempts=[], deferred_after_observation=rate_limited_accounts[credential_id])
                elif not account['credential']:
                    public.update(status='missing_credentials', attempts=[])
                elif account['adapter'] not in ('openai-compatible', '@ai-sdk/openai-compatible', '@ai-sdk/openai'):
                    public.update(status='unverified_adapter', attempts=[])
                elif not account.get('endpoint'):
                    public.update(status='missing_endpoint', attempts=[])
                else:
                    attempts = []
                    for protocol in PROTOCOLS:
                        result = diagnostic(base, token, 'audit-%d' % index, account['model_api_id'], 'model', protocol, private, len(observations) * 10 + len(attempts))
                        result.update(kind='model', source_protocol=protocol, upstream_protocol='chat')
                        attempts.append(result)
                        # Account failures do not cause protocol/endpoint guessing.
                        if result['status'] in ('unauthorized', 'rate_limited'):
                            break
                    if attempts[0]['status'] == 'passed':
                        result = diagnostic(base, token, 'audit-%d' % index, account['model_api_id'], 'streaming', None, private, len(observations) * 10 + len(attempts))
                        result.update(kind='streaming', source_protocol='chat', upstream_protocol='chat'); attempts.append(result)
                    public.update(status='diagnostic_observed', attempts=attempts)
                    if attempts[0]['status'] not in ('unauthorized', 'rate_limited', 'gateway_rejected'):
                        from model_protocol_service_wire import verify_relay
                        public['production_attempts'] = verify_relay(relay_binary, private / ('relay-%d' % index), account, (catalog_row or {}).get('capabilities', {}), private_json)
                    else:
                        public['production_attempts'] = []
                        if attempts[0]['status'] == 'unauthorized': public['status'] = 'access_denied'
                    actual = public['production_attempts']
                    if actual:
                        bounded = [a for a in actual if a['kind'] != 'public_default_text']
                        public['status'] = 'bounded_modes_passed' if bounded and all(a['status'] == 'passed' for a in bounded) else 'observed_with_failures'
                    if any(a.get('status') == 'rate_limited' for a in attempts + actual):
                        rate_limited_accounts[credential_id] = len(observations)
                observations.append(public)
                print(json.dumps({'provider': account['provider_id'], 'model': account['model_api_id'], 'status': public['status'], 'attempts': [a['status'] for a in public['attempts']]}), flush=True)
    records = service_records(offline['records'], observations)
    report = {'schema_version': 1, 'checked_at': datetime.now(timezone.utc).isoformat(),
              'source_revision': revision, 'source_diff_sha256': hashlib.sha256(worktree_diff).hexdigest(),
              'source_hashes': offline['source_hashes'], 'gateway': binding,
              'production_relay': {'binary_sha256': hashlib.sha256(relay_binary.read_bytes()).hexdigest(),
                                   'host_source_sha256': hashlib.sha256((ROOT / 'scripts/model_protocol_service_relay.rs').read_bytes()).hexdigest(),
                                   'public_api': 'RelayServer + ProviderRelayBackend'},
              'research_reports': [{'path': str(p.relative_to(ROOT)), 'sha256': hashlib.sha256(p.read_bytes()).hexdigest(), 'checked_at': json.loads(p.read_text())['checked_at']} for p in sorted((ROOT / 'docs/dev/model-protocols/research').glob('group-*.json'))],
              'scope': 'Configured active model/account snapshots. Saved diagnostics and public production ACP relay are separate evidence. Public relay checks JSON/SSE, correlated fixed echo tool rounds and explicit thinking modes; no tool command executes and no endpoint is guessed. Default Gemini requests and explicitly disabled Gemini thinking remain separate observations; the current production conversion may disable thinking for a default request. Converted response model fields may use the public binding alias; model_api_id is the configured upstream request ID and native Chat observations retain the upstream response ID.',
              'records': records, 'observations': observations,
              'summary': {'source_records': coverage(inventory['records'], records), 'configured_models': len(observations),
                          'model_status': summarize(observations), 'attempt_status': summarize([a for o in observations for a in o['attempts']]),
                          'production_attempt_status': summarize([a for o in observations for a in o.get('production_attempts', [])])}}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    metadata = {key: value for key, value in report.items() if key != 'records'}
    prefix = json.dumps(metadata, ensure_ascii=False, indent=2)[:-1].rstrip()
    serialized = prefix + ',\n  "records": [\n' + ',\n'.join(
        '    ' + json.dumps(row, ensure_ascii=False, separators=(',', ':')) for row in records) + '\n  ]\n}\n'
    args.output.write_text(ensure_public(serialized, {account.get('credential') for account in accounts}))
    print(json.dumps(report['summary'], ensure_ascii=False))


def boundary_tests():
    """Compile the actual small modules, with the same owned slot dependencies."""
    target = Path(os.environ['CARGO_TARGET_DIR']).resolve()
    if target.parent != ROOT / 'target' or target.name not in {'rust-slot-%d' % i for i in range(1, 21)}:
        raise ValueError('boundary verification requires an owned repository Cargo slot')
    slot = target.name.removeprefix('rust-slot-')
    if not (ROOT / 'target/rust-slot-locks' / ('slot-%s.lock' % slot)).is_dir():
        raise ValueError('claim the shared Cargo slot before boundary verification')
    dependencies = target / 'debug/deps'
    with tempfile.TemporaryDirectory(prefix='shiju-protocol-boundaries-') as directory:
        root = Path(directory)
        wrapper = root / 'boundaries.rs'
        wrapper.write_text(
            '#[path="%s"] mod global;\nmod config {\n'
            '#[path="%s"] mod runtime_home;\n'
            '#[path="%s"] mod runtime_home_tests;\n}\n' % (
                ROOT / 'crates/vw-agent/src/global/mod.rs',
                ROOT / 'crates/vw-agent/src/config/runtime_home.rs',
                ROOT / 'crates/vw-agent/src/config/runtime_home_tests.rs'))
        binary = target / 'debug/model-protocol-boundary-tests'
        command = ['rustc', '--edition=2024', '--test', str(wrapper), '--crate-name',
                   'model_protocol_boundaries', '-L', 'dependency=' + str(dependencies), '-o', str(binary)]
        for name in ('directories', 'vw_config_types', 'tempfile', 'anyhow'):
            library = max(dependencies.glob('lib%s-*.rlib' % name), key=lambda path: path.stat().st_mtime)
            command.extend(['--extern', name + '=' + str(library)])
        compiled = subprocess.run(command, capture_output=True)
        if compiled.returncode:
            raise RuntimeError('boundary test compilation failed (exit %d)' % compiled.returncode)
        environment = os.environ.copy()
        environment['VIBEWINDOW_TEST_ROOT'] = str(root)
        return subprocess.run([str(binary), '--nocapture'], env=environment).returncode


def parser():
    result = argparse.ArgumentParser(description=__doc__)
    commands = result.add_subparsers(dest='command', required=True)
    acp = commands.add_parser('acp')
    acp.add_argument('--inventory', type=Path, required=True)
    acp.add_argument('--output', type=Path, required=True)
    acp.add_argument('--binary', type=Path)
    acp.add_argument('--config-root', type=Path, action='append')
    service = commands.add_parser('service')
    service.add_argument('--inventory', type=Path, required=True)
    service.add_argument('--output', type=Path, required=True)
    service.add_argument('--binary', type=Path)
    service.add_argument('--config-root', type=Path, action='append')
    commands.add_parser('boundary-tests')
    commands.add_parser('compile-relay')
    return result


if __name__ == '__main__':
    def interrupted(signum, frame):
        raise KeyboardInterrupt('service audit interrupted')
    signal.signal(signal.SIGTERM, interrupted)
    arguments = parser().parse_args()
    if arguments.command == 'boundary-tests':
        raise SystemExit(boundary_tests())
    if arguments.command == 'compile-relay':
        compile_relay(Path(os.environ['CARGO_TARGET_DIR']).resolve())
        raise SystemExit(0)
    if arguments.command == 'acp':
        raise SystemExit(run_acp(arguments))
    run_service(arguments)
