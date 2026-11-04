"""Probe the real fixed adapter against a private Responses upstream; not production acceptance."""
import importlib.util, os, sys, tempfile, subprocess, json, pathlib, threading, queue, time, signal, hashlib, shutil, secrets
root_path = pathlib.Path(__file__).resolve().parents[5]
spec = importlib.util.spec_from_file_location('fixture', pathlib.Path(__file__).with_name('responses_fixture.py'))
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)
binary = str((root_path / 'target/acp-codex-inspection/runtime/node_modules/.bin/codex-acp').absolute())

class Client:

    def __init__(self, env, cwd):
        self.p = subprocess.Popen([binary] + (["cli", "--shiju-model-source"] if env.get("SHIJU_CODEX_MODEL_LOCK") else []), cwd=cwd, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, start_new_session=True)
        self.q = queue.Queue()
        self.i = 0
        self.updates = []
        self.closed = False

        def read():
            for line in self.p.stdout:
                try:
                    self.q.put(json.loads(line))
                except ValueError:
                    pass
        threading.Thread(target=read, daemon=True).start()

    def req(self, method, params, timeout=45):
        self.i += 1
        i = self.i
        self.p.stdin.write(json.dumps({'jsonrpc': '2.0', 'id': i, 'method': method, 'params': params}) + '\n')
        self.p.stdin.flush()
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            try:
                r = self.q.get(timeout=1)
            except queue.Empty:
                if self.p.poll() is not None:
                    raise RuntimeError('adapter exited ' + str(self.p.returncode))
                continue
            if 'method' in r and 'id' in r:
                if r['method'] == 'session/request_permission':
                    option = next((x for x in r['params']['options'] if x.get('kind') == 'allow_once'), r['params']['options'][0])
                    result = {'outcome': {'outcome': 'selected', 'optionId': option['optionId']}}
                else:
                    result = {}
                self.p.stdin.write(json.dumps({'jsonrpc': '2.0', 'id': r['id'], 'result': result}) + '\n')
                self.p.stdin.flush()
            elif r.get('method') == 'session/update':
                self.updates.append(r['params'])
            elif r.get('id') == i:
                if 'error' in r:
                    raise RuntimeError('ACP error ' + str(r['error'].get('code')) + ' ' + str(r['error'].get('message'))[:160])
                return r.get('result')
        raise TimeoutError(method)

    def init(self):
        return self.req('initialize', {'protocolVersion': 1, 'clientCapabilities': {}, 'clientInfo': {'name': 'codex-offline-probe', 'version': '1'}})

    def close(self):
        if self.closed:
            return
        self.closed = True
        try:
            os.killpg(self.p.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            self.p.wait(timeout=3)
        except subprocess.TimeoutExpired:
            os.killpg(self.p.pid, signal.SIGKILL)
            self.p.wait()
        self.p.stdin.close()
        self.p.stdout.close()
mode = sys.argv[1] if len(sys.argv) > 1 else 'managed'
with tempfile.TemporaryDirectory(prefix='codex13-direct-') as temp, mod.ResponsesFixture() as fixture:
    root = pathlib.Path(temp)
    native = root / 'native'
    native.mkdir(mode=448)
    cwd = root / 'project'
    cwd.mkdir()
    for name in ('one.txt', 'two.txt'):
        (cwd / name).write_text(fixture.challenge)
    fixture.read_path = str(cwd / 'one.txt')
    fixture.second_read_path = str(cwd / 'two.txt')
    config = {'model': 'acp-fixture-primary', 'review_model': 'acp-fixture-primary', 'model_context_window': 200000, 'model_provider': 'offline', 'model_providers': {'offline': {'name': 'Offline', 'base_url': fixture.url + '/v1', 'wire_api': 'responses', 'requires_openai_auth': False, 'env_key': 'SHIJU_CODEX_RELAY_TOKEN', 'request_max_retries': 0, 'stream_max_retries': 0}}}
    text = 'model_context_window = 200000\nmodel = "acp-fixture-primary"\nreview_model = "acp-fixture-primary"\nmodel_provider = "offline"\n[model_providers.offline]\nname = "Offline"\nbase_url = ' + json.dumps(fixture.url + '/v1') + '\nwire_api = "responses"\nrequires_openai_auth = false\nenv_key = "SHIJU_CODEX_RELAY_TOKEN"\nrequest_max_retries = 0\nstream_max_retries = 0\n'
    if mode.startswith('managed'):
        provider = 'shiju_session_' + hashlib.sha256(fixture.key.encode()).hexdigest()
        config['model_provider'] = provider
        config['model_providers'] = {provider: config['model_providers']['offline']}
        text = text.replace('"offline"', json.dumps(provider)).replace('[model_providers.offline]', '[model_providers.' + provider + ']')
    (native / 'config.toml').write_text(text)
    os.chmod(native / 'config.toml', 384)
    env = {k: os.environ[k] for k in ('PATH', 'HOME', 'TMPDIR') if k in os.environ}
    env.update(CODEX_HOME=str(native), SHIJU_CODEX_RELAY_TOKEN=fixture.key)
    if mode.startswith('managed'):
        history = root / 'history'
        history.mkdir(mode=0o700)
        env.update(SHIJU_CODEX_HISTORY_HOME=str(history), CODEX_CONFIG=json.dumps(config), MODEL_PROVIDER=config['model_provider'], SHIJU_CODEX_MODEL_LOCK='acp-fixture-primary', APP_SERVER_LOGS='')
        (cwd / '.codex').mkdir()
        (cwd / '.codex/config.toml').write_text('model = "forbidden-local-model"\nmodel_provider = "openai"\n')
    if mode == 'managed-missing-credential':
        env.pop('SHIJU_CODEX_RELAY_TOKEN')
    initial_token = fixture.key
    client = Client(env, str(cwd))
    result = {'mode': mode, 'evidence_kind': 'direct-real-acp-probe, not production acceptance'}
    try:
        client.init()
        session = client.req('session/new', {'cwd': str(cwd), 'mcpServers': []})
        sid = session['sessionId']
        result['new'] = True
        prompt = {'sessionId': sid, 'prompt': [{'type': 'text', 'text': 'Read one.txt and two.txt and reply.'}]}
        reply = client.req('session/prompt', prompt)
        result['first_reply'] = reply
        result['text_seen'] = 'FIXTURE_MODEL_REPLY' in json.dumps(client.updates)
        client.req('session/prompt', prompt)
        result['second_turn'] = True
        cancellation = {}

        def cancel_turn():
            try:
                cancellation['reply'] = client.req('session/prompt', {'sessionId': sid, 'prompt': [{'type': 'text', 'text': 'CANCEL_PROBE: read one.txt'}]})
            except Exception as error:
                cancellation['error'] = str(error)
        pending = threading.Thread(target=cancel_turn)
        pending.start()
        for _ in range(200):
            if any((r.get('cancel_probe') for r in fixture.records)):
                break
            time.sleep(0.05)
        client.p.stdin.write(json.dumps({'jsonrpc': '2.0', 'method': 'session/cancel', 'params': {'sessionId': sid}}) + '\n')
        client.p.stdin.flush()
        pending.join(timeout=15)
        result['cancelled'] = cancellation.get('reply', {}).get('stopReason') == 'cancelled'
        client.close()
        if mode.startswith('managed'):
            shutil.rmtree(native)
            fixture.key = secrets.token_hex(24)
            replacement = 'shiju_session_' + hashlib.sha256(fixture.key.encode()).hexdigest()
            config['model_providers'] = {replacement: config['model_providers'][provider]}
            config['model_provider'] = replacement
            text = text.replace(provider, replacement)
            provider = replacement
            env.update(SHIJU_CODEX_RELAY_TOKEN=fixture.key, MODEL_PROVIDER=provider, CODEX_CONFIG=json.dumps(config))
            result['lease_rotated'] = initial_token != fixture.key
            renewed = root / 'renewed-runtime'
            renewed.mkdir(mode=0o700)
            (renewed / 'config.toml').write_text(text)
            os.chmod(renewed / 'config.toml', 0o600)
            env['CODEX_HOME'] = str(renewed)
            result['runtime_rotated'] = True
        client = Client(env, str(cwd))
        client.init()
        client.req('session/load', {'sessionId': sid, 'cwd': str(cwd), 'mcpServers': []})
        client.req('session/prompt', prompt)
        result['restored'] = True
        if mode == 'managed':
            client.close()
            switched_home = root / 'switched-native'
            switched_home.mkdir(mode=0o700)
            (switched_home / 'config.toml').write_text(text.replace('acp-fixture-primary', 'acp-fixture-switched'))
            os.chmod(switched_home / 'config.toml', 0o600)
            switched_history = root / 'switched-history'
            switched_history.mkdir(mode=0o700)
            config['model'] = config['review_model'] = 'acp-fixture-switched'
            env.update(CODEX_HOME=str(switched_home), SHIJU_CODEX_HISTORY_HOME=str(switched_history), CODEX_CONFIG=json.dumps(config), SHIJU_CODEX_MODEL_LOCK='acp-fixture-switched')
            client = Client(env, str(cwd))
            client.init()
            switched = client.req('session/new', {'cwd': str(cwd), 'mcpServers': []})
            client.req('session/prompt', {'sessionId': switched['sessionId'], 'prompt': [{'type': 'text', 'text': 'Read one.txt and two.txt and reply.'}]})
            for _ in range(100):
                if any(r['model'] == 'acp-fixture-switched' and r['auxiliary'] for r in fixture.records):
                    break
                time.sleep(0.05)
            result['model_switched'] = switched['sessionId'] != sid and 'FIXTURE_MODEL_REPLY' in json.dumps(client.updates) and any(r['model'] == 'acp-fixture-switched' and r['auxiliary'] for r in fixture.records)
    except Exception as e:
        result['error'] = str(e)
    finally:
        client.close()
    if mode == 'managed':
        result['history_contains_no_token'] = all(initial_token.encode() not in path.read_bytes() and fixture.key.encode() not in path.read_bytes() for path in history.rglob('*') if path.is_file())
    result.update(requests=fixture.records, errors=fixture.errors)
    if mode == 'managed-missing-credential':
        result['probe_passed'] = bool(result.get('error')) and (not fixture.records)
    else:
        result['probe_passed'] = all((result.get(k) is True for k in ('new', 'text_seen', 'second_turn', 'cancelled', 'restored'))) and (not result.get('error')) and (fixture.errors == 0) and bool(fixture.records) and all((r['model'] in ('acp-fixture-primary', 'acp-fixture-switched') and r['authenticated'] for r in fixture.records)) and ({r['model'] for r in fixture.records} == ({'acp-fixture-primary', 'acp-fixture-switched'} if mode == 'managed' else {'acp-fixture-primary'})) and (mode != 'managed' or (result.get('model_switched') is True and result.get('runtime_rotated') is True and result.get('lease_rotated') is True and result.get('history_contains_no_token') is True)) and any((r['tool_results'] >= 2 for r in fixture.records)) and any((r['auxiliary'] for r in fixture.records))
    out = root_path / 'target/acp-model-source-evidence/codex'
    out.mkdir(parents=True, exist_ok=True)
    (out / ('direct-' + mode + '.json')).write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result))
raise SystemExit(0 if result['probe_passed'] else 1)
