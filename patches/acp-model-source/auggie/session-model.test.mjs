import { test } from 'node:test';
import assert from 'node:assert/strict';
import { toChatRequest, fromChatStream, validateConfig } from './session-model.mjs';
const config = { model: 'selected', endpoint: 'http://127.0.0.1:1234/', token: 'fixture', outputLimit: 4096 };
test('tool history retains associations, text order and reasoning', () => {
  const request = toChatRequest({ chatHistory: [{ request_nodes: [{type: 0, text_node: {content:'write'}}], response_nodes: [
    {type:8,thinking:{content:'reason'}}, {type:5,tool_use:{tool_use_id:'a',tool_name:'save',input_json:'{"x":1}'}}
  ] }], nodes:[{type:4,ide_state_node:{current_terminal:{current_working_directory:'/fixture'}}},{type:1,tool_result_node:{tool_use_id:'a',content:'saved'}},{type:0,text_node:{content:'continue'}}],
  toolDefinitions:[{name:'save',input_schema_json:'{"type":"object"}'}]}, config);
  assert.deepEqual(request.messages.slice(1).map(m=>m.role), ['user','assistant','tool','user']);
  assert.equal(request.messages[2].reasoning_content,'reason');
  assert.equal(request.messages[2].tool_calls[0].id,'a');
  assert.equal(request.messages[3].tool_call_id,'a');
  assert.equal(request.model,'selected');
});
test('unknown content and mismatched model fail closed', () => {
  assert.throws(()=>toChatRequest({nodes:[{type:2}]},config), /unsupported/);
  assert.throws(()=>toChatRequest({model:'other'},config), /model/);
  assert.throws(()=>toChatRequest({nodes:[{type:1,tool_result_node:{tool_use_id:'orphan',content:'x'}}]},config), /tool/);
  assert.throws(()=>validateConfig({...config,endpoint:'https://example.com/'}), /loopback/);
});
async function* bytes(text) { for (const byte of new TextEncoder().encode(text)) yield new Uint8Array([byte]); }
function sse(value) { return 'data: '+JSON.stringify(value)+'\n\n'; }
test('stream preserves UTF-8 and assembles parallel tool arguments before execution', async () => {
 const stream = sse({choices:[{delta:{content:'你好',tool_calls:[{index:0,id:'a',function:{name:'save',arguments:'{"'}},{index:1,id:'b',function:{name:'read',arguments:'{}'}}]}}]})+
 sse({choices:[{delta:{tool_calls:[{index:0,function:{arguments:'x":1}'}}]},finish_reason:'tool_calls'}]})+'data: [DONE]\n\n';
 const chunks=[];for await(const chunk of fromChatStream(bytes(stream),new Set(['save','read']))) chunks.push(chunk);
 assert.equal(chunks.map(c=>c.text).join(''),'你好');
 assert.equal(chunks.at(-1).stop_reason,3);
 assert.deepEqual(chunks.at(-1).nodes.map(n=>n.tool_use.input_json), ['{"x":1}','{}']);
});
test('truncated or failed streams never produce successful completion', async () => {
 for (const stream of [sse({choices:[{delta:{content:'partial'}}]}),sse({error:{message:'secret'}})]) {
  await assert.rejects(async()=>{for await(const _ of fromChatStream(bytes(stream),new Set())){}}, /Auggie/);
 }
});
test('unknown tools and invalid JSON cannot execute from an otherwise complete stream', async () => {
 for (const [name,args] of [['unknown','{}'],['save','{broken']]) {
  const stream=sse({choices:[{delta:{tool_calls:[{index:0,id:'a',function:{name,arguments:args}}]},finish_reason:'tool_calls'}]})+'data: [DONE]\n\n';
  await assert.rejects(async()=>{for await(const _ of fromChatStream(bytes(stream),new Set(['save']))){}}, /Auggie invalid/);
 }
});
