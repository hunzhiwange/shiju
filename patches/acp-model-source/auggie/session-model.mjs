// Binding for the pinned Auggie 0.37.1 runtime. Native ACP, tools and permissions stay upstream.
import { readFileSync } from 'node:fs';
export function validateConfig(config) {
  const url = new URL(config.endpoint);
  if (url.protocol !== 'http:' || url.hostname !== '127.0.0.1' || !url.port || url.pathname !== '/' || url.username || url.password || url.search || url.hash)
    throw new Error('Auggie requires a loopback relay');
  if (!config.model || !config.token || !Number.isSafeInteger(config.outputLimit) || config.outputLimit < 1)
    throw new Error('Invalid Auggie session model binding');
  return config;
}
function config() { return validateConfig(JSON.parse(readFileSync(process.env.SHIJU_AUGGIE_MODEL_CONFIG,'utf8'))); }
export function modelConfig(flags, flagsV2) {
 const c=config();return { defaultModel:c.model,models:[],languages:[],featureFlags:{...flags,agentChatModel:c.model,
 modelInfoRegistry:{[c.model]:{displayName:`Session model (${c.model})`,isDefault:true}},enableModelRegistry:true},featureFlagsV2:flagsV2,userTier:'unknown' };
}
function requestNodes(messages, nodes, fallback, pending) {
 let text='';const context=[];
 const flush=()=>{if(text){messages.push({role:'user',content:text});text='';}};
 for(const node of nodes??[]) {
  if(node.type===0) { if(pending.size) throw new Error('Auggie incomplete tool results');text+=node.text_node?.content??''; }
  else if(node.type===1 && node.tool_result_node) {
   flush();const r=node.tool_result_node;
   if(!pending.delete(r.tool_use_id)) throw new Error('Auggie unmatched tool result');
   if(r.content_nodes?.some(n=>n.type!==1)) throw new Error('Auggie unsupported tool result content');
   messages.push({role:'tool',tool_call_id:r.tool_use_id,content:r.content??''});
  } else if(node.type===4&&node.ide_state_node){context.push(JSON.stringify(node.ide_state_node));}
  else throw new Error('Auggie unsupported request content');
 }
 if(context.length){if(pending.size)throw new Error('Auggie incomplete tool results');text+='\nWorkspace context: '+context.join('\n');}
 if(text)flush();else if(!(nodes?.length)&&fallback){if(pending.size)throw new Error('Auggie incomplete tool results');messages.push({role:'user',content:fallback});}
}
function responseNodes(messages, nodes, fallback, pending) {
 let content='',reasoning='',calls=[];
 for(const node of nodes??[]) {
  if(node.type===0)content+=node.content??'';
  else if(node.type===5&&node.tool_use){const t=node.tool_use;if(pending.has(t.tool_use_id))throw new Error('Auggie duplicate tool ID');
   JSON.parse(t.input_json);pending.add(t.tool_use_id);calls.push({id:t.tool_use_id,type:'function',function:{name:t.tool_name,arguments:t.input_json}});}
  else if(node.type===8&&node.thinking&&!node.thinking.encrypted_content)reasoning+=node.thinking.content??'';
  else if(node.type!==10)throw new Error('Auggie unsupported response content');
 }
 if(content||calls.length||fallback)messages.push({role:'assistant',content:content||fallback||null,...reasoning?{reasoning_content:reasoning}:{},...calls.length?{tool_calls:calls}:{}});
}
export function toChatRequest(input,c) {
 validateConfig(c);
 if(input.model && input.model!==c.model)throw new Error('Auggie selected model mismatch');
 if(input.systemPromptReplacements?.length)throw new Error('Auggie unsupported system prompt replacements');
 const messages=[{role:'system',content:[input.systemPrompt??'You are Auggie, a coding assistant. Use the provided tools to inspect and edit files. Complete the user request accurately.',input.systemPromptAppend,input.userGuidelines,input.workspaceGuidelines,...(input.rules??[]).map(r=>typeof r==='string'?r:JSON.stringify(r)),...(input.skills??[]).map(r=>typeof r==='string'?r:JSON.stringify(r))].filter(Boolean).join('\n\n')}];
 const pending=new Set();
 for(const h of input.chatHistory??[]){requestNodes(messages,h.request_nodes,h.request_message,pending);responseNodes(messages,h.response_nodes,h.response_text,pending);}
 requestNodes(messages,input.nodes,input.message,pending);
 if(pending.size)throw new Error('Auggie incomplete tool results');
 const tools=(input.toolDefinitions??[]).map(t=>({type:'function',function:{name:t.name,description:t.description??'',parameters:JSON.parse(t.input_schema_json)}}));
 return {model:c.model,messages,stream:true,max_tokens:c.outputLimit,...tools.length?{tools}:{}};
}
export async function* fromChatStream(body,allowed) {
 const decoder=new TextDecoder();let buffer='',done=false,finish=null,id=0;const calls=new Map();
 function consume(line) {
  if(!line.startsWith('data:'))return null;
  const data=line.slice(5).trim();if(data==='[DONE]'){done=true;return null;}
  let event;try{event=JSON.parse(data);}catch{throw new Error('Auggie invalid model stream');}
  if(event.error)throw new Error('Auggie model stream failed');
  const choices=event.choices??[];if(choices.length>1)throw new Error('Auggie unsupported multiple choices');
  const choice=choices[0];if(!choice)return null;
  const delta=choice.delta??{};
  if(choice.finish_reason)finish=choice.finish_reason;
  for(const t of delta.tool_calls??[]) {
   if(!Number.isInteger(t.index))throw new Error('Auggie invalid tool stream');
   let call=calls.get(t.index);if(!call){call={id:'',name:'',arguments:''};calls.set(t.index,call);}
   call.id+=t.id??'';call.name+=t.function?.name??'';call.arguments+=t.function?.arguments??'';
  }
  const nodes=[];if(delta.content)nodes.push({id:++id,type:0,content:delta.content});
  if(delta.reasoning_content)nodes.push({id:++id,type:8,content:delta.reasoning_content,thinking:{content:delta.reasoning_content,summary:delta.reasoning_content}});
  return nodes.length?{text:delta.content??'',nodes}:null;
 }
 for await(const chunk of body){buffer+=decoder.decode(chunk,{stream:true});let pos;
  while((pos=buffer.indexOf('\n'))>=0){const line=buffer.slice(0,pos).replace(/\r$/,'');buffer=buffer.slice(pos+1);const result=consume(line);if(result)yield result;}}
 buffer+=decoder.decode();if(buffer.trim()){const result=consume(buffer.trim());if(result)yield result;}
 if(!done||!finish)throw new Error('Auggie truncated model stream');
 if(!['stop','length','tool_calls'].includes(finish))throw new Error('Auggie unsupported model stop reason');
 if((finish==='tool_calls')!==!!calls.size)throw new Error('Auggie inconsistent tool completion');
 const seen=new Set(),nodes=[];
 for(const call of calls.values()){
  if(!call.id||seen.has(call.id)||!allowed.has(call.name))throw new Error('Auggie invalid returned tool');
  seen.add(call.id);try{const value=JSON.parse(call.arguments);if(!value||typeof value!=='object'||Array.isArray(value))throw 0;}catch{throw new Error('Auggie invalid returned tool arguments');}
  nodes.push({id:++id,type:5,content:'',tool_use:{tool_use_id:call.id,tool_name:call.name,input_json:call.arguments}});
 }
 yield {text:'',nodes,stop_reason:finish==='tool_calls'?3:finish==='length'?2:1};
}
export async function chatStream(input) {
 const c=config(),request=toChatRequest(input,c);
 const response=await fetch(new URL('v1/chat/completions',c.endpoint),{method:'POST',headers:{'Content-Type':'application/json',Authorization:`Bearer ${c.token}`},body:JSON.stringify(request),signal:input.signal,redirect:'error'});
 if(!response.ok){await response.body?.cancel();throw new Error(`Auggie model request failed (HTTP ${response.status})`);}
 return fromChatStream(response.body,new Set((input.toolDefinitions??[]).map(t=>t.name)));
}
