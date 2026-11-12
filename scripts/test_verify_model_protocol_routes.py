"""Service audit contract: never promote HTTP success or offline evidence."""
import json
import os
import sqlite3
from pathlib import Path
import tempfile
import unittest
from unittest.mock import MagicMock, patch

import verify_model_protocol_routes as audit


def result(fields=None):
    return {'kind': 'model', 'providerId': 'saved', 'modelId': 'model',
            'report': {'source': 'real_service', 'outcome': {'status': 'succeeded',
                'evidence': {'httpStatus': 200, 'actualModelId': 'model', 'route': {
                    'sourceProtocol': 'messages', 'upstreamProtocol': 'chat',
                    'endpointFingerprint': 'a' * 64, 'checkedAt': 1,
                    'verifiedFields': fields if fields is not None else ['text', 'terminal_status', 'model']}}}}}


class ServiceAuditTests(unittest.TestCase):
    def test_acp_prefers_verified_native_account_when_other_modes_are_rejected(self):
        from model_protocol_acp import working_account_bindings
        native = {'kind': 'model', 'source_protocol': 'chat', 'upstream_protocol': 'chat', 'status': 'passed'}
        rows = [{'provider_id': 'fixture', 'model_api_id': 'working', 'status': 'observed_with_failures', 'attempts': [native]},
                {'provider_id': 'fixture', 'model_api_id': 'denied', 'status': 'access_denied', 'attempts': [dict(native, status='unauthorized')]},
                {'provider_id': 'fixture', 'model_api_id': 'converted-only', 'status': 'observed_with_failures', 'attempts': [dict(native, source_protocol='messages')]}]
        self.assertEqual(working_account_bindings(rows), {('fixture', 'working')})

    def test_acp_result_requires_binding_and_tool_output(self):
        evidence = {'connected': True, 'terminal': 'chat.done', 'errors': False,
                    'binding_verified': True, 'tool_write': True, 'tool_read': True,
                    'file_sha256': 'a' * 64, 'expected_sha256': 'a' * 64,
                    'directory_scope': True, 'http_statuses': [200, 200],
                    'upstream_models': ['wire'], 'expected_model': 'wire', 'upstream_terminal': True, 'stream_requests': 1}
        self.assertEqual(audit.acp_result(evidence), 'passed')
        for key in ('connected', 'binding_verified', 'tool_write', 'tool_read', 'directory_scope', 'upstream_terminal'):
            self.assertNotEqual(audit.acp_result(dict(evidence, **{key: False})), 'passed')
        for changes in ({'file_sha256': None}, {'terminal': None}, {'errors': True},
                        {'http_statuses': []}, {'upstream_models': ['other']}):
            self.assertNotEqual(audit.acp_result(dict(evidence, **changes)), 'passed')

    def test_acp_upstream_requires_finish_and_complete_sse(self):
        from model_protocol_acp import upstream_terminal
        row = {'stream': True, 'response': b'data: {"choices":[{"finish_reason":"stop"}]}\n\ndata: [DONE]\n\n'}
        self.assertTrue(upstream_terminal([row]))
        self.assertFalse(upstream_terminal([dict(row, response=row['response'].replace(b'data: [DONE]', b''))]))
        self.assertFalse(upstream_terminal([]))

    def test_acp_file_scope_rejects_outside_and_command_tools(self):
        from model_protocol_acp import tool_scope
        def rows(name, arguments):
            return [{'messages': [{'tool_calls': [{'function': {'name': name, 'arguments': json.dumps(arguments)}}]}]}]
        root = Path('/private/fixture/workspace')
        self.assertTrue(tool_scope(rows('write_file', {'path': 'acp-proof.txt'}), root))
        self.assertFalse(tool_scope(rows('write_file', {'path': '../outside.txt'}), root))
        self.assertFalse(tool_scope(rows('read_file', {'file_path': '/etc/passwd'}), root))
        self.assertFalse(tool_scope(rows('shell', {'command': 'pwd'}), root))
        self.assertFalse(tool_scope(rows('write_file', []), root))

    def test_acp_tools_require_correlated_write_and_read_results(self):
        from model_protocol_acp import wire_tools
        messages = [
            {'role': 'assistant', 'tool_calls': [{'id': 'w', 'function': {'name': 'write_file', 'arguments': '{"path":"acp-proof.txt","content":"challenge"}'}},
                                                {'id': 'r', 'function': {'name': 'read_file', 'arguments': '{"path":"acp-proof.txt"}'}}]},
            {'role': 'tool', 'tool_call_id': 'w', 'content': 'written'},
            {'role': 'tool', 'tool_call_id': 'r', 'content': 'challenge'}]
        self.assertEqual(wire_tools([{'messages': messages}], 'acp-proof.txt', 'challenge'), (True, True))
        self.assertEqual(wire_tools([{'messages': messages[:1]}], 'acp-proof.txt', 'challenge'), (False, False))
        messages[-1]['tool_call_id'] = 'unrelated'
        self.assertEqual(wire_tools([{'messages': messages}], 'acp-proof.txt', 'challenge'), (True, False))

    def test_acp_equivalent_commands_require_exact_allowlist_and_results(self):
        from model_protocol_acp import tool_scope, wire_tools
        command = 'python3 -c \'print("acp-proof.txt challenge")\''
        messages = [{'tool_calls': [{'id': 'w', 'function': {'name': 'exec_command', 'arguments': json.dumps({'cmd': command})}}]},
                    {'role': 'tool', 'tool_call_id': 'w', 'content': 'completed'}]
        rows = [{'messages': messages}]
        self.assertTrue(tool_scope(rows, Path('/workspace'), (command,)))
        self.assertFalse(tool_scope(rows, Path('/workspace'), (command + '; rm -rf /',)))
        self.assertEqual(wire_tools(rows, 'acp-proof.txt', 'challenge', (command,)), (True, False))

    def test_acp_native_messages_uses_only_reviewed_same_origin_pairs(self):
        from model_protocol_acp import native_messages_endpoint, upstream_terminal
        account = {'endpoint': 'https://api.deepseek.com', 'model_api_id': 'deepseek-v4-pro'}
        self.assertEqual(native_messages_endpoint(account), 'https://api.deepseek.com/anthropic/v1/messages')
        self.assertIsNone(native_messages_endpoint(dict(account, endpoint='https://example.com')))
        body = b'data: {"type":"message_delta","delta":{"stop_reason":"end_turn"}}\n\ndata: {"type":"message_stop"}\n\n'
        self.assertTrue(upstream_terminal([{'stream': True, 'response': body}], 'messages'))
        self.assertTrue(upstream_terminal([{'stream': True, 'response': body.replace(b'data: ', b'data:')}], 'messages'))
        self.assertFalse(upstream_terminal([{'stream': True, 'response': body.split(b'data: {"type":"message_stop"}')[0]}], 'messages'))

    def test_acp_failure_preserves_upstream_failure_category(self):
        for code, expected in [(401, 'unauthorized'), (402, 'insufficient_balance'), (403, 'forbidden'), (429, 'rate_limited'), (500, 'upstream_server_error')]:
            self.assertEqual(audit.acp_result({'http_statuses': [code]}), expected)

    def test_acp_pair_requires_same_account_model_and_successful_tools(self):
        from model_protocol_acp import acceptance_summary
        rows = [{'provider_id': 'deepseek', 'model_api_id': 'wire', 'account_binding': 'account',
                 'status': 'passed', 'route_mode': 'native', 'source_protocol': 'chat'},
                {'provider_id': 'deepseek', 'model_api_id': 'wire', 'account_binding': 'account',
                 'status': 'passed', 'route_mode': 'forced_conversion', 'source_protocol': 'responses'}]
        self.assertTrue(acceptance_summary(rows)['provider_pairs']['deepseek'])
        for field in ('model_api_id', 'account_binding', 'status'):
            changed = [rows[0], dict(rows[1], **{field: 'different'})]
            self.assertFalse(acceptance_summary(changed)['provider_pairs']['deepseek'])

    def test_cli_has_acp_inventory_and_output(self):
        args = audit.parser().parse_args(['acp', '--inventory', 'i.json', '--output', 'o.json'])
        self.assertEqual(args.command, 'acp')

    def test_service_result_requires_content_and_terminal(self):
        for fields in [[], ['text'], ['terminal_status'], ['model', 'text']]:
            self.assertEqual(audit.service_result(result(fields))['status'], 'invalid_response')
        self.assertEqual(audit.service_result(result())['status'], 'passed')

    def test_offline_evidence_cannot_be_service_success(self):
        value = result(); value['report']['source'] = 'controlled_fixture'
        self.assertEqual(audit.service_result(value)['status'], 'invalid_response')

    def test_failure_classification_retains_http_status(self):
        for code, status in [('unauthorized', 401), ('rate_limited', 429), ('timeout', None), ('unexpected_http_status', 404)]:
            value = result(); value['report']['outcome'] = {'status': 'failed', 'error': {'code': code, 'phase': 'response_headers'}, 'httpStatus': status}
            actual = audit.service_result(value)
            self.assertEqual(actual['status'], code); self.assertEqual(actual['http_status'], status)

    def test_success_requires_bound_model_and_endpoint(self):
        for field in ['actualModelId', 'route']:
            value = result(); del value['report']['outcome']['evidence'][field]
            self.assertEqual(audit.service_result(value)['status'], 'invalid_response')

    def test_raw_response_files_are_private_and_exclusive(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'raw.json'
            audit.private_json(path, {'secret': 'private'})
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            with self.assertRaises(FileExistsError): audit.private_json(path, {})

    def test_concurrent_audits_are_rejected_and_lock_is_released(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'audit.lock'
            with audit.exclusive(path):
                with self.assertRaises(RuntimeError):
                    with audit.exclusive(path): pass
            with audit.exclusive(path): pass
            self.assertFalse(path.exists())

    def test_lock_is_released_when_owner_record_cannot_be_written(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'audit.lock'
            with patch.object(audit, 'private_json', side_effect=OSError('owner write failed')):
                with self.assertRaisesRegex(OSError, 'owner write failed'):
                    with audit.exclusive(path): pass
            self.assertFalse(path.exists())

    def test_disabled_models_are_not_enabled_by_credentials(self):
        config = {'providers': {'p': {'api': 'https://example.com/v1', 'models': {'yes': {'status': 'active'}, 'no': {'status': 'disabled'}}}}}
        self.assertEqual([m['model_key'] for m in audit.enabled_models(config)], ['yes'])

    def test_coverage_rejects_duplicate_and_missing_rows(self):
        rows = [{'source': 'a', 'provider_id': 'p', 'model_key': 'm'}]
        self.assertEqual(audit.coverage(rows, rows), 1)
        for actual in [[], rows + rows]:
            with self.assertRaises(ValueError): audit.coverage(rows, actual)

    def test_summary_does_not_count_missing_credentials_as_passed(self):
        self.assertEqual(audit.summarize([{'status': 'passed'}, {'status': 'missing_credentials'}, {'status': 'unauthorized'}]), {'passed': 1, 'missing_credentials': 1, 'unauthorized': 1})

    def test_cli_has_service_inventory_and_output(self):
        args = audit.parser().parse_args(['service', '--inventory', 'inventory.json', '--output', 'matrix.json'])
        self.assertEqual(args.command, 'service'); self.assertEqual(args.inventory, Path('inventory.json'))

    def test_boundary_verification_is_reproducible(self):
        self.assertEqual(audit.parser().parse_args(['boundary-tests']).command, 'boundary-tests')

    def test_diagnostic_rejects_wrong_model_provider_or_protocol(self):
        for provider, model, protocol in [('other', 'model', 'messages'), ('saved', 'other', 'messages'), ('saved', 'model', 'chat')]:
            with tempfile.TemporaryDirectory() as directory, patch.object(audit, 'http', side_effect=[
                (202, {'taskId': 'id'}),
                (200, {'state': {'status': 'succeeded'}, 'results': [result()]})
            ]):
                actual = audit.diagnostic('http://127.0.0.1', 'private', provider, model, 'model', protocol, Path(directory), 0)
                self.assertEqual(actual['status'], 'binding_mismatch')

    def test_http_uses_the_production_diagnostic_token_header(self):
        opener = MagicMock()
        response = opener.open.return_value.__enter__.return_value
        response.status = 200; response.read.return_value = b'{}'
        with patch.object(audit.urllib.request, 'build_opener', return_value=opener):
            audit.http('http://127.0.0.1', '/diagnostics', 'private')
        request = opener.open.call_args[0][0]
        self.assertEqual(request.get_header('X-agent-apps-token'), 'private')

    def test_relay_compilation_uses_exact_dependency_fingerprints(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); target = root / 'target/rust-slot-1'
            deps = target / 'debug/deps'; deps.mkdir(parents=True)
            (root / 'target/rust-slot-locks/slot-1.lock').mkdir(parents=True)
            fingerprints = target / 'debug/.fingerprint'
            agent = fingerprints / 'vw-agent-fixture'; agent.mkdir(parents=True)
            (deps / 'libvw_agent-fixture.rlib').touch()
            entries = []
            for index, name in enumerate(['tokio', 'serde', 'serde_json', 'anyhow', 'vw_api_types'], 1):
                folder = fingerprints / (name + '-correct'); folder.mkdir()
                (folder / ('lib-' + name)).write_text(index.to_bytes(8, 'little').hex())
                (deps / ('lib' + name + '-correct.rlib')).touch()
                (deps / ('lib' + name + '-newer_wrong.rlib')).touch()
                entries.append([0, name, False, index])
            (agent / 'lib-vw_agent.json').write_text(json.dumps({'deps': entries}))
            with patch.object(audit, 'ROOT', root), patch.object(audit.subprocess, 'run', return_value=MagicMock(returncode=0)) as compile_process:
                audit.compile_relay(target)
            command = compile_process.call_args[0][0]
            self.assertIn('tokio=' + str(deps / 'libtokio-correct.rlib'), command)
            self.assertFalse(any('newer_wrong' in value for value in command))

    def test_configured_account_preserves_production_environment_precedence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root / 'database').mkdir()
            (root / 'auth.json').write_text(json.dumps({'p': {'type': 'api', 'key': 'fixture-file'}}))
            with sqlite3.connect(str(root / 'database/vibe-window.sqlite3')) as connection:
                connection.execute('CREATE TABLE system_configs(platform_id TEXT,company_id TEXT,payload_json TEXT)')
                connection.execute('INSERT INTO system_configs VALUES(?,?,?)', ('p', 'c', json.dumps({'providers': {'p': {'api': 'https://example.com/v1', 'env': ['SHIJU_PROTOCOL_FIXTURE_KEY'], 'models': {'m': {'status': 'active'}}}}})))
            with patch.dict(os.environ, {'SHIJU_PROTOCOL_FIXTURE_KEY': 'fixture-environment'}):
                account = audit.configured_accounts([root])[0]
            self.assertEqual(account['credential'], 'fixture-environment')
            self.assertEqual(account['credential_state'], 'environment')

    def test_public_report_refuses_credential_material(self):
        with self.assertRaisesRegex(ValueError, 'credential'):
            audit.ensure_public('{"model":"fixture-private-credential"}', ['fixture-private-credential'])
        self.assertEqual(audit.ensure_public('{"status":"passed"}', ['fixture-private-credential']), '{"status":"passed"}')

    def test_source_record_is_not_verified_by_a_different_configured_binding(self):
        row = {'source': 'catalog.json', 'provider_id': 'p', 'model_key': 'm', 'model_api_id': 'wire', 'canonical_model_id': None, 'source_declarations': {'api': 'https://example.com/v1'}, 'applicability': 'text', 'research_conclusion': '资料不足', 'research_evidence_ids': ['e'], 'research_reason': 'fixture'}
        for model, endpoint in [('other', 'https://example.com/v1'), ('wire', 'https://other.example/v1')]:
            observation = {'provider_id': 'p', 'model_key': 'm', 'model_api_id': model, 'endpoint_fingerprint': audit.hashlib.sha256(endpoint.encode()).hexdigest()}
            record = audit.service_records([row], [observation])[0]
            self.assertEqual(record['service_observation_indices'], [])
            self.assertEqual(record['configured_override_observation_indices'], [0])

class ProductionWireTests(unittest.TestCase):
    def test_wire_json_requires_content_and_successful_terminal(self):
        from model_protocol_service_wire import validate_json
        for protocol, body in [
            ('chat', {'model': 'm', 'choices': [{'message': {'role': 'assistant', 'content': ''}, 'finish_reason': 'stop'}]}),
            ('messages', {'model': 'm', 'type': 'message', 'role': 'assistant', 'content': [{'type': 'text', 'text': 'OK'}]}),
            ('responses', {'model': 'm', 'status': 'incomplete', 'output': []}),
            ('gemini', {'modelVersion': 'm', 'candidates': [{'finishReason': 'STOP', 'content': {'role': 'model', 'parts': []}}]})
        ]:
            self.assertEqual(validate_json(protocol, body)['status'], 'invalid_response')

    def test_wire_tools_require_the_exact_echo_name_arguments_and_id(self):
        from model_protocol_service_wire import validate_json
        body = {'model': 'm', 'choices': [{'message': {'role': 'assistant', 'tool_calls': [{'id': 'call', 'type': 'function', 'function': {'name': 'other', 'arguments': '{"value":"probe"}'}}]}, 'finish_reason': 'tool_calls'}]}
        self.assertEqual(validate_json('chat', body, tools=True)['status'], 'invalid_response')
        body['choices'][0]['message']['tool_calls'][0]['function']['name'] = 'audit_echo'
        self.assertEqual(validate_json('chat', body, tools=True)['status'], 'passed')

    def test_thinking_tool_acceptance_requires_observed_thinking(self):
        from model_protocol_service_wire import validate_json
        body = {'model': 'm', 'type': 'message', 'role': 'assistant', 'stop_reason': 'tool_use', 'content': [{'type': 'tool_use', 'id': 'call', 'name': 'audit_echo', 'input': {'value': 'probe'}}]}
        self.assertEqual(validate_json('messages', body, tools=True, require_thinking=True)['status'], 'invalid_response')
        body['content'].insert(0, {'type': 'thinking', 'thinking': 'fixture', 'signature': 'fixture-receipt'})
        self.assertEqual(validate_json('messages', body, tools=True, require_thinking=True)['status'], 'passed')

    def test_sse_requires_text_and_protocol_terminal(self):
        from model_protocol_service_wire import validate_events
        for protocol, events in [('chat', [('message', {'choices': [{'delta': {'content': 'OK'}}]})]), ('messages', [('message_stop', {'type': 'message_stop'})]), ('responses', [('response.completed', {'type': 'response.completed', 'response': {'status': 'completed'}})]), ('gemini', [('message', {'candidates': [{'finishReason': 'STOP'}]})])]:
            self.assertEqual(validate_events(protocol, events)['status'], 'invalid_response')

    def test_wire_does_not_forward_binding_token_to_a_redirect_target(self):
        from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
        import threading
        from model_protocol_service_wire import exchange
        redirected = []
        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args): pass
            def do_POST(self):
                self.rfile.read(int(self.headers['Content-Length']))
                self.send_response(302); self.send_header('Location', '/redirect-target'); self.end_headers()
            def do_GET(self):
                redirected.append(self.headers.get('Authorization'))
                self.send_response(200); self.end_headers(); self.wfile.write(b'{}')
        server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
        try:
            with tempfile.TemporaryDirectory() as directory:
                result, _ = exchange('http://127.0.0.1:%d' % server.server_port, 'fixture-token', 'chat', 'm', {}, Path(directory) / 'raw.json', audit.private_json)
            self.assertEqual(redirected, [])
            self.assertEqual(result.get('http_status'), 302)
        finally:
            server.shutdown(); server.server_close(); thread.join()

    def test_chat_sse_records_content_finish_and_done(self):
        from model_protocol_service_wire import validate_events
        events = [('message', {'model': 'm', 'choices': [{'delta': {'content': 'OK'}, 'finish_reason': None}]}), ('message', {'choices': [{'delta': {}, 'finish_reason': 'stop'}]}), ('done', None)]
        result = validate_events('chat', events)
        self.assertEqual(result['status'], 'passed')
        self.assertTrue(result['terminal'])


class StreamingToolEvidenceTests(unittest.TestCase):
    def test_tool_finish_requires_complete_arguments_and_done(self):
        from model_protocol_service_wire import validate_events
        events = [('message', {'model': 'm', 'choices': [{'delta': {'tool_calls': [{'index': 0, 'id': 'echo-1', 'type': 'function', 'function': {'name': 'audit_echo', 'arguments': '{"value":"probe"}'}}]}, 'finish_reason': None}]}), ('message', {'choices': [{'delta': {}, 'finish_reason': 'tool_calls'}]}), ('done', None)]
        self.assertEqual(validate_events('chat', events, tools=True)['status'], 'passed')
        self.assertNotEqual(validate_events('chat', events[:-1], tools=True)['status'], 'passed')
        events[0][1]['choices'][0]['delta']['tool_calls'][0]['function']['arguments'] = '{'
        self.assertNotEqual(validate_events('chat', events, tools=True)['status'], 'passed')


if __name__ == '__main__': unittest.main()
