#!/usr/bin/env python3
"""Apply the Linux approval overlay to the reviewed iFlow v2 engine."""
import hashlib
from pathlib import Path
import sys

V2_ENGINE_SHA256 = "e8590e0e5923809928c2c831272fe4d05f917a13e95dfd862fb1c52c4634cd57"


def replace(source, old, new):
    if source.count(old) != 1:
        raise SystemExit("iFlow Linux patch context differs")
    return source.replace(old, new)


def patch_engine(data):
    if hashlib.sha256(data).hexdigest() != V2_ENGINE_SHA256:
        raise SystemExit("iFlow Linux overlay requires the reviewed v2 engine")
    source = data.decode()
    managed = 'process.platform==="linux"&&Boolean(process.env.SHIJU_IFLOW_MODEL_LOCK)'

    # Preserve host policy metadata through the vendor's ACP schema. v3 only
    # accepts unfiltered stdio mounts explicitly projected by the host.
    schema = 'g2t=qe.object({name:qe.string().optional(),'
    source = replace(source, schema, schema + '_meta:qe.record(qe.unknown()).optional(),')
    merge = 'async newSessionConfig(e,r,n,o,s,a,u){let c={...this.settings.merged.mcpServers};'
    source = replace(source, merge, merge + f'''
if({managed}){{
    for (const server of n) {{
        const policy = server._meta?.["shiju.iflow.toolPolicy"];
        if (!policy || !Array.isArray(policy.allow) || !Array.isArray(policy.deny) ||
            policy.allow.length || policy.deny.length) {{
            throw El.invalidRequest("iFlow cannot enforce this host MCP tool policy");
        }}
        if (!server.name || !server.command || (server.type && server.type !== "stdio") ||
            server.url || server.httpUrl || server.tcp || server.trust ||
            server.authProviderType || server.oauth) {{
            throw El.invalidRequest("iFlow managed mounts require untrusted stdio");
        }}
        if (Object.hasOwn(c, server.name) || server.name in Object.prototype ||
            n.filter(item => item.name === server.name).length !== 1) {{
            throw El.invalidRequest("Ambiguous iFlow MCP server name");
        }}
    }}
}}
''')

    # Keep PLAN's native deny rule. In managed Linux sessions, even DEFAULT
    # reads and tools with native remembered grants must ask the host.
    old = 'let d=this.config.getApprovalMode();if(d!==dn.YOLO)if(d===dn.PLAN)'
    source = replace(source, old,
        f'let d=this.config.getApprovalMode(),shijuManaged={managed};'
        'if(d!==dn.YOLO&&(!shijuManaged||d===dn.PLAN))if(d===dn.PLAN)')
    execute = 'let f=await m.execute({callId:o,...s},e),'
    source = replace(source, execute, '''
if (shijuManaged) {
    if (e?.aborted || Object.hasOwn(s, "callId") ||
        n.name.toLowerCase() === "task" || m.kind === "execute") {
        return u(new Error("iFlow tool execution unavailable in this prompt"));
    }
    const reply = await this.client.requestPermission({
        sessionId: this.id,
        options: E2t,
        _meta: {"shiju.iflow/tool-v3": n.name},
        toolCall: {
            toolCallId: o,
            status: "pending",
            title: m.getDescription(s),
            rawInput: s,
            locations: m.toolLocations(s),
            kind: m.kind
        }
    });
    if (e?.aborted || reply?.outcome?.outcome !== "selected" ||
        reply.outcome.optionId !== cn.ProceedOnce) {
        return u(new Error("Tool execution rejected by user"));
    }
}
''' + execute)

    # Vendor stdio inherited the full model relay environment before discovery.
    stdio = 'env:{...process.env,...e.env||{}},cwd:e.cwd,stderr:"pipe"'
    source = replace(source, stdio,
        f'env:{managed}?{{...e.env,HOME:process.env.IFLOW_HOME}}:'
        '{...process.env,...e.env||{}},cwd:e.cwd,stderr:"pipe"')

    # These prompt shortcuts can run outside the central tool dispatcher.
    prompt = 'async prompt(e){this.pendingPrompt?.abort();let r=new AbortController;'
    source = replace(source, prompt, prompt + f'''
if ({managed} && e.prompt.some((part, index) =>
    (index === 0 && part.type === "text" && /^[/$]/.test(part.text.trim())) ||
    part.type === "resource" || part.type === "resource_link")) {{
    throw El.invalidRequest("iFlow managed prompts require explicit tool calls");
}}
''')
    return source.encode()


if __name__ == "__main__":
    engine = Path(sys.argv[1]) / "bundle/iflow.js"
    engine.write_bytes(patch_engine(engine.read_bytes()))
