#!/usr/bin/env python3
"""Apply the host-only Linux MCP overlay to the reviewed iFlow v2 engine."""
import hashlib
from pathlib import Path
import sys

V2_ENGINE_SHA256 = "e8590e0e5923809928c2c831272fe4d05f917a13e95dfd862fb1c52c4634cd57"


LINUX_HELPERS = '''
const shijuIFlowCatalog = Symbol("host MCP projection");
const shijuIFlowOpening = new WeakSet();
const shijuIFlowLiveServers = new WeakMap();

function shijuIFlowManaged() {
    return shijuIFlowLaunch.managed;
}

function shijuIFlowName(name) {
    return typeof name === "string" && /^[A-Za-z0-9_-]{1,64}$/.test(name) &&
        !Object.hasOwn(Object.prototype, name);
}

function shijuIFlowNativeDirectory(cwd) {
    let directory, file;
    try {
        const flags = x1.constants;
        directory = x1.openSync(y5.join(cwd, ".iflow"),
            flags.O_RDONLY | flags.O_DIRECTORY | flags.O_NOFOLLOW);
        file = x1.openSync("/proc/self/fd/" + directory + "/settings.json",
            flags.O_RDONLY | flags.O_NOFOLLOW | flags.O_NONBLOCK);
        if (!x1.fstatSync(file).isFile()) throw El.invalidRequest("Invalid iFlow project settings");
        const limit = 1024 * 1024;
        const data = Buffer.alloc(limit + 1);
        let size = 0;
        while (size < data.length) {
            const count = x1.readSync(file, data, size, data.length - size, null);
            if (!count) break;
            size += count;
        }
        if (size > limit) throw El.invalidRequest("iFlow project settings exceed size limit");
        const settings = JSON.parse((0,Uht.default)(data.toString("utf8", 0, size)));
        return Object.keys(settings?.mcpServers ?? {}).filter(shijuIFlowName)
            .map(name => ({name, scope: "project"}));
    } catch (error) {
        if (error.code === "ENOENT") return [];
        throw El.invalidRequest("Cannot read iFlow project MCP directory");
    } finally {
        if (file !== undefined) x1.closeSync(file);
        if (directory !== undefined) x1.closeSync(directory);
    }
}

function shijuIFlowRelay() {
    const {endpoint, token} = shijuIFlowLaunch;
    if (!endpoint || !token || !/^[A-Za-z0-9_-]+$/.test(token)) return null;
    try {
        const url = new URL(endpoint);
        if (url.protocol !== "http:" || !["127.0.0.1", "[::1]"].includes(url.hostname) ||
            !url.port || url.pathname !== "/" || url.search || url.hash ||
            url.username || url.password || endpoint !== url.origin) return null;
        return {origin: url.origin, authorization: "Bearer " + token};
    } catch {
        return null;
    }
}

function shijuIFlowRelayUrl(name, relay) {
    return relay.origin + "/shiju-vtcode-mcp/" + name;
}

function shijuIFlowProject(servers, native) {
    const catalog = Object.create(null);
    const relay = shijuIFlowRelay();
    const nativeNames = new Set(native.map(server => server.name));
    for (const server of servers) {
        const name = server.name;
        const origin = server._meta?.["shiju.iflow.mcp-v4"];
        const project = origin === "project";
        if (!relay || !shijuIFlowName(name) || Object.hasOwn(catalog, name) ||
            (!project && origin !== "mounted") || project !== nativeNames.has(name) ||
            server.type !== "http" || server.url !== shijuIFlowRelayUrl(name, relay) ||
            server.command || server.args || server.env || server.cwd || server.httpUrl ||
            server.tcp || server.trust || server.authProviderType || server.oauth ||
            server.includeTools || server.excludeTools || server.extensionName ||
            !Array.isArray(server.headers) || server.headers.length !== 1 ||
            server.headers[0].name.toLowerCase() !== "authorization" ||
            server.headers[0].value !== relay.authorization) {
            throw El.invalidRequest("iFlow MCP requires a scoped host projection");
        }
        // Host catalog already enforces service and tool allow/deny policy.
        catalog[name] = {type: "http", url: server.url};
    }
    for (const [name, server] of Object.entries(catalog)) {
        shijuIFlowLiveServers.set(server, name);
    }
    return catalog;
}

function shijuIFlowIdentity(server, tool) {
    const identity = "mcp__" + server + "__" + tool;
    if (!shijuIFlowName(server) || !shijuIFlowName(tool) || identity.length > 64) {
        throw El.invalidRequest("Invalid iFlow relay tool identity");
    }
    return identity;
}

function shijuIFlowValidateHistory(history, sessionId) {
    if (shijuIFlowManaged() &&
        (!history || history.sessionId !== sessionId || !Array.isArray(history.chatHistory))) {
        throw El.invalidRequest("Cannot restore iFlow managed session history");
    }
}

function shijuIFlowCallParams(call) {
    const params = {name: call.name, arguments: call.args};
    if (shijuIFlowManaged()) {
        if (typeof call.shijuCallId !== "string" || !call.shijuCallId.trim()) {
            throw El.invalidRequest("Missing iFlow MCP call identity");
        }
        params._meta = {shijuCallId: call.shijuCallId};
    }
    return params;
}

'''
LINUX_TRANSPORT = '''
if (shijuIFlowManaged()) {
    const relay = shijuIFlowRelay();
    if (!relay || shijuIFlowLiveServers.get(e) !== t || e.type !== "http" ||
        e.url !== shijuIFlowRelayUrl(t, relay)) {
        throw El.invalidRequest("iFlow cannot connect directly to an MCP server");
    }
    return new RM(new URL(e.url), {
        fetch: (url, options) => {
            if (shijuIFlowLiveServers.get(e) !== t || String(url) !== e.url) {
                throw El.invalidRequest("iFlow relay endpoint is retired or changed");
            }
            return fetch(url, {...options, redirect: "error"});
        },
        requestInit: {headers: {Authorization: relay.authorization}}
    });
}
'''
LINUX_NEW_SESSION = '''
async newSession(request) {
    if (!shijuIFlowManaged()) return this.shijuOriginalNewSession(request);
    if (request.sessionId || shijuIFlowOpening.has(this) || this.sessions.size) {
        throw El.invalidRequest("iFlow managed process already owns a session");
    }
    shijuIFlowOpening.add(this);
    try {
        return await this.shijuOriginalNewSession(request);
    } finally {
        shijuIFlowOpening.delete(this);
    }
}
'''
LINUX_RESTORE = '''
async loadSession(request) {
    if (!shijuIFlowManaged()) return this.shijuOriginalLoadSession(request);
    const id = request.sessionId;
    if (shijuIFlowOpening.has(this) || (this.sessions.size && !this.sessions.has(id))) {
        throw El.invalidRequest("iFlow managed process already owns a session");
    }
    shijuIFlowOpening.add(this);
    const previous = this.sessions.get(id);
    this.sessions.delete(id);
    previous?.pendingPrompt?.abort();
    try {
        if (previous) {
            const config = previous.getConfig();
            for (const server of Object.values(config.getMcpServers() ?? {})) {
                shijuIFlowLiveServers.delete(server);
            }
            const registry = await config.getToolRegistry();
            await registry.getMcpClientManager().stop();
        }
        return await this.shijuOriginalLoadSession(request);
    } finally {
        shijuIFlowOpening.delete(this);
    }
}
'''

def replace(source, old, new):
    if source.count(old) != 1:
        raise SystemExit("iFlow Linux patch context differs")
    return source.replace(old, new)


def patch_engine(data):
    if hashlib.sha256(data).hexdigest() != V2_ENGINE_SHA256:
        raise SystemExit("iFlow Linux overlay requires the reviewed v2 engine")
    source = data.decode()
    startup = """const shijuIFlowLaunch = Object.freeze({
    managed: process.platform === "linux" && Boolean(process.env.SHIJU_IFLOW_MODEL_LOCK),
    endpoint: process.env.SHIJU_IFLOW_MCP_RELAY,
    token: process.env.SHIJU_IFLOW_MCP_TOKEN,
});
"""
    position = source.find("\n") + 1 if source.startswith("#!") else 0
    source = source[:position] + startup + source[position:]
    source = replace(source, 'function qht(){let t=nnu(process.cwd());',
        'function qht(){if(shijuIFlowLaunch.managed)return;let t=nnu(process.cwd());')
    managed = "shijuIFlowManaged()"

    # Only this adapter can mark settings as the live host projection. Every
    # startup/config rebuild otherwise sees an empty executable MCP directory.
    discovery = 'function ncu(t,e){let r={...t.mcpServers||{}};'
    source = replace(source, discovery, LINUX_HELPERS +
        'function ncu(t,e){if(shijuIFlowManaged())return t[shijuIFlowCatalog]??{};'
        'let r={...t.mcpServers||{}};')
    schema = 'g2t=qe.object({name:qe.string().optional(),'
    source = replace(source, schema, schema + '_meta:qe.record(qe.unknown()).optional(),')
    headers = 'headers:qe.record(qe.string(),qe.string()).optional(),tcp:'
    source = replace(source, headers,
        'headers:qe.union([qe.record(qe.string(),qe.string()),'
        'qe.array(qe.object({name:qe.string(),value:qe.string()}))]).optional(),tcp:')
    merge = 'async newSessionConfig(e,r,n,o,s,a,u){let c={...this.settings.merged.mcpServers};'
    source = replace(source, merge, '''async newSessionConfig(e,r,n,o,s,a,u){
const shijuManaged = shijuIFlowManaged();
const shijuServers = shijuManaged
    ? shijuIFlowProject(n, shijuIFlowNativeDirectory(r))
    : undefined;
let c=shijuManaged?shijuServers:{...this.settings.merged.mcpServers};
if(shijuManaged){n=[];o=undefined;s=undefined;a=undefined;}
''')
    config = 'let m={...this.settings.merged,mcpServers:c,hooks:s,'
    source = replace(source, config,
        'let m={...this.settings.merged,[shijuIFlowCatalog]:shijuServers,mcpServers:c,hooks:s,')
    directory = 'async getAvailableMcpServers(){let e=[];'
    source = replace(source, directory, directory +
        'if(shijuIFlowManaged())return shijuIFlowNativeDirectory(this.config.getTargetDir());')
    commands = 'toolDiscoveryCommand:t.toolDiscoveryCommand,toolCallCommand:t.toolCallCommand,mcpServerCommand:t.mcpServerCommand,mcpServers:g,'
    source = replace(source, commands,
        'toolDiscoveryCommand:shijuIFlowManaged()?void 0:t.toolDiscoveryCommand,'
        'toolCallCommand:shijuIFlowManaged()?void 0:t.toolCallCommand,'
        'mcpServerCommand:shijuIFlowManaged()?void 0:t.mcpServerCommand,mcpServers:g,')
    source = replace(source, 'hookManager:t.hookManager??!0,',
        'hookManager:shijuIFlowManaged()?!1:t.hookManager??!0,')
    transport = 'async function _9e(t,e,r){'
    source = replace(source, transport, transport + LINUX_TRANSPORT)
    capabilities = 'agentCapabilities:{loadSession:!0,promptCapabilities:'
    source = replace(source, capabilities,
        'agentCapabilities:{loadSession:!0,...(shijuIFlowManaged()&&shijuIFlowRelay()?'
        '{mcpCapabilities:{http:!0}}:{}),promptCapabilities:')
    response_schema = 's8u=qe.object({loadSession:qe.boolean().optional(),'
    source = replace(source, response_schema, response_schema +
        'mcpCapabilities:qe.object({http:qe.boolean().optional()}).optional(),')

    # The previous registry and any outstanding approval become unusable as
    # soon as load starts, including when the replacement projection is invalid.
    source = replace(source, 'async loadSession(e){let{sessionId:r,cwd:n,mcpServers:o,',
        LINUX_RESTORE + 'async shijuOriginalLoadSession(e){let{sessionId:r,cwd:n,mcpServers:o,')
    new_session = 'async newSession(e){let r=e.cwd,n=e.mcpServers,'
    source = replace(source, new_session, LINUX_NEW_SESSION +
        'async shijuOriginalNewSession(e){let r=e.cwd,n=e.mcpServers,')
    source = replace(source, 'this.newSession({sessionId:r,cwd:n,mcpServers:o||[],',
        'this.shijuOriginalNewSession({sessionId:r,cwd:n,mcpServers:o||[],')
    source = replace(source, 'let m=await this.sessionStorage.loadSession(r);if(!m)',
        'let m=await this.sessionStorage.loadSession(r);shijuIFlowValidateHistory(m,r);if(!m)')
    source = replace(source, 'A=new v2t(c,p,this.client,this,b);this.sessions.set(c,A);',
        'A=new v2t(c,p,this.client,this,b);'
        'if(shijuIFlowManaged())await A.saveSessionState();this.sessions.set(c,A);')

    # Preserve the ACP call ID all the way to the host-side tools/call frame.
    source = replace(source, 'o=[{name:this.serverToolName,args:n}];',
        'o=[{name:this.serverToolName,args:n,shijuCallId:r}];')
    source = replace(source, 'let a=await o.callTool({name:n.name,arguments:n.args},void 0,s);',
        'let a=await o.callTool(shijuIFlowCallParams(n),void 0,s);')

    # Keep PLAN's native deny rule. In managed Linux sessions, even DEFAULT
    # reads and tools with native remembered grants must ask the host.
    old = 'let d=this.config.getApprovalMode();if(d!==dn.YOLO)if(d===dn.PLAN)'
    source = replace(source, old,
        f'let d=this.config.getApprovalMode(),shijuManaged={managed};'
        'if(d!==dn.YOLO&&(!shijuManaged||d===dn.PLAN))if(d===dn.PLAN)')
    execute = 'let f=await m.execute({callId:o,...s},e),'
    source = replace(source, execute, '''
if (shijuManaged) {
    if (e?.aborted || this.agent.sessions.get(this.id) !== this || Object.hasOwn(s, "callId") ||
        n.name.toLowerCase() === "task" || m.kind === "execute") {
        return u(new Error("iFlow tool execution unavailable in this prompt"));
    }
    const tool = m instanceof bg ? shijuIFlowIdentity(m.serverName, m.serverToolName) : n.name;
    const reply = await this.client.requestPermission({
        sessionId: this.id,
        options: E2t,
        _meta: {"shiju.iflow/tool-v3": tool},
        toolCall: {
            toolCallId: o,
            status: "pending",
            title: m.getDescription(s),
            rawInput: s,
            locations: m.toolLocations(s),
            kind: m.kind
        }
    });
    if (e?.aborted || this.agent.sessions.get(this.id) !== this ||
        reply?.outcome?.outcome !== "selected" ||
        reply.outcome.optionId !== cn.ProceedOnce) {
        return u(new Error("Tool execution rejected by user"));
    }
}
''' + execute)

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
