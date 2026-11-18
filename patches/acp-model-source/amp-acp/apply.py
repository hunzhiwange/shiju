"""Patch only the integrity-checked Amp adapter and local ThreadWorker input."""
import hashlib
from pathlib import Path
import shutil
import sys

root = Path(sys.argv[1])
engine = root.parent / '@sourcegraph/amp/dist/main.js'
if hashlib.sha256(engine.read_bytes()).hexdigest() != '458435712c9c4e222822847fc9e70080cdf023ffb3c820e959a30425a22a5836':
    raise RuntimeError('Amp engine differs from reviewed input')
s = engine.read_text()
def replace(before, after):
    global s
    if s.count(before) != 1:
        raise RuntimeError('Amp patch contract changed')
    s = s.replace(before, after)
replace('let V=!Q.takeMeBack&&gA(D)&&at(D.features,TB.THREAD_ACTORS_TUI)', 'let V=!1')
replace('F=Z&&!Q.takeMeBack;if(Q.showWelcome', 'F=!1;if(Q.showWelcome')
replace('let D=gA($.serverStatus)&&at($.serverStatus.features,TB.THREAD_ACTORS_TUI)&&!A.takeMeBack,G;', 'let D=!1,G;')
# Runs before hooks, native argument processing, filesystem or command tool bodies.
replace('async invokeTool(A,Q){if(this.runningTools.has(A.id))', '''async invokeTool(A,Q){
const selectedMcp = typeof A.name === "string" && A.name.startsWith("mcp__");
try {
  const response = await fetch(new URL(selectedMcp ? '/shiju/mcp-permission' : '/shiju/tool', process.env.AMP_URL), {
    method: 'POST',
    headers: {'content-type': 'application/json', authorization: 'Bearer ' + process.env.AMP_API_KEY},
    body: JSON.stringify({sessionId: process.env.SHIJU_AMP_SESSION, id: A.id, name: A.name, input: A.input})
  });
  if (!response.ok) throw Error('Host rejected Amp tool');
  if (!selectedMcp) {
    const hostResult = await response.json();
    if (typeof hostResult.output !== 'string') throw Error('Invalid Amp host tool output');
    await this.callbacks.handle({type:'tool:data',toolUse:A.id,data:{status:'done',result:hostResult.output,isFinal:!1}});
  }
} catch {
  await this.callbacks.handle({type:'tool:data',toolUse:A.id,data:{status:'error',error:{message:'Amp host tool rejected',displayMessage:'Amp host tool rejected'}}});
  this.resolveToolCompletion(A.id, true);
  return;
}
if (!selectedMcp) { this.resolveToolCompletion(A.id, true); return; }
if(this.runningTools.has(A.id))''')
# Disable ambient discovery before scanning files or starting plugin/MCP processes.
replace('async function s70(A,Q,B,$){', 'async function s70(A,Q,B,$){return {skills:[],errors:[],warnings:[]};')
replace('async function kq(A,Q,B){', 'async function kq(A,Q,B){return [];')
replace('function qk6({filesystem:A,configService:Q},B){', 'function qk6({filesystem:A,configService:Q},B){return u0.of([]);')
replace('function Nk6({filesystem:A,configService:Q},B){', 'function Nk6({filesystem:A,configService:Q},B){return u0.of([]);')
replace('async function V0(k0){try{return(await $.readdir(k0))', 'async function V0(k0){return [];try{return(await $.readdir(k0))')
replace('mcpServers:T,mcpPermissions:x,mcpTrustedServers:h,mcpRegistryUrl:gA(f)?f.workspace?.mcpRegistryUrl??null:null,externalMCPServers:u,skillMCPServers:k', 'mcpServers:shijuSelectedMcp,mcpPermissions:[],mcpTrustedServers:[],mcpRegistryUrl:null,externalMCPServers:{},skillMCPServers:{}')
# Keep native discovery/calls, with only the parent-validated stdio list.
replace('if(A.mcpConfig){let F=await wg4(A.mcpConfig);J=Cg4(J,F)}', 'if(process.env.SHIJU_AMP_MCP_SERVERS)shijuSelectedMcp=await wg4(process.env.SHIJU_AMP_MCP_SERVERS);')
replace('let J=A.env?Object.entries(A.env).reduce((V,[U,K])=>({...V,[U]:cd(K,process.env)}),{}):void 0,Y=BN6({...process.env,...J,AWS_VAULT_PROMPT:"stdout"}),Z=cd(A.command,Y),F=A.args?A.args.map((V)=>cd(V,Y)):void 0', 'let Y={PATH:process.env.PATH,LANG:process.env.LANG,...A.env},Z=A.command,F=A.args')
replace('async function yD(){return{trees:[await Jk4(r0.file(process.cwd()))],platform:Yk4()}}', 'async function yD(){return{trees:[{uri:r0.file(process.cwd()).toString()}],platform:Yk4()}}')
replace('I=w0,await A0(w0);let n=tv4(w0.messages)', 'I=w0;let localLast=w0.messages.at(-1);if(localLast?.role==="assistant"&&localLast.state.type!=="streaming"&&!localLast.content.some(part=>part.type==="tool_use"))await Y.threadService.flushVersion(W,w0.v);await A0(w0);let n=tv4(w0.messages)')
replace('await pF(s)}if(N0.unsubscribe()', 'await Y.threadService.flushVersion(W,I.v);await pF(s)}if(N0.unsubscribe()')
replace('function dM4(A,Q){', 'function dM4(A,Q){return {action:null};')
replace('function cM4(A,Q){', 'function cM4(A,Q){return {action:null};')
replace('getHooks:async()=>(await this.getConfig()).settings?.hooks', 'getHooks:async()=>undefined')
replace('trackFilesFromHistory(){', 'trackFilesFromHistory(){return;')
replace('async restoreFileChangesFromBackups(){', 'async restoreFileChangesFromBackups(){return;')
replace('async function tM4(A,{configService:Q,filesystem:B,previousUserState:$}){', 'async function tM4(A,{configService:Q,filesystem:B,previousUserState:$}){return {message:{role:"user",messageId:0,content:A.content,source:A.source,agentMode:A.agentMode,meta:{sentAt:Date.now()}}};')
guard = """let shijuSelectedMcp = {};
const shijuOrigin = new URL(process.env.AMP_URL);
if (shijuOrigin.protocol !== 'http:' || shijuOrigin.hostname !== '127.0.0.1' || !shijuOrigin.port || shijuOrigin.username || shijuOrigin.password || !process.env.SHIJU_AMP_SESSION) throw Error('Amp local control boundary is required');
const shijuFetch = globalThis.fetch.bind(globalThis);
globalThis.fetch = (input, init) => {
 const target = new URL(typeof input === 'string' ? input : input.url || input.href);
 if (target.origin !== shijuOrigin.origin || target.username || target.password) throw Error('Amp direct network access is disabled');
 return shijuFetch(input, { ...init, redirect: 'error' });
};
"""
s = s.replace('\n', '\n' + guard, 1)
engine.write_text(s)
entry = root / 'dist/index.js'
s = entry.read_text()
replace('class AmpAcpAgent {', 'class AmpAcpAgent {\n  local;')
replace('    this.client = client;\n    this.transport = transport;', '    this.client = client;\n    this.local = new LocalAmp(this);\n    this.transport = transport;')
# The pinned engine uses the legacy mode vocabulary; arbitrary model IDs never
# select another provider because the control bridge pins all model requests.
replace('      mode: s.model,', '      mode: "smart",')
replace('      env: { TERM: "dumb" },', '      env: { TERM: "dumb", ...this.local.environment(params.sessionId) },')
replace('      if (Object.keys(s.mcpConfig).length > 0) {\n        options.mcpConfig = s.mcpConfig;\n      }', '      // Selected MCP configuration travels in the private worker environment.')
replace('  async newSession(params) {', '  async newSession(params) {\n    await this.local.start();')
replace('  async loadSession(params) {', '  async loadSession(params) {\n    await this.local.start();')
assert s.count('convertAcpMcpServersToAmpConfig(params.mcpServers)') == 2
s = s.replace('convertAcpMcpServersToAmpConfig(params.mcpServers)', 'selectedStdio(params.mcpServers)')
replace('    let session = this.sessions.get(params.sessionId);\n    if (!session) {', '    let session = this.sessions.get(params.sessionId);\n    if (session) await this.local.reloadSelection(session, params);\n    if (!session) {')
replace('  async resumeSession(params) {', '  async resumeSession(params) {\n    const existing = this.sessions.get(params.sessionId);\n    if (existing) await this.local.reloadSelection(existing, params);')
replace('  const closeSession = (sessionId) => {\n    terminateSession(sessionId);\n  };', '  const closeSession = terminateSession;')
conversion_start = s.index('// src/mcp-config.ts')
conversion_end = s.index('// src/thread-mapping-store.ts', conversion_start)
s = s[:conversion_start] + s[conversion_end:]
# Restrict execution to the local CLI and disable ambient plugin discovery.
replace('    this.discoverPluginModes = options.discoverPluginModes ?? discoverPluginModes;', '    this.discoverPluginModes = () => [];')
replace('  async prompt(params) {', '  async prompt(params) {\n    if (params.prompt.some(block => block.type !== "text")) throw Error("Amp non-text prompts are unavailable");\n    await this.local.start();\n    if (this.sessions.get(params.sessionId)?.executor !== "local") throw Error("Amp remote execution is unavailable");\n    if (this.sessions.get(params.sessionId).reloading) throw Error("Amp session is loading");')
replace('function createCliTransport(command = process.env.AMP_CLI_PATH ?? "amp", commandArgs = [], preserveCancelledProcess = process.env.AMP_ACP_CANCEL_MODE === "steer")', 'function createCliTransport(command = ampCommand, commandArgs = [], preserveCancelledProcess = false)')
replace('function createAmpTransport(name = process.env.AMP_ACP_TRANSPORT ?? "cli")', 'function createAmpTransport(name = "cli")')
assert s.count('  args.push("--no-archive-after-execute");') == 2
s = s.replace('  args.push("--no-archive-after-execute");', '  // Legacy local persistence.')
replace('  async setSessionConfigOption(params) {', '  async setSessionConfigOption(params) { throw Error("Amp native options are unavailable for the bound session model");')
replace('  async setSessionMode(params) {', '  async setSessionMode(params) { throw Error("Amp host policy cannot be changed by native mode");')
replace('    this.exportThread = options.exportThread ?? exportThreadHistory;', '    this.exportThread = (threadId, cwd) => this.local.exportThread(threadId, cwd);')
replace('    this.setThreadArchived = options.setThreadArchived ?? setAmpThreadArchived;', '    this.setThreadArchived = async () => { throw Error("Amp native archive is unavailable"); };')
replace('else if (process.env.AMP_ACP_CONTINUE_LATEST)', 'else if (false)')
replace('      model: "medium",', '      model: process.env.SHIJU_AMP_MODEL,')
replace('  sessionFromMapping(mapping, params) {', '  sessionFromMapping(mapping, params) { if(mapping.binding !== process.env.SHIJU_AMP_BINDING || mapping.model !== process.env.SHIJU_AMP_MODEL || (params.cwd && path5.resolve(params.cwd) !== path5.resolve(mapping.cwd))) throw Error("Amp persisted session identity mismatch");')
replace('      threadId: s.threadId,', '      threadId: s.threadId,\n      binding: process.env.SHIJU_AMP_BINDING,')
replace('["mode", "model", "effort", "executor", "cwd", "title", "updatedAt"]', '["mode", "model", "effort", "executor", "cwd", "title", "updatedAt", "binding"]')
replace('  async persistSessionQuiet(sessionId, s) {\n    try {\n      await this.persistSession(sessionId, s);\n    } catch (e) {\n      console.error("[acp] failed to persist session settings", e);\n    }\n  }', '  async persistSessionQuiet(sessionId, s) { await this.persistSession(sessionId, s); }')
s = s.replace('throw RequestError.authRequired();', 'throw new Error("Selected session model authentication failed");')
replace('        if (message.type === "result" && message.is_error) {', '        if (message.type === "result" && message.is_error) { if(typeof message.error === "string" && isAuthError(message.error)) throw new Error("Selected session model authentication failed"); throw new Error("Amp selected model execution failed");')
assert s.count('env: { ...process.env, ...options.env }') == 3
s = s.replace('env: { ...process.env, ...options.env }', 'env: localAmpEnvironment(options.env)')
replace('        console.error("[acp] failed to replay thread history", e);', '        throw new Error("Amp persisted thread replay failed");')
replace('  async cancel(params) {', '  async cancel(params) { this.local.revoke(params.sessionId);')
replace('  async closeSession(params) {', '  async closeSession(params) { this.local.revoke(params.sessionId);')
replace('  async deleteSession(params) {', '  async deleteSession(params) { this.local.revoke(params.sessionId);')
replace('  close() {\n    this.transport.closeAll?.();', '  close() {\n    for (const session of this.sessions.values()) this.cancelSessionTurns(session);\n    this.local.close();\n    this.transport.closeAll?.();')
auth_start = s.index('      authMethods: [', s.index('class AmpAcpAgent'))
auth_end = s.index('\n    };\n  }\n  async newSession', auth_start)
s = s[:auth_start] + '      authMethods: []' + s[auth_end:]
meta_start = s.index('        _meta: {', s.index('class AmpAcpAgent'))
meta_end = s.index('\n      },\n      authMethods:', meta_start)
s = s[:meta_start] + '        _meta: {}' + s[meta_end:]
# Hide options which would imply native model/permission/orb changes.
s = s.replace('configOptions: buildSessionConfigOptions(session)', 'configOptions: []')
s = s.replace('promptCapabilities: { image: true, embeddedContext: true }', 'promptCapabilities: { image: false, embeddedContext: false }')
s = s.replace('mcpCapabilities: { http: true, sse: true }', 'mcpCapabilities: { http: false, sse: false }')
main_start = s.index('if (process.argv.includes("--setup")) {')
s = s[:main_start] + 'if (process.argv.length !== 2) throw new Error("Amp reviewed adapter requires empty arguments");\nrunAcp();\nprocess.stdin.resume();\n'
s = s.replace('\n', '\nimport { LocalAmp, ampCommand, localAmpEnvironment, selectedStdio } from "./shiju-local.mjs";\n', 1)
entry.write_text(s)
shutil.copy2(Path(__file__).with_name('local.mjs'), root / 'dist/shiju-local.mjs')
launcher = root / 'dist/amp-cli'
launcher.write_text('#!/bin/sh\nexec node "$(dirname "$0")/../../@sourcegraph/amp/dist/main.js" "$@"\n')
launcher.chmod(0o700)
