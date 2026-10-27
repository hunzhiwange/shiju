"""Offline by default; two durable, opt-in synthetic readiness probes.

Set VW_SUITE_PROFILE to an absolute mode-0600 suite profile. Live evidence is
separate from official scores. A reserved case is never automatically replayed.
"""
import json
import os
from pathlib import Path
import tempfile
import unittest

os.environ['LITELLM_LOCAL_MODEL_COST_MAP'] = 'True'


class IntegrationTests(unittest.TestCase):
    def test_stop_requires_terminal_usage_and_expected_answer(self):
        raw = stream('stop', '437')
        facts = assess(raw, 0, '437')
        self.assertTrue(facts['passed'])
        self.assertEqual(facts['usage']['output_tokens'], 1)

    def test_length_is_failed_even_with_correct_answer(self):
        self.assertFalse(assess(stream('length', '437'), 0, '437')['passed'])

    def test_connection_interruption_does_not_infer_success(self):
        raw = stream('stop', '437').split('{"type": "stream_end"')[0]
        self.assertFalse(assess(raw, 0, '437')['passed'])

    def test_missing_usage_or_wrong_answer_fails(self):
        self.assertFalse(assess(stream('stop', '436'), 0, '437')['passed'])
        raw = stream('stop', '437').replace('"input_tokens": 3, "output_tokens": 1', '')
        self.assertFalse(assess(raw, 0, '437')['passed'])

    def test_secrets_are_redacted_recursively(self):
        value = {'nested': [{'error': 'Bearer fixture-secret', 'api_key': 'other'}]}
        result = redact(value, 'fixture-secret')
        self.assertNotIn('fixture-secret', json.dumps(result))
        self.assertNotIn('other', json.dumps(result))

    def test_reservation_rejects_replay_without_rewriting(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'text.json'
            reserve_case(path, 'text')
            before = path.read_bytes()
            with self.assertRaises(FileExistsError):
                reserve_case(path, 'text')
            self.assertEqual(path.read_bytes(), before)

    def test_synthetic_image_has_real_png_bytes_without_answer_in_prompt(self):
        request, expected, image = synthetic('image')
        self.assertTrue(image.startswith(b'\x89PNG\r\n\x1a\n'))
        content = request['messages'][0]['content']
        self.assertIn('[IMAGE:data:image/png;base64,', content)
        self.assertNotIn(expected, content.split('[IMAGE:')[0])

    def test_outbound_drift_and_text_substitution_are_rejected(self):
        body = {'model': 'qwen3.8-flash', 'max_completion_tokens': 32000, 'messages': []}
        validate_outbound(body, 'text', b'')
        for field, value in (('model', 'other'), ('max_completion_tokens', 4096), ('max_completion_tokens', 32000.0)):
            with self.assertRaises(ValueError):
                validate_outbound({**body, field: value}, 'text', b'')
        with self.assertRaises(ValueError):
            validate_outbound(body, 'image', b'expected-image')
        image = b'fixture-image'
        part = {'type': 'image_url', 'image_url': {'url': 'data:image/png;base64,' +
                                                 base64.b64encode(image).decode()}}
        body['messages'] = [{'role': 'user', 'content': [part]}]
        self.assertEqual(validate_outbound(body, 'image', image), sha(image))
        with self.assertRaises(ValueError):
            validate_outbound(body, 'image', b'different-image')

    def test_mock_http_start_stop_and_interruption(self):
        exercise_http(self)


class EvidenceRecoveryTests(unittest.TestCase):
    def test_frozen_reasoning_frames_do_not_change_expected_answer(self):
        rows = [json.loads(line) for line in stream('stop', '437').splitlines()]
        rows[:1] = [{'event': {'type': 'chat.delta', 'delta': delta}}
                    for delta in ('<think>plan', '</think>\n\n', '437')]
        facts = assess('\n'.join(json.dumps(row) for row in rows), 0, '437')
        self.assertTrue(facts['passed'])
        self.assertEqual(facts['reasoning'], 'plan')
        self.assertEqual(facts['answer'], '437')
        rows[2]['event']['delta'] = '436'
        self.assertFalse(assess('\n'.join(json.dumps(row) for row in rows), 0, '437')['passed'])

    def test_reasoning_only_and_literal_content_cannot_supply_answer(self):
        rows = [json.loads(line) for line in stream('stop', '').splitlines()]
        rows[:1] = [{'event': {'type': 'chat.delta', 'delta': delta}}
                    for delta in ('<think>437', '</think>\n\n')]
        self.assertFalse(assess('\n'.join(json.dumps(row) for row in rows), 0, '437')['passed'])
        self.assertFalse(assess(stream('stop', '<think>plan</think>437'), 0, '437')['passed'])

    def test_upstream_usage_missing_zero_and_nonzero_are_distinct(self):
        self.assertEqual(observe_upstream(None)['usage_status'], 'not_captured')
        for usage, status in ((None, 'missing'),
                              ({'prompt_tokens': 0, 'completion_tokens': 0}, 'reported_zero'),
                              ({'prompt_tokens': 3, 'completion_tokens': 1}, 'reported_nonzero')):
            raw = 'data: ' + json.dumps({'choices': [], 'usage': usage}) + '\n\ndata: [DONE]\n\n'
            self.assertEqual(observe_upstream(raw)['usage_status'], status)

    def test_usage_only_trailer_recovers_counts_without_rewriting_gateway(self):
        wire = stream('stop', '437').replace('"input_tokens": 3, "output_tokens": 1',
                                             '"input_tokens": 0, "output_tokens": 0')
        trailer = {'model': 'qwen3.8-flash', 'choices': [], 'usage': {
            'prompt_tokens': 11, 'completion_tokens': 2,
            'prompt_tokens_details': {'cached_tokens': 3},
            'completion_tokens_details': {'reasoning_tokens': 1}}}
        raw = 'data: ' + json.dumps(trailer) + '\n\ndata: [DONE]\n\n'
        facts = assess(wire, 0, '437', upstream=raw, upstream_complete=True)
        self.assertTrue(facts['passed'])
        self.assertEqual(facts['usage_source'], 'provider_sse')
        self.assertEqual(facts['usage'], {'input_tokens': 11, 'output_tokens': 2,
                                         'cached_tokens': 3, 'reasoning_tokens': 1})
        self.assertEqual(facts['reported_usage']['input_tokens'], 0)
        self.assertIsNone(facts['gateway_usage'])

    def test_incomplete_or_invalid_provider_usage_cannot_pass(self):
        wire = stream('stop', '437').replace('"input_tokens": 3, "output_tokens": 1',
                                             '"input_tokens": 0, "output_tokens": 0')
        for model, usage, complete, done in (
                ('other', {'prompt_tokens': 11, 'completion_tokens': 2}, True, True),
                ('qwen3.8-flash', {'prompt_tokens': True, 'completion_tokens': 2}, True, True),
                ('qwen3.8-flash', {'prompt_tokens': -1, 'completion_tokens': 2}, True, True),
                ('qwen3.8-flash', {'prompt_tokens': 11, 'completion_tokens': 2,
                                    'completion_tokens_details': {'reasoning_tokens': 3}}, True, True),
                ('qwen3.8-flash', {'prompt_tokens': 11, 'completion_tokens': 2}, False, True),
                ('qwen3.8-flash', {'prompt_tokens': 11, 'completion_tokens': 2}, True, False)):
            raw = 'data: ' + json.dumps({'model': model, 'choices': [], 'usage': usage}) + '\n\n'
            if done:
                raw += 'data: [DONE]\n\n'
            self.assertFalse(assess(wire, 0, '437', upstream=raw,
                                    upstream_complete=complete)['passed'])

    def test_raw_http_capture_preserves_usage_and_separate_content(self):
        exercise_capture(self)


@unittest.skipUnless(os.environ.get('VW_SUITE_LIVE_READINESS') == '1',
                     'live readiness requires explicit opt-in')
class LiveReadinessTests(unittest.TestCase):
    def test_1_synthetic_text(self):
        self.assertTrue(live_case('text')['passed'], 'see readiness.json; no automatic replay')

    def test_2_synthetic_image(self):
        self.assertTrue(live_case('image')['passed'], 'see readiness.json; no automatic replay')


# The readiness harness stays here: it is not an official scorer or runtime API.
import base64
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import socket
import struct
import subprocess
import threading
import time
import urllib.error
import urllib.request
import uuid
import zlib

from suite.agent import LIMIT, MODEL, SuiteAgent, _validate_profile
from suite.runtime import verify_runtime
from suite.state import GLOBAL_LOCK, atomic_json, locked

BASE = Path(__file__).resolve().parents[4]
OUTPUT = BASE / 'evals/promptfoo/results/qwen38-suite'
RUNTIME = OUTPUT / 'runtime-v2'
IMAGE = 'sha256:929a6d631b592c82e0d0f5d4a65cf03c946dc554cfc9bb09916e82076e7d3215'
HARNESS_SOURCE = Path(__file__).read_bytes()


def sha(value):
    return hashlib.sha256(value).hexdigest()


def redact(value, secret):
    if isinstance(value, dict):
        return {key: '[REDACTED]' if key.lower() in
                ('key', 'api_key', 'authorization', 'auth', 'access_token')
                else redact(item, secret) for key, item in value.items()}
    if isinstance(value, list):
        return [redact(item, secret) for item in value]
    if isinstance(value, str) and secret:
        return value.replace(secret, '[REDACTED]')
    return value


def stream(reason, answer):
    return '\n'.join(json.dumps(row) for row in [
        {'event': {'type': 'chat.delta', 'delta': answer}},
        {'event': {'type': 'chat.done', 'finish_reason': reason,
                   'usage': {'input_tokens': 3, 'output_tokens': 1}}},
        {'type': 'stream_end', 'status': 'terminated'}])


def assess(raw, code, expected, *, upstream=None, upstream_complete=False):
    facts = SuiteAgent.summarize_stream(raw, code)
    events = [json.loads(line).get('event', {}) for line in raw.splitlines() if line.strip()]
    answer, reasoning, framed = gateway_channels(events)
    gateway_usage = facts['usage']
    usage, source = gateway_usage, 'gateway' if gateway_usage is not None else 'unavailable'
    observed = observe_upstream(upstream)
    if (usage is None and upstream_complete and observed.get('done_seen')
            and not observed.get('parse_error') and observed.get('models') == ['qwen3.8-flash']
            and observed['usage_status'] == 'reported_nonzero'):
        usage = provider_usage(observed['usage'])
        if usage is not None:
            source = 'provider_sse'
    facts.update(usage=usage, usage_missing=usage is None, gateway_usage=gateway_usage,
                 usage_source=source, answer=answer, reasoning=reasoning, reasoning_frames_complete=framed,
                 passed=(framed and facts['status'] == 'completed' and answer == expected
                 and isinstance(usage, dict) and usage.get('input_tokens', 0) > 0
                 and usage.get('output_tokens', 0) > 0))
    return facts


def provider_usage(value):
    """Normalize only counters actually supplied by the measured provider SSE."""
    mapped = {'input_tokens': value.get('prompt_tokens'),
              'output_tokens': value.get('completion_tokens')}
    if any(type(v) is not int or v <= 0 for v in mapped.values()):
        return None
    for detail, field, output in (('prompt_tokens_details', 'cached_tokens', 'cached_tokens'),
                                  ('completion_tokens_details', 'reasoning_tokens', 'reasoning_tokens')):
        details = value.get(detail)
        if details is not None:
            if not isinstance(details, dict):
                return None
            if field in details:
                count = details[field]
                maximum = mapped['input_tokens' if output == 'cached_tokens' else 'output_tokens']
                if type(count) is not int or not 0 <= count <= maximum:
                    return None
                mapped[output] = count
    return mapped


def gateway_channels(events):
    """Decode the frozen llm_runner's framing, without rewriting content tags.

    llm_runner.rs emits a standalone closing frame before ordinary answer text.
    A single content delta containing literal think tags is not such a frame.
    """
    answer, reasoning, opened = [], [], False
    for event in events:
        if event.get('type') != 'chat.delta':
            continue
        delta = event.get('delta', '')
        if opened:
            if delta == '</think>\n\n':
                opened = False
            else:
                reasoning.append(delta)
        elif not answer and delta.startswith('<think>') and '</think>' not in delta:
            reasoning.append(delta[len('<think>'):])
            opened = True
        elif delta.startswith('\n\n<think>') and delta.endswith('</think>\n\n'):
            reasoning.append(delta[len('\n\n<think>'):-len('</think>\n\n')])
        else:
            answer.append(delta)
    return ''.join(answer).strip(), ''.join(reasoning).strip(), not opened


def observe_upstream(raw):
    """Observe provider fields; absent telemetry never becomes measured zero."""
    result = {'usage_status': 'not_captured', 'usage': None, 'content': '',
              'reasoning_content': '', 'models': [], 'done_seen': False}
    if raw is None:
        return result
    result['usage_status'] = 'missing'
    content, reasoning, usages = [], [], []
    for block in raw.replace('\r\n', '\n').split('\n\n'):
        data = '\n'.join(line[5:].lstrip() for line in block.splitlines() if line.startswith('data:'))
        if data == '[DONE]':
            result['done_seen'] = True
            continue
        if not data:
            continue
        try:
            record = json.loads(data)
        except ValueError:
            result['parse_error'] = True
            continue
        if not isinstance(record, dict):
            result['parse_error'] = True
            continue
        if record.get('model') and record['model'] not in result['models']:
            result['models'].append(record['model'])
        if isinstance(record.get('usage'), dict):
            usages.append(record['usage'])
        choices = record.get('choices') or []
        if not isinstance(choices, list):
            result['parse_error'] = True
            continue
        for choice in choices:
            if not isinstance(choice, dict):
                result['parse_error'] = True
                continue
            delta = choice.get('delta') or {}
            if not isinstance(delta, dict):
                result['parse_error'] = True
                continue
            if isinstance(delta.get('content'), str):
                content.append(delta['content'])
            if isinstance(delta.get('reasoning_content'), str):
                reasoning.append(delta['reasoning_content'])
    result.update(content=''.join(content), reasoning_content=''.join(reasoning),
                  usage_observations=usages)
    if usages:
        result['usage'] = usages[-1]
        counts = [v for k, v in usages[-1].items() if k in
                  ('prompt_tokens', 'completion_tokens', 'input_tokens', 'output_tokens')]
        result['usage_status'] = ('invalid' if not counts or any(type(v) is not int or v < 0 for v in counts)
                                  else 'reported_nonzero' if any(counts) else 'reported_zero')
    return result


def relay_response(response, writer, row, secret):
    """Capture the exact provider SSE before Gateway parsing, including partials."""
    payload, complete = bytearray(), False
    try:
        while True:
            data = response.read1(65536)
            if not data:
                complete = True
                break
            payload.extend(data)
            writer.write(data)
            writer.flush()
    finally:
        archived = redact(bytes(payload).decode('utf-8', errors='replace'), secret)
        row.update(upstream_http_status=response.status,
                   upstream_content_type=response.headers.get('Content-Type'),
                   upstream_response_sse=archived, upstream_response_sha256=sha(payload),
                   upstream_response_complete=complete)
        row['upstream_observation'] = observe_upstream(archived)


def exercise_capture(test):
    import io
    records = [{'model': 'qwen3.8-flash', 'choices': [{'delta': {'reasoning_content': 'fixture-secret 思考'}}]},
               {'choices': [{'delta': {'content': '437'}}]},
               {'choices': [], 'usage': {'prompt_tokens': 3, 'completion_tokens': 1}}]
    payload = (''.join('data: ' + json.dumps(r, ensure_ascii=False) + '\n\n' for r in records)
               + 'data: [DONE]\n\n').encode()

    class Boundary(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_GET(self):
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.end_headers()
            cut = payload.index('思'.encode()) + 1
            for chunk in (payload[:cut], payload[cut:]):
                self.wfile.write(chunk)
                self.wfile.flush()
    server = ThreadingHTTPServer(('127.0.0.1', 0), Boundary)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        row, sink = {}, io.BytesIO()
        with urllib.request.urlopen('http://127.0.0.1:' + str(server.server_port), timeout=2) as response:
            relay_response(response, sink, row, 'fixture-secret')
        test.assertEqual(sink.getvalue(), payload)
        test.assertEqual(row['upstream_response_sha256'], sha(payload))
        test.assertTrue(row['upstream_response_complete'])
        test.assertNotIn('fixture-secret', json.dumps(row))
        test.assertEqual(row['upstream_observation']['content'], '437')
        test.assertEqual(row['upstream_observation']['reasoning_content'], '[REDACTED] 思考')
        test.assertEqual(row['upstream_observation']['usage_status'], 'reported_nonzero')
        test.assertEqual(row['upstream_observation']['usage']['completion_tokens'], 1)
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


def reserve_case(path, case):
    path.parent.mkdir(parents=True, exist_ok=True)
    # O_EXCL records the attempt before any uncertain dispatch; no automatic replay.
    with path.open('x') as file:
        json.dump({'kind': 'readiness', 'case': case, 'state': 'reserved',
                   'model_requests': 0, 'passed': False}, file)


def synthetic(case):
    image = b''
    if case == 'text':
        content, expected = 'Compute 19 * 23. Reply with only the integer. Do not use tools.', '437'
    elif case == 'image':
        def chunk(kind, data):
            return struct.pack('!I', len(data)) + kind + data + struct.pack('!I', zlib.crc32(kind + data))
        # One solid primary-color square; no label, filename or textual answer leak.
        image = (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('!2I5B', 64, 64, 8, 2, 0, 0, 0))
                 + chunk(b'IDAT', zlib.compress((b'\0' + bytes((255, 0, 0)) * 64) * 64))
                 + chunk(b'IEND', b''))
        # The frozen session converter preserves image markers in string content;
        # its text projection drops image_url array parts before provider dispatch.
        content = ('Name the dominant color in the attached image. '
                   'Reply with only one lowercase English color word. Do not use tools.\n'
                   '[IMAGE:data:image/png;base64,' + base64.b64encode(image).decode() + ']')
        expected = 'red'
    else:
        raise ValueError('unknown readiness case')
    return {'session_id': uuid.uuid4().hex, 'model': MODEL, 'allowed_tools': [],
            'messages': [{'role': 'user', 'content': content}]}, expected, image


def validate_outbound(body, case, image):
    limit = body.get('max_completion_tokens', body.get('max_tokens'))
    if body.get('model') != 'qwen3.8-flash' or type(limit) is not int or limit != 32000:
        raise ValueError('frozen model or output limit mismatch')
    parts = [part for message in body.get('messages', [])
             if isinstance(message.get('content'), list) for part in message['content']
             if part.get('type') == 'image_url']
    if case == 'image':
        try:
            actual = [base64.b64decode(part['image_url']['url'].split(',', 1)[1], validate=True)
                      for part in parts]
        except (KeyError, IndexError, ValueError, TypeError):
            raise ValueError('invalid actual image payload') from None
        if actual != [image]:
            raise ValueError('actual image payload differs or is missing')
    return sha(image) if case == 'image' else None


def exercise_http(test):
    class Boundary(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_GET(self):
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'{"status":"ok"}')

        def do_POST(self):
            self.server.calls.append(self.path)
            self.rfile.read(int(self.headers.get('Content-Length', 0)))
            if self.path == '/interrupt':
                self.connection.shutdown(socket.SHUT_RDWR)
                self.connection.close()
                return
            self.send_response(200)
            self.end_headers()
            self.wfile.write(stream('length' if self.path == '/length' else 'stop', '437').encode())
    server = ThreadingHTTPServer(('127.0.0.1', 0), Boundary)
    server.calls = []
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    address = 'http://127.0.0.1:' + str(server.server_port)
    try:
        with urllib.request.urlopen(address, timeout=2) as response:
            test.assertEqual(json.load(response), {'status': 'ok'})
        for path in ('/stop', '/length', '/interrupt'):
            request = urllib.request.Request(address + path, b'{}')
            if path == '/interrupt':
                with test.assertRaises((OSError, __import__('http.client').client.RemoteDisconnected)):
                    urllib.request.urlopen(request, timeout=2)
            else:
                with urllib.request.urlopen(request, timeout=2) as response:
                    facts = assess(response.read().decode(), 0, '437')
                test.assertEqual(facts['passed'], path == '/stop')
        test.assertEqual(server.calls, ['/stop', '/length', '/interrupt'])
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


def command(args, *, timeout=60):
    result = subprocess.run(args, capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        # Do not leak Docker/runtime diagnostics or credential-bearing response text.
        raise RuntimeError('subprocess failed: ' + args[0] + ' exit ' + str(result.returncode))
    return result.stdout.strip()


def api(address, path, body=None, method=None):
    if address.startswith('docker://'):
        # Keep Gateway loopback-only; credential JSON travels over stdin, never argv.
        args = ['docker', 'exec', '-i', address[9:], 'curl', '--silent', '--show-error',
                '--fail', '--max-time', '10', '-H', 'Content-Type: application/json',
                '-H', 'X-Platform-ID: 10000', '-H', 'X-Company-ID: 100000000',
                '-X', method or ('GET' if body is None else 'POST')]
        if body is not None:
            args += ['--data-binary', '@-']
        args += ['http://127.0.0.1:42619' + path]
        result = subprocess.run(args, input='' if body is None else json.dumps(body),
                                capture_output=True, text=True, timeout=15)
        if result.returncode:
            raise OSError('container loopback HTTP failed')
        return json.loads(result.stdout)
    request = urllib.request.Request(address + path,
              None if body is None else json.dumps(body).encode(),
              {'Content-Type': 'application/json', 'X-Platform-ID': '10000',
               'X-Company-ID': '100000000'}, method=method)
    with urllib.request.urlopen(request, timeout=10) as response:
        return json.load(response)


def live_case(case):
    receipt = OUTPUT / 'readiness' / (case + '.json')
    # Use the suite-wide model lease, so readiness cannot overlap official work.
    with locked(GLOBAL_LOCK):
        reserve_case(receipt, case)
        row = json.loads(receipt.read_text())
        container, proxy, thread = None, None, None
        secret = ''
        try:
            profile_path = os.environ.get('VW_SUITE_PROFILE')
            if not profile_path or not Path(profile_path).is_absolute():
                raise ValueError('VW_SUITE_PROFILE requires an absolute temporary 0600 profile')
            profile = _validate_profile(profile_path)
            secret = profile['auth']['key']
            row['runtime'] = verify_runtime(RUNTIME)
            row['source_sha256'] = sha(HARNESS_SOURCE)
            receipt.with_name(case + '-source.py').write_bytes(HARNESS_SOURCE)
            request, expected, image = synthetic(case)
            row.update(kind='readiness', official_score=None, model=MODEL,
                       max_output_tokens=32000, input_window_tokens=200000,
                       client_request=request, request_sha256=sha(json.dumps(request).encode()),
                       image_sha256=sha(image) if image else None)
            upstream, route = profile['provider']['api'].rstrip('/'), '/' + uuid.uuid4().hex
            row['provider_sha256'] = sha(json.dumps(profile['provider'], sort_keys=True).encode())
            calls = []
            call_lock = threading.Lock()

            class Forward(BaseHTTPRequestHandler):
                def log_message(self, *args):
                    pass

                def do_POST(self):
                    if self.path != route + '/v1/chat/completions' or self.headers.get('Authorization') != 'Bearer ' + secret:
                        self.send_error(403)
                        return
                    payload = self.rfile.read(int(self.headers.get('Content-Length', 0)))
                    body = json.loads(payload)
                    with call_lock:
                        if calls:
                            self.send_error(429, 'readiness single-request budget exhausted')
                            return
                        public = redact(body, secret)
                        public['request_sha256'] = sha(payload)
                        calls.append(public)
                        row.update(state='intercepted', model_requests=0, upstream_requests=calls)
                        atomic_json(receipt, redact(row, secret))
                    # Reject drift BEFORE touching the remote model. No retry at this boundary.
                    try:
                        row['upstream_image_sha256'] = validate_outbound(body, case, image)
                    except ValueError:
                        row['parameter_or_image_rejected_before_dispatch'] = True
                        self.send_error(400, 'readiness frozen request validation failed; no replay')
                        return
                    row.update(state='dispatched', model_requests=1)
                    atomic_json(receipt, redact(row, secret))
                    target = upstream + '/chat/completions'
                    outgoing = urllib.request.Request(target, payload, {
                        'Content-Type': 'application/json', 'Authorization': 'Bearer ' + secret})
                    try:
                        with urllib.request.urlopen(outgoing, timeout=120) as response:
                            self.send_response(response.status)
                            self.send_header('Content-Type', response.headers.get('Content-Type', 'text/event-stream'))
                            self.end_headers()
                            relay_response(response, self.wfile, row, secret)
                    except Exception as error:
                        row['upstream_error_type'] = type(error).__name__
                        if isinstance(error, urllib.error.HTTPError):
                            row['upstream_http_status'] = error.code
                        # Error response body can contain sensitive upstream echoes.
                        try:
                            self.send_error(502, 'readiness upstream failed; no replay')
                        except OSError:
                            pass

            proxy = ThreadingHTTPServer(('0.0.0.0', 0), Forward)
            thread = threading.Thread(target=proxy.serve_forever, daemon=True)
            thread.start()
            # Existing, pinned capacity image; no image pull and no package downloads.
            command(['docker', 'image', 'inspect', IMAGE])
            container = command(['docker', 'run', '--detach', '--rm', '--pull', 'never',
                '--platform', 'linux/amd64', '--cpus', '1', '--memory', '2g', '--pids-limit', '256',
                '--mount',
                'type=bind,src=' + str(RUNTIME / 'runtime') + ',dst=/runtime,readonly',
                '--env', 'HOME=/tmp/readiness-home', '--env', 'RUST_LOG=warn',
                '--env', 'LD_LIBRARY_PATH=/runtime', '--env', 'SSL_CERT_FILE=/runtime/ca-certificates.crt',
                '--env', 'OPENSSL_CONF=/runtime/openssl.cnf', '--entrypoint', '/runtime/shiju',
                IMAGE, '--config-dir', '/tmp/readiness-config', 'gateway', '--host', '127.0.0.1', '--port', '42619'])
            address = 'docker://' + container
            for _ in range(60):
                try:
                    health = api(address, '/v1/health')
                    break
                except (OSError, ValueError):
                    time.sleep(.1)
            else:
                raise RuntimeError('readiness Gateway startup failed')
            if health.get('build_id') != row['runtime']['binary_sha256']:
                raise ValueError('readiness health build identity mismatch')
            row['health'] = health
            provider = {**profile['provider'], 'api': 'http://host.docker.internal:' + str(proxy.server_port) + route + '/v1'}
            api(address, '/v1/config', {'providers': {'token-plan': provider},
                'reliability': {'provider_retries': 0}}, 'PATCH')
            api(address, '/v1/auth/token-plan', profile['auth'], 'PUT')
            data = api(address, '/v1/provider?directory=%2Fapp')
            model = next(p for p in data['all'] if p['id'] == 'token-plan')['models']['qwen3.8-flash']
            row['model_readback'] = {key: model.get(key) for key in ('limit', 'status', 'capabilities')}
            if model.get('limit') != LIMIT:
                raise ValueError('readiness model readback limit mismatch')
            tools = command(['docker', 'exec', container, 'bash', '-c',
                     'for tool in ffmpeg ffprobe; do command -v "$tool" || true; done'])
            row['video_tools'] = tools.splitlines()
            with tempfile.TemporaryDirectory(prefix='vw-suite-readiness-request-') as directory:
                path = Path(directory) / 'request.json'
                with open(path, 'x', opener=lambda name, flags: os.open(name, flags, 0o600)) as file:
                    json.dump(request, file)
                command(['docker', 'cp', str(path), container + ':/tmp/readiness-request.json'])
            cli = ['docker', 'exec', container, '/runtime/shiju', 'realtime', '--host', '127.0.0.1',
                   '--port', '42619', '--directory', '/app', '--platform', '10000', '--company',
                   '100000000', '--output', 'jsonl', '--yes']
            result = subprocess.run(cli + ['chat-stream', '--request-id', request['session_id'],
                     '--input', '/tmp/readiness-request.json'], capture_output=True, text=True, timeout=180)
            row.update(assess(result.stdout, result.returncode, expected,
                              upstream=row.get('upstream_response_sse'),
                              upstream_complete=row.get('upstream_response_complete', False)))
            row.update(cli_exit_code=result.returncode, state='finished', model_requests=row['model_requests'],
                       upstream_requests=calls, events=redact(result.stdout, secret),
                       stderr=redact(result.stderr, secret))
            stop = subprocess.run(cli + ['chat-stop', request['session_id']],
                                  capture_output=True, text=True, timeout=15)
            row['stop_exit_code'] = stop.returncode
            sent_images = [part for call in calls for message in call.get('messages', [])
                           if isinstance(message.get('content'), list) for part in message['content']
                           if part.get('type') == 'image_url']
            row['real_image_sent'] = bool(sent_images)
            row['passed'] = bool(row['passed'] and len(calls) == 1 and stop.returncode == 0
                                 and (case != 'image' or sent_images))
        except Exception as error:
            row.update(state='failed' if row.get('model_requests') else 'not_sent',
                       passed=False, error_type=type(error).__name__)
            # Explicit configuration failures are safe; other exception text stays private.
            if isinstance(error, RuntimeError):
                row['infrastructure_error'] = redact(str(error), secret)
            if isinstance(error, ValueError):
                row['configuration_error'] = redact(str(error), secret)
        finally:
            if container:
                cleanup = subprocess.run(['docker', 'rm', '--force', container], capture_output=True, timeout=30)
                row['container_cleanup_exit_code'] = cleanup.returncode
                if cleanup.returncode:
                    row['passed'] = False
            if proxy:
                proxy.shutdown()
                proxy.server_close()
                thread.join()
            atomic_json(receipt, redact(row, secret))
            records = [json.loads(p.read_text()) for p in sorted(receipt.parent.glob('*.json'))]
            atomic_json(OUTPUT / 'readiness.json', {'kind': 'readiness', 'official_scores': [],
                'passed': len(records) == 2 and all(r.get('passed') for r in records), 'cases': records})
        return redact(row, secret)
