// Controlled fixture only. CDP launches Chrome and clicks the extension action;
// all page/file actions are sent by the Rust model tool over Native Messaging.
import {spawn} from 'node:child_process';
import {createServer} from 'node:http';
import {readFile,access} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import assert from 'node:assert/strict';
const profile=process.env.HOME+'/.shiju/computer-control/chrome-profile';
const delay=ms=>new Promise(r=>setTimeout(r,ms));
let uploaded='',downloads=0;
const server=createServer(async(req,res)=>{
  if(req.url==='/upload-proof'){
    for await(const chunk of req)uploaded+=chunk.toString();
    res.end('ok');return;
  }
  if(req.url==='/download'){
    downloads++;res.setHeader('Content-Type','application/octet-stream');res.end('cc10-download-fixture');return;
  }
  if(req.url==='/redirect'){
    res.writeHead(302,{Location:`http://localhost:${server.address().port}/destination`});res.end();return;
  }
  res.setHeader('Content-Type','text/html; charset=utf-8');
  res.end(`<!doctype html><title>ShiJu CC10 fixture</title><input id="file" type="file"><a id="download" download="fixture.txt" href="/download">fixture download</a><p>controlled fixture</p><script>document.querySelector('#file').onchange=async event=>{const file=event.target.files[0];await fetch('/upload-proof',{method:'POST',body:await file.text()});};</script>`);
});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const fixtureUrl=`http://127.0.0.1:${server.address().port}/fixture`;
const chrome=spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',[
  `--user-data-dir=${profile}`,'--remote-debugging-port=0','--enable-unsafe-extension-debugging','--no-first-run','--no-default-browser-check','--headless=new','about:blank'
],{stdio:'ignore'});
let socket;
try{
  let address;
  for(let i=0;i<100;i++){try{address=(await readFile(join(profile,'DevToolsActivePort'),'utf8')).trim().split('\n');break;}catch{}await delay(100);}
  assert.ok(address,'DevTools startup');
  socket=new WebSocket(`ws://127.0.0.1:${address[0]}${address[1]}`);
  await new Promise((r,j)=>{socket.onopen=r;socket.onerror=j;});
  let next=0;const pending=new Map();
  socket.onmessage=e=>{const m=JSON.parse(e.data);if(m.id){const p=pending.get(m.id);pending.delete(m.id);m.error?p.reject(new Error(JSON.stringify(m.error))):p.resolve(m.result);}};
  const send=(method,params={},sessionId)=>new Promise((r,j)=>{const id=++next;pending.set(id,{resolve:r,reject:j});socket.send(JSON.stringify({id,method,params,...(sessionId?{sessionId}:{})}));});
  const version=await send('Browser.getVersion');
  const loaded=await send('Extensions.loadUnpacked',{path:resolve('plugins/shiju-computer-control/chrome')});
  assert.equal(loaded.id,'ogfgcnpdhnmkpcbaclbglefaclanipmg');
  let worker;
  for(let i=0;i<60;i++){worker=(await send('Target.getTargets')).targetInfos.find(t=>t.type==='service_worker'&&t.url.includes(loaded.id));if(worker)break;await delay(100);}
  assert.ok(worker);
  const session=(await send('Target.attachToTarget',{targetId:worker.targetId,flatten:true})).sessionId;
  const page=await send('Target.createTarget',{url:fixtureUrl});
  await delay(300);
  const tabs=await send('Runtime.evaluate',{expression:'chrome.tabs.query({active:true}).then(t=>t.map(x=>({id:x.id})))',awaitPromise:true,returnByValue:true},session);
  const tabId=tabs.result.value[0].id;
  console.log(JSON.stringify({tabId,fixtureUrl,version,extensionId:loaded.id}));
  await delay(1000);
  const tabTarget=(await send('Target.getTargets',{filter:[{type:'tab',exclude:false}]})).targetInfos.find(t=>t.url===fixtureUrl);
  await send('Extensions.triggerAction',{id:loaded.id,targetId:tabTarget.targetId});
  for(let i=0;i<600;i++){try{await access(process.env.SHIJU_CC10_DONE);break;}catch{}await delay(100);}
  const after=await send('Target.getTargetInfo',{targetId:page.targetId});
  assert.equal(uploaded,'cc10-upload-fixture');
  assert.ok(downloads>=1);
  assert.equal(new URL(after.targetInfo.url).hostname,'localhost');
  console.log(JSON.stringify({uploadedBytes:uploaded.length,downloads,finalUrl:after.targetInfo.url}));
  await send('Extensions.uninstall',{id:loaded.id});
  await send('Browser.close');
}finally{
  socket?.close();chrome.kill('SIGTERM');
  await new Promise(r=>{if(chrome.exitCode!==null)r();else chrome.once('exit',r);});
  await new Promise(r=>server.close(r));
}
