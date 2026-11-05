"""Anthropic wire envelopes for the fixed Claude SDK; native tools stay real."""
import json


def messages_message(fixture, body):
    messages = [{'role': 'system', 'content': body.get('system', '')}]
    for message in body.get('messages', []):
        content = message.get('content', '')
        if not isinstance(content, list):
            messages.append(message)
            continue
        text = []
        for block in content:
            if block.get('type') == 'tool_result':
                messages.append({'role': 'tool', 'tool_call_id': block['tool_use_id'], 'content': block.get('content', '')})
            elif block.get('type') == 'text':
                text.append(block['text'])
        if text:
            messages.append({'role': message['role'], 'content': '\n'.join(text)})
    tools = [{'name': t['name'], 'parameters': t.get('input_schema', {}), 'description': t.get('description', '')} for t in body.get('tools', [])]
    return fixture.response({'messages': messages, 'tools': tools, 'claude_native': True})


def messages_wire(message, model):
    if message.get('tool_calls'):
        call = message['tool_calls'][0]
        block = {'type': 'tool_use', 'id': call['id'], 'name': call['function']['name'], 'input': json.loads(call['function']['arguments'])}
        reason = 'tool_use'
    else:
        block = {'type': 'text', 'text': message['content']}
        reason = 'end_turn'
    value = {'id': 'msg_skill_fixture', 'type': 'message', 'role': 'assistant', 'model': model,
             'content': [block], 'stop_reason': reason, 'stop_sequence': None,
             'usage': {'input_tokens': 1, 'output_tokens': 1}}
    initial = dict(block, **({'input': {}} if reason == 'tool_use' else {'text': ''}))
    delta = ({'type': 'input_json_delta', 'partial_json': json.dumps(block['input'])} if reason == 'tool_use'
             else {'type': 'text_delta', 'text': block['text']})
    events = [
        {'type': 'message_start', 'message': dict(value, content=[], stop_reason=None, usage={'input_tokens': 1, 'output_tokens': 0})},
        {'type': 'content_block_start', 'index': 0, 'content_block': initial},
        {'type': 'content_block_delta', 'index': 0, 'delta': delta},
        {'type': 'content_block_stop', 'index': 0},
        {'type': 'message_delta', 'delta': {'stop_reason': reason, 'stop_sequence': None}, 'usage': {'output_tokens': 1}},
        {'type': 'message_stop'},
    ]
    return value, ''.join('event: ' + e['type'] + '\ndata: ' + json.dumps(e) + '\n\n' for e in events).encode()
