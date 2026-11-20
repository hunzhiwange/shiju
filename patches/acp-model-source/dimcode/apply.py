"""Patch only the pinned DimCode Darwin arm64 ACP image."""
import hashlib
from pathlib import Path
import sys

SOURCE_SHA256 = "0d6298cc8881b15ae8cdf1901b6f39674b74dbf5c14caedadfe509ab5ae114b0"


def patch(data, start, end, replacements):
    start, end = start.encode(), end.encode()
    if data.count(start) != 1:
        raise RuntimeError("DimCode patch anchor mismatch")
    left = data.index(start)
    right = data.index(end, left + len(start))
    segment = data[left:right].decode()
    for old, new in replacements:
        if segment.count(old) != 1:
            raise RuntimeError("DimCode patch replacement mismatch")
        segment = segment.replace(old, new)
    result = "\n".join(line.strip() for line in segment.splitlines()).encode() + b"\n"
    if start == b"const permFlowCtx = {":
        result = result.replace(b": ", b":").replace(b"\n", b"")
    if len(result) > right - left:
        raise RuntimeError(f"DimCode patch exceeds its Bun bundle slot: {start!r} ({len(result)} > {right-left})")
    return data[:left] + result.ljust(right - left) + data[right:]


def apply(source, target):
    data = source.read_bytes()
    if hashlib.sha256(data).hexdigest() != SOURCE_SHA256:
        raise RuntimeError("DimCode upstream executable digest mismatch")
    # Use session runtime settings only during the current ACP prompt. A fork
    # or subagent has a different ID and receives no inherited selection.
    data = patch(data, "async function ensureSkills(settings, cwd)", "var DEFAULT_REGISTRY", [
        ("""async function ensureSkills(settings, cwd) {
  const generation = skillCacheGeneration;
  const hostUser = await ensureHostUserSkills(settings);
  if (generation !== skillCacheGeneration)
    return ensureSkills(settings, cwd);
  const workspace = await ensureWorkspaceSkills(settings, cwd, hostUser);
  if (generation !== skillCacheGeneration)
    return ensureSkills(settings, cwd);
  return [...hostUser, ...workspace];
}""", """
async function ensureSkills(s,c){
 const g=skillCacheGeneration,u=await ensureHostUserSkills(s);
 if(g!==skillCacheGeneration)return ensureSkills(s,c);
 const w=await ensureWorkspaceSkills(s,c,u);
 if(g!==skillCacheGeneration)return ensureSkills(s,c);
 const a=[...u,...w],r=s.shijuRoot;
 return r?[...a,...await scanDirectory(r,"host",new Set(a.map(x=>x.name)),MAX_SCAN_FILES,{value:0})]:a;
}
"""),
    ])
    data = patch(data, "function getSettings(context) {\n  return context.settings", "var skillPlugin =", [
        ("function getSettings(context) {\n  return context.settings ?? {};\n}", """
function getSettings(c){return {...c.settings,shijuRoot:globalThis.shRoot?.get(c.sessionId)}}
"""),
        ('throw new PluginSkillError2("context.cwd is required for skill discovery");', 'throw new PluginSkillError2("Skill cwd required");'),
    ])
    data = patch(data,
        "const runInput = {\n      ...await mapAcpPromptToRunInput(prompt, input.runtime.agentSession),",
        "    await input.runtime.eventDispatchChain;", [
        ("""const runInput = {
      ...await mapAcpPromptToRunInput(prompt, input.runtime.agentSession),
      ...input.params.messageId == null ? {} : { messageId: input.params.messageId },
      providerInvocationConfig: await buildAcpProviderInvocationConfig({
        context: input.context,
        agentSession: input.runtime.agentSession,
        cwd: input.runtime.cwd
      })
    };""", """
const r=input.runtime,meta=input.params._meta?.["shiju.dimcode"];
const on=meta?.version===1,root=meta?.root;
if(on&&root!=null){
 if(typeof root!=="string"||!path2.isAbsolute(root))throw invalidParams("Invalid skill root");
 (globalThis.shRoot??=new Map).set(r.sessionId,root);
}
const runInput={
 ...await mapAcpPromptToRunInput(prompt,r.agentSession),
 messageId:input.params.messageId??undefined,
 metadata:on?{shId:r.sessionId,shMcp:r.mcpRuntime.servers.map(server=>server.serverId)}:undefined,
 providerInvocationConfig:await buildAcpProviderInvocationConfig({context:input.context,agentSession:r.agentSession,cwd:r.cwd})
};"""),
    ])
    data = patch(data, "  } finally {\n    uninstallElicitation?.();", "function acpGoalObjectiveFromRunInput", [
        ("if (input.runtime.activePrompt === activePrompt) {", """
if(input.runtime.activePrompt===activePrompt){
 globalThis.shRoot?.delete(input.runtime.sessionId);"""),
    ])
    data = patch(data, "const permFlowCtx = {", "  } catch (error) {", [
        ("sessionId: ctx.sessionId,", "shMcp:ctx.runMetadata?.shId===ctx.sessionId?ctx.runMetadata.shMcp:undefined,sessionId:ctx.sessionId,"),
    ])
    data = patch(data, "async function* runPermissionFlow(", "function filterCustomArgs(", [
        ("  let handoff;", """
if(ctx.shMcp&&permissionDecision.decision==="deny"&&permissionDecision.reason==="source: default"){
 if(toolDefinition?.name==="skill"&&toolDefinition.metadata?.sourcePluginId==="skill-loader"
   &&(await ctx.session.getPluginSettings("skill-loader"))?.enabled!==false)return;
 const source=buildToolSourceInfo(toolDefinition);
 if(source?.pluginId==="mcp-client"&&ctx.shMcp.includes(source.mcpServerId)){
  const answer=yield* askPermission(ctx,effectiveToolCall,scope,permissionRequestId,{reason:"ACP MCP permission"},undefined,source);
  if(answer.decision==="allow"&&!answer.remember)return;
  return await handleDeny(ctx,toolCall,scope,permissionRequestId,answer.reason);
 }
}
let handoff;"""),
    ])
    with target.open("xb") as output:
        output.write(data)
    target.chmod(0o700)


if __name__ == "__main__":
    apply(*map(Path, sys.argv[1:]))
