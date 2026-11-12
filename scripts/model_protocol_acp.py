"""Real CLI -> gateway -> selected deployment acceptance, with private wire capture."""
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import hashlib
import http.client
import json
import os
from pathlib import Path
import secrets
import signal
import subprocess
import tempfile
import threading
import urllib.error
import urllib.parse
import urllib.request

CLIENTS = {'opencode': 'chat', 'claude': 'messages', 'codex': 'responses', 'gemini': 'gemini'}


def acp_result(evidence):
    required = ('connected', 'binding_verified', 'tool_write', 'tool_read', 'directory_scope', 'upstream_terminal')
    valid = (all(evidence.get(key) is True for key in required)
             and evidence.get('terminal') == 'chat.done' and evidence.get('errors') is False
             and isinstance(evidence.get('file_sha256'), str) and len(evidence['file_sha256']) == 64
             and evidence['file_sha256'] == evidence.get('expected_sha256')
             and evidence.get('stream_requests', 0) > 0
             and bool(evidence.get('http_statuses'))
             and all(200 <= code < 300 for code in evidence['http_statuses'])
             and bool(evidence.get('upstream_models'))
             and set(evidence['upstream_models']) == {evidence.get('expected_model')})
    if valid: return 'passed'
    for code in evidence.get('http_statuses', []):
        if code in (401, 402, 403, 429):
            return {401: 'unauthorized', 402: 'insufficient_balance', 403: 'forbidden', 429: 'rate_limited'}[code]
        if code >= 500: return 'upstream_server_error'
        if code >= 400: return 'upstream_rejected'
    return 'incomplete_evidence'


def upstream_terminal(requests, protocol="chat"):
    if not requests: return False
    for row in requests:
        try:
            if row['stream']:
                payloads = [line[5:].removeprefix(' ') for line in row['response'].decode().splitlines() if line.startswith('data:')]
                if protocol == 'messages':
                    values = [json.loads(value) for value in payloads]
                    if not values or values[-1].get('type') != 'message_stop': return False
                else:
                    if not payloads or payloads[-1] != '[DONE]': return False
                    values = [json.loads(value) for value in payloads[:-1]]
            else:
                values = [json.loads(row['response'])]
            if any('error' in value for value in values): return False
            if protocol == 'messages':
                if not any(value.get('stop_reason', value.get('delta', {}).get('stop_reason'))
                           in ('end_turn', 'tool_use') for value in values): return False
                continue
            if not any(choice.get('finish_reason') in ('stop', 'tool_calls')
                       for value in values for choice in value.get('choices', [])): return False
        except (ValueError, UnicodeError, TypeError):
            return False
    return True


def native_messages_endpoint(account):
    url = urllib.parse.urlsplit(account['endpoint'])
    if url.scheme != 'https' or url.username or url.password or url.query or url.fragment or url.port:
        return None
    if (url.hostname == 'api.deepseek.com' and url.path.rstrip('/') in ('', '/v1')
            and account['model_api_id'] in ('deepseek-flash', 'deepseek-v4-pro')):
        path = '/anthropic/v1/messages'
    elif (url.hostname in ('token-plan.cn-beijing.maas.aliyuncs.com', 'token-plan.ap-southeast-1.maas.aliyuncs.com')
          and url.path.rstrip('/') in ('/compatible-mode/v1', '/compatible-mode/v1/chat/completions')):
        path = '/apps/anthropic/v1/messages'
    else:
        return None
    # Exact same-origin pairs already supported by production transport.rs; no discovery.
    return urllib.parse.urlunsplit((url.scheme, url.netloc, path, '', ''))


def messages_for_audit(messages):
    normalized = []
    for message in messages:
        blocks = message.get('content', [])
        if not isinstance(blocks, list): continue
        calls = [{'id': block.get('id'), 'function': {'name': block.get('name'),
                  'arguments': json.dumps(block.get('input', {}))}}
                 for block in blocks if block.get('type') == 'tool_use']
        if calls: normalized.append({'role': 'assistant', 'tool_calls': calls})
        for block in blocks:
            if block.get('type') == 'tool_result':
                normalized.append({'role': 'tool', 'tool_call_id': block.get('tool_use_id'),
                                   'content': block.get('content', '')})
    return normalized


@contextmanager
def recording_endpoint(account, protocol="chat"):
    """Forward one configured or production-reviewed operation; reject redirects."""
    from verify_model_protocol_routes import NoRedirect
    target = account['endpoint'].rstrip('/')
    operation = '/v1/chat/completions'
    if protocol == 'messages':
        target = native_messages_endpoint(account)
        if not target: raise ValueError('no reviewed native Messages endpoint for this binding')
        operation = '/v1/messages'
    elif not target.endswith('/chat/completions'):
        target += '/chat/completions'
    requests = []
    key = secrets.token_hex(24)
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_POST(self):
            authenticated = (self.headers.get('x-api-key') == key if protocol == 'messages'
                             else self.headers.get('Authorization') == 'Bearer ' + key)
            if self.path != operation or not authenticated:
                self.send_error(403); return
            body = self.rfile.read(int(self.headers.get('Content-Length', 0)))
            payload = json.loads(body)
            observation = {'model': payload.get('model'), 'stream': payload.get('stream', False),
                           'messages': messages_for_audit(payload.get('messages', [])) if protocol == 'messages' else payload.get('messages', []), 'response': b'', 'http_status': None}
            requests.append(observation)
            headers = {'Content-Type': 'application/json', 'Authorization': 'Bearer ' + account['credential']}
            if protocol == 'messages':
                headers = {'Content-Type': 'application/json', 'x-api-key': account['credential'],
                           'anthropic-version': self.headers.get('anthropic-version', '2023-06-01')}
                if self.headers.get('anthropic-beta'): headers['anthropic-beta'] = self.headers['anthropic-beta']
            request = urllib.request.Request(target, body, headers)
            try:
                response = opener.open(request, timeout=120)
            except urllib.error.HTTPError as error:
                response = error
            except (OSError, urllib.error.URLError):
                observation['http_status'] = 502
                self.send_error(502); return
            with response:
                observation['http_status'] = response.code
                self.send_response(response.code)
                self.send_header('Content-Type', response.headers.get('Content-Type', 'application/json'))
                self.send_header('Connection', 'close')
                self.end_headers()
                self.close_connection = True
                try:
                    while True:
                        block = response.read1(65536)
                        if not block: break
                        observation['response'] += block
                        self.wfile.write(block); self.wfile.flush()
                except (BrokenPipeError, ConnectionResetError, TimeoutError):
                    observation['interrupted'] = True

    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    server.daemon_threads = False
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield 'http://127.0.0.1:%d/v1' % server.server_port, key, requests
    finally:
        server.shutdown(); server.server_close(); thread.join()


def tool_scope(requests, workspace, allowed_commands=()):
    for row in requests:
        for message in row['messages']:
            for call in message.get('tool_calls', []):
                function = call.get('function', {})
                name = function.get('name', '').lower()
                command_tool = any(word in name for word in ('bash', 'shell', 'terminal', 'exec', 'command'))
                try:
                    arguments = json.loads(function.get('arguments', '{}'))
                except (ValueError, TypeError):
                    return False
                if not isinstance(arguments, dict): return False
                if command_tool and arguments.get('cmd', arguments.get('command')) not in allowed_commands:
                    return False
                for key in ('path', 'file_path', 'filePath', 'dir_path', 'workdir'):
                    value = arguments.get(key)
                    if not isinstance(value, str): continue
                    target = Path(value)
                    if not target.is_absolute(): target = workspace / target
                    try:
                        target.resolve().relative_to(workspace.resolve())
                    except ValueError:
                        return False
    return True


def wire_tools(requests, output_name, challenge, allowed_commands=()):
    """Require a model-emitted write, a correlated result, and a subsequent read result."""
    calls, results = {}, {}
    for request in requests:
        for message in request['messages']:
            for call in message.get('tool_calls', []):
                function = call.get('function', {})
                calls[call.get('id')] = (function.get('name', ''), function.get('arguments', ''))
            if message.get('role') == 'tool':
                results[message.get('tool_call_id')] = str(message.get('content', ''))
    write = read = False
    for identifier, (name, arguments) in calls.items():
        if identifier not in results or output_name not in arguments: continue
        try:
            decoded = json.loads(arguments)
        except (ValueError, TypeError):
            decoded = {}
        if not isinstance(decoded, dict): continue
        command = decoded.get('cmd', decoded.get('command'))
        if command in allowed_commands:
            write = write or (challenge in command)
            read = read or (('read_text' in command or command.startswith('cat ')) and challenge in results[identifier])
        lowered = name.lower()
        if any(word in lowered for word in ('write', 'edit', 'patch')) and challenge in arguments:
            write = True
        if 'read' in lowered and challenge in results[identifier]:
            read = True
    return write, read


def turn(binary, entry, account, root, thinking_mode="request_default", upstream_protocol="chat"):
    from verify_acp_approval import environment, gateway, request
    from verify_acp_model_source import locate, claude_sdk_identity
    from acp_skill_mount_codex import locate_codex
    backend = entry['key']
    env = environment(root)
    # Existing application-managed installations are not always on the login PATH.
    managed = [path / 'apps/bin' for path in (Path.home() / '.shiju-harness', Path.home() / '.shiju-harness-dev')]
    contracts = json.loads((Path(__file__).resolve().parents[1] / 'crates/vw-acp/src/managed_runtime/contracts.json').read_text())
    if backend in contracts:
        release = contracts[backend]['version']
        runtime = Path.home() / '.local/share/shiju/acp-runtime' / (backend + '-' + release + '-model-source-v1')
        managed.insert(0, runtime / 'node_modules/.bin')
    env['PATH'] = os.pathsep.join([str(path) for path in managed if path.is_dir()] + [env.get('PATH', '')])
    env['VIBEWINDOW_TEST_ROOT'] = str(root / 'home')
    spec = locate_codex(entry, env) if backend == 'codex' else locate(entry, env)
    cli_identity = {'release': entry['distribution']['release'],
                    'entry_sha256': hashlib.sha256(Path(spec['command']).read_bytes()).hexdigest()}
    if backend == 'claude':
        cli_identity.update(claude_sdk_identity(spec['command'], entry, env))
    if backend == 'gemini':
        spec['args'] = ['--experimental-acp']
    workspace = root / 'workspace'; workspace.mkdir()
    challenge = secrets.token_hex(24)
    output = workspace / 'acp-proof.txt'
    output.write_text('before acceptance\n')
    write_command = "python3 -c 'from pathlib import Path; Path(\"acp-proof.txt\").write_text(\"" + challenge + "\")'"
    read_command = "python3 -c 'from pathlib import Path; print(Path(\"acp-proof.txt\").read_text(), end=\"\")'"
    commands = (write_command, read_command, 'cat acp-proof.txt', 'cat ' + str(output),
                'cat -- acp-proof.txt', 'cat -- ' + str(output), 'pwd', 'ls', 'ls -la', 'ls -l')
    prompt = ('In this workspace only, overwrite the existing file ' + str(output) + ' with exactly '
              + challenge + ' with no newline. Then read that same absolute file path using a separate tool call. '
              'Use native file tools when available. If your only read/write tools require commands, use only '
              'these exact commands in separate calls with this workspace as cwd: ' + write_command + ' and '
              + read_command + '. Do not change any other files. Reply with the file contents.')
    with recording_endpoint(account, upstream_protocol) as (endpoint, key, requests):
        env['ACP_PROTOCOL_AUDIT_KEY'] = key
        # Single-protocol diagnostic candidates keep forced conversion out of defaults.
        # The recorder authorizes exactly one reviewed account operation.
        candidate = {'protocol': upstream_protocol, 'url': endpoint + ('/messages' if upstream_protocol == 'messages' else '/chat/completions'),
                     'deployment': 'private-acp-acceptance', 'authentication': ({'kind': 'api_key_header', 'name': 'x-api-key'}
                                     if upstream_protocol == 'messages' else {'kind': 'bearer'}),
                     'models': {'kind': 'models', 'ids': [account['model_api_id']]},
                     'evidence': [{'kind': 'configuration', 'source': 'isolated acceptance fixture',
                                   'reference': 'configured account operation or production same-origin Messages pair', 'checked_at': '2026-11-12'}]}
        model = dict(account.get('capabilities', {}))
        model.update({'id': account['model_api_id'], 'status': 'active', 'tool_call': True,
                 'temperature': True, 'protocols': [candidate],
                 'limit': {'context': 200000, 'output': 16384}})
        config = {'workspace_dir': str(workspace), 'gateway': {'auth_enabled': False},
                  'agents': {'acp-audit': {'enabled': True, 'runtime': {'kind': 'acp', 'backend': backend},
                              'defaults': {'skills': {'mode': 'fixed', 'value': []},
                                           'mcp': {'mode': 'fixed', 'value': []}}}},
                  'acp': {backend: spec}, 'providers': {account['provider_id']: {'name': 'Private ACP audit',
                      'api': endpoint, 'adapter': 'anthropic' if upstream_protocol == 'messages' and backend == 'claude' else 'openai-compatible', 'env': ['ACP_PROTOCOL_AUDIT_KEY'],
                      'models': {account['model_api_id']: model}}}}
        with gateway(str(binary), root, env) as base:
            request(base, '/global/config', config, method='PATCH')
            query = '?' + urllib.parse.urlencode({'directory': str(workspace)})
            session = json.loads(request(base, '/session' + query, {'title': 'Private protocol acceptance'}))['id']
            body = {'session_id': session, 'agent': 'acp-audit', 'acp_model_source': 'session_model',
                    'model': account['provider_id'] + '/' + account['model_api_id'],
                    'options': {'full_access': True, 'cwd': str(workspace),
                                'acp_close_client_after_request': False},
                    'messages': [{'role': 'user', 'content': prompt}]}
            if thinking_mode == 'explicit_none':
                body['reasoning_effort'] = 'none'
            response = {}; finished = threading.Event()
            def receive():
                try:
                    response['stream'] = request(base, '/chat/stream' + query, body)
                except http.client.IncompleteRead as error:
                    response['stream'] = error.partial.decode(errors='replace')
                except Exception as error:
                    response['error'] = error
                finally:
                    finished.set()
            reader = threading.Thread(target=receive, daemon=True)
            reader.start()
            connected_during_turn = False
            while not finished.wait(0.1):
                current = json.loads(request(base, '/session/' + session + '/acp-options' + query))
                connected_during_turn = connected_during_turn or current.get('connected') is True
            reader.join()
            if 'error' in response: raise response['error']
            stream = response['stream']
            controls = json.loads(request(base, '/session/' + session + '/acp-options' + query))
            stored = json.loads(request(base, '/session/' + session + query))
            # All raw content remains inside the mode-0700 temporary directory.
            from verify_model_protocol_routes import private_json
            private_json(root / 'raw.json', {'sse': stream, 'controls': controls, 'session': stored,
                         'wire': [dict(row, response=row['response'].decode(errors='replace')) for row in requests]})
        events = [json.loads(line[6:]) for line in stream.splitlines() if line.startswith('data: {')]
        types = [event.get('type') for event in events]
        write, read = wire_tools(requests, output.name, challenge, commands)
        answer = ''.join(event.get('delta', '') for event in events if event.get('type') == 'chat.delta')
        selected = controls.get('selected_model') or {}
        binding = (controls.get('acp_model_source') == 'session_model'
                   and selected.get('source') == 'session_model'
                   and selected.get('provider_id') == account['provider_id']
                   and selected.get('model') == account['provider_id'] + '/' + account['model_api_id']
                   and stored.get('id') == session and stored.get('directory') == str(workspace))
        evidence = {'connected': connected_during_turn or controls.get('connected') is True,
                    'connected_after_turn': controls.get('connected') is True, 'binding_verified': binding,
                    'terminal': 'chat.done' if types and types[-1] == 'chat.done' else None,
                    'event_sequence': types, 'errors': 'chat.error' in types,
                    'tool_write': write, 'tool_read': read and challenge in answer,
                    'file_sha256': hashlib.sha256(output.read_bytes()).hexdigest() if output.is_file() else None,
                    'expected_sha256': hashlib.sha256(challenge.encode()).hexdigest(),
                    'directory_scope': tool_scope(requests, workspace, commands),
                    'http_statuses': [row['http_status'] or 0 for row in requests],
                    'upstream_models': sorted({row['model'] for row in requests if row['model']}),
                    'expected_model': account['model_api_id'], 'request_count': len(requests),
                    'tool_names': sorted({call.get('function', {}).get('name', '') for row in requests
                                          for message in row['messages'] for call in message.get('tool_calls', [])}),
                    'stream_requests': sum(row['stream'] is True for row in requests),
                    'upstream_terminal': upstream_terminal(requests, upstream_protocol)}
        evidence['thinking_mode'] = thinking_mode
        evidence['cli_identity'] = cli_identity
        evidence['status'] = acp_result(evidence)
        # Errors are classified, never copied from CLI/provider payloads.
        if evidence['errors']:
            from verify_acp_approval import stream_failure_categories
            evidence['failure_categories'] = stream_failure_categories(stream)
            if (thinking_mode == 'explicit_none' and not requests
                    and 'session/set_config_option' in stream and 'Invalid params' in stream):
                evidence['failure_categories'].append('explicit_none_control_rejected')
            if 'Unsupported Messages capability' in stream:
                evidence['failure_categories'].append('unsupported_messages_capability')
                evidence['relay_http_status'] = 422
                evidence['status'] = 'capability_rejected'
        return evidence


def stored_deepseek_diagnostic(roots, records, accounts):
    """Use saved credentials only in the explicitly requested isolated diagnostic."""
    if any(a['provider_id'] == 'deepseek' for a in accounts): return []
    row = next((r for r in records if r['provider_id'] == 'deepseek' and r['model_key'] == 'deepseek-v4-pro'), None)
    if row is None: return []
    import sqlite3
    for root in reversed(roots):
        auth = root / 'auth.json'; database = root / 'database/vibe-window.sqlite3'
        if not auth.is_file() or not database.is_file(): continue
        credential = json.loads(auth.read_text()).get('deepseek', {})
        if credential.get('type') != 'api' or not credential.get('key'): continue
        with sqlite3.connect('file:' + str(database) + '?mode=ro', uri=True) as connection:
            for platform, company, payload in connection.execute('SELECT platform_id,company_id,payload_json FROM system_configs'):
                provider = json.loads(payload).get('providers', {}).get('deepseek', {})
                if not provider.get('api'): continue
                return [dict(provider_id='deepseek', model_key=row['model_key'], model_api_id=row['model_api_id'],
                             endpoint=provider['api'], adapter=provider.get('adapter') or 'openai-compatible',
                             config_root=root.name, platform=platform, company=company,
                             credential=credential['key'], credential_state='stored_api_explicit_diagnostic',
                             diagnostic_only=True, capabilities=row.get('capabilities', {}))]
    return []


def acceptance_summary(rows):
    passed = [row for row in rows if row['status'] == 'passed']
    pairs = {}
    for provider in ('alibaba-token-plan-cn', 'deepseek'):
        native = {(r['model_api_id'], r['account_binding']) for r in passed
                  if r['provider_id'] == provider and r['route_mode'] == 'native'}
        converted = {(r['model_api_id'], r['account_binding']) for r in passed
                     if r['provider_id'] == provider and r['route_mode'] == 'forced_conversion'}
        pairs[provider] = bool(native & converted)
    protocols = {protocol: any(row['source_protocol'] == protocol for row in passed)
                 for protocol in CLIENTS.values()}
    shiju = any(row['provider_id'] == 'shiju' and row.get('http_statuses') for row in rows)
    return {'provider_pairs': pairs, 'successful_client_protocols': protocols,
            'shiju_public_interface_observed': shiju,
            'core_acceptance_passed': all(pairs.values()) and all(protocols.values()) and shiju,
            'all_probes_passed': len(passed) == len(rows)}


def working_account_bindings(observations):
    # A rejected conversion mode must not discard a verified native account and
    # accidentally prefer a lexically earlier model whose entitlement was denied.
    return {(o['provider_id'], o['model_api_id']) for o in observations
            if o['status'] == 'bounded_modes_passed' or any(
                a.get('kind') == 'model' and a.get('source_protocol') == 'chat'
                and a.get('upstream_protocol') == 'chat' and a.get('status') == 'passed'
                for a in o.get('attempts', []))}


def run_acp(args):
    from datetime import datetime, timezone
    from verify_model_protocol_routes import ROOT, configured_accounts, exclusive, ensure_public, summarize
    from verify_acp_model_source import cargo_slot, Unavailable
    def deadline(signum, frame):
        raise TimeoutError('real ACP turn deadline exceeded')
    signal.signal(signal.SIGALRM, deadline)
    slot = cargo_slot(os.environ)
    binary = (args.binary or Path(slot) / 'debug/shiju').resolve()
    if not binary.is_file(): raise ValueError('freshly built gateway binary is missing')
    inventory = json.loads(args.inventory.read_text())
    for source, expected in inventory['sources'].items():
        if hashlib.sha256((ROOT / source).read_bytes()).hexdigest() != expected['sha256']:
            raise ValueError('inventory source hash changed: ' + source)
    roots = args.config_root or [Path.home() / '.shiju-harness', Path.home() / '.shiju-harness-dev']
    accounts = configured_accounts(roots)
    offline = json.loads((ROOT / 'docs/dev/model-protocols/offline-matrix.json').read_text())
    accounts.extend(stored_deepseek_diagnostic(roots, offline['records'], accounts))
    capabilities = {(row['provider_id'], row['model_key']): row.get('capabilities', {}) for row in offline['records']}
    for account in accounts:
        account['capabilities'] = capabilities.get((account['provider_id'], account['model_key']), {})
    manifest = json.loads((ROOT / 'docs/shiju/acp-model-source/cli-versions.json').read_text())
    entries = {entry['key']: entry for entry in manifest['entries']}
    # Prefer an already observed working account; never try alternative endpoints.
    prior = json.loads((ROOT / 'docs/dev/model-protocols/service-matrix.json').read_text())
    working = working_account_bindings(prior['observations'])
    observations = []
    with exclusive(ROOT / 'target/model-protocol-acp.lock'):
        for provider in ('alibaba-token-plan-cn', 'deepseek', 'shiju'):
            candidates = [a for a in accounts if a['provider_id'] == provider and a['credential']]
            candidates.sort(key=lambda a: ((provider, a['model_api_id']) not in working, a['model_api_id']))
            if not candidates:
                observations.append({'provider_id': provider, 'status': 'missing_credentials',
                                     'reason': 'No configured active account for this provider; an Alibaba-hosted DeepSeek model is a separate deployment.'})
                continue
            account = candidates[0]
            for backend, protocol in CLIENTS.items():
                variants = [('chat', 'request_default')]
                if backend == 'claude':
                    variants.append(('chat', 'explicit_none'))
                    if native_messages_endpoint(account): variants.insert(0, ('messages', 'request_default'))
                elif native_messages_endpoint(account):
                    # The client keeps its original source protocol; only the reviewed
                    # upstream candidate changes. Default and explicit none are
                    # separate; a CLI option rejection is not relay wire evidence.
                    variants.extend([('messages', 'request_default'), ('messages', 'explicit_none')])
                for upstream, mode in variants:
                    public = {'provider_id': provider, 'model_api_id': account['model_api_id'],
                              'client': backend, 'thinking_mode': mode, 'source_protocol': protocol, 'upstream_protocol': upstream,
                              'diagnostic_only': account.get('diagnostic_only', False),
                              'route_mode': 'native' if protocol == upstream else 'forced_conversion',
                              'endpoint_fingerprint': hashlib.sha256(account['endpoint'].encode()).hexdigest(),
                              'account_binding': hashlib.sha256(json.dumps({k: account[k] for k in ('config_root', 'platform', 'company', 'provider_id', 'endpoint')}, sort_keys=True).encode()).hexdigest()}
                    with tempfile.TemporaryDirectory(prefix='shiju-real-acp-') as directory:
                        root = Path(directory).resolve(); os.chmod(root, 0o700)
                        try:
                            signal.alarm(120)
                            public.update(turn(binary, entries[backend], account, root, mode, upstream))
                        except Unavailable as error:
                            # Identity errors are fixed strings; HTTP errors can contain secrets.
                            from verify_acp_approval import GatewayError
                            public.update(status='gateway_rejected' if isinstance(error, GatewayError) else 'client_unavailable',
                                          gateway_http_status=error.status if isinstance(error, GatewayError) else None,
                                          reason='public gateway rejected request' if isinstance(error, GatewayError) else str(error))
                        except (OSError, TimeoutError, urllib.error.URLError, http.client.HTTPException):
                            public.update(status='transport_error', reason='Gateway/CLI transport did not complete before its bounded deadline.')
                        finally:
                            signal.alarm(0)
                    observations.append(public)
                    print(json.dumps({k: public.get(k) for k in ('provider_id', 'client', 'upstream_protocol', 'thinking_mode', 'status', 'request_count', 'connected', 'tool_write', 'tool_read', 'directory_scope') }), flush=True)
    diff = subprocess.check_output(['git', 'diff', 'HEAD', '--', 'crates', 'Cargo.toml', 'Cargo.lock'], cwd=ROOT)
    report = {'schema_version': 1, 'checked_at': datetime.now(timezone.utc).isoformat(),
              'source_revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
              'source_diff_sha256': hashlib.sha256(diff).hexdigest(),
              'production_source_sha256': {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted((ROOT / 'crates/vw-agent/src/model_relay').glob('*.rs'))},
              'inventory_sha256': hashlib.sha256(args.inventory.read_bytes()).hexdigest(),
              'source_records': len(inventory['records']), 'cargo_target_dir': slot,
              'gateway_binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'scope': 'Real installed clients via /v1/chat/stream, session_model, isolated tenant 10000/100000000 and temporary workspace. Forwarding permits configured Chat and production-reviewed same-origin native Messages operations only. Explicit single-protocol candidates force conversion without altering production defaults. DeepSeek saved credentials without enabled models are explicit isolated diagnostic candidates. Default and explicit-none thinking are separate observations. No offline fixture counts as real-service success.',
              'observations': observations, 'summary': summarize(observations),
              'acceptance': acceptance_summary(observations)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(ensure_public(json.dumps(report, ensure_ascii=False, indent=2) + '\n',
                                       [a['credential'] for a in accounts]))
    print(json.dumps(report['summary']), flush=True)
    return 0 if all(o['status'] in ('passed', 'missing_credentials', 'client_unavailable') for o in observations) else 1
