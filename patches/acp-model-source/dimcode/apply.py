"""Patch only the pinned DimCode Darwin arm64 payload in a private install stage."""
import hashlib
from pathlib import Path
import sys

SOURCE_SHA256 = "0d6298cc8881b15ae8cdf1901b6f39674b74dbf5c14caedadfe509ab5ae114b0"


def replace_span(data, start, end, replacement):
    start, end = start.encode(), end.encode()
    if data.count(start) != 1:
        raise RuntimeError("DimCode patch anchor mismatch")
    begin = data.index(start)
    finish = data.index(end, begin + len(start))
    replacement = "".join(line.strip() for line in replacement.splitlines()).encode()
    if len(replacement) > finish - begin:
        raise RuntimeError("DimCode patch exceeds its Bun bundle slot")
    # The embedded bundle's byte offsets are fixed.
    return data[:begin] + replacement.ljust(finish - begin) + data[finish:]


def apply(source, target):
    data = source.read_bytes()
    if hashlib.sha256(data).hexdigest() != SOURCE_SHA256:
        raise RuntimeError("DimCode upstream executable digest mismatch")

    data = replace_span(data,
        'function filterToolsetByMetadata(toolset, metadata) {',
        'function applyModelCapabilityAwareToolDefinitions',
        '''
        function filterToolsetByMetadata(t,m){
            const s=readStringArrayMetadata(m,"tools"),a=m?.shijuAcpMcp;
            return s||a?t.filter(x=>(!s||s.includes(x.name))&&(!a||a.includes(x.metadata?.mcpServerId))):t;
        }
        ''')
    data = replace_span(data,
        'const runInput = {\n      ...await mapAcpPromptToRunInput(prompt, input.runtime.agentSession),',
        '    abortController.signal.throwIfAborted();\n    await applyAcpStoredSessionMode',
        '''
        const r=input.runtime,m=r.mcpRuntime;
        const runInput={
            ...await mapAcpPromptToRunInput(prompt,r.agentSession),
            messageId:input.params.messageId??undefined,
            metadata:m.servers.length||m.warnings[0]?{shijuAcpMcp:m.servers.map(x=>x.serverId)}:undefined,
            providerInvocationConfig:await buildAcpProviderInvocationConfig({context:input.context,agentSession:r.agentSession,cwd:r.cwd})
        };
        ''')
    data = replace_span(data,
        'async function* runToolCall(ctx, toolCall) {',
        '  if (preToolResult?.block) {',
        '''
        async function* runToolCall(ctx,toolCall){
            if(ctx.isInterruptedOrStopping())return {success:false,error:createUserInterruptError()};
            const toolDef=ctx.toolRuntime.getEffectiveDefinition(toolCall.name),s=ctx.runMetadata?.shijuAcpMcp;
            if(s&&!s.includes(toolDef?.metadata?.mcpServerId)){
                const error=new Error("Outside MCP scope");
                yield* completeDeclinedToolCall(ctx,toolCall,toolCall.input,error,error.message);
                return {success:false,error};
            }
            let effectiveInput=toolCall.input,preToolResult;
            try{
                preToolResult=await ctx.hooks.dispatch("PreToolUse",{
                    toolCallId:toolCall.id,toolName:toolCall.name,toolInput:toolCall.input,
                    toolDefinition:toolDef??{name:toolCall.name,description:"",inputSchema:{type:"object"},source:"dynamic"},
                    permissionSnapshot:null
                },ctx.hookContext);
            }catch(e){
                const error=toError3(e);
                yield* completeFailedToolCall(ctx,toolCall,effectiveInput,error);
                return {success:false,error};
            }
            if(preToolResult?.toolResult){
                yield* completeSuccessfulToolCall(ctx,toolCall,preToolResult.toolResult);
                return {success:true,result:preToolResult.toolResult};
            }
            if(preToolResult?.toolInputPatch)effectiveInput=applyToolInputPatch(effectiveInput,preToolResult.toolInputPatch);
        ''')
    with target.open("xb") as output:
        output.write(data)
    target.chmod(0o700)


if __name__ == "__main__":
    apply(*map(Path, sys.argv[1:]))
