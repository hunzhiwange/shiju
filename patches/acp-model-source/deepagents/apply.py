"""Patch only the reviewed DeepAgents ACP CLI in a disposable managed install."""
import hashlib
from pathlib import Path
import shutil
import sys

root = Path(sys.argv[1])
entry = root / 'dist/cli.js'
source = entry.read_text()
expected = '6574016c7212831f66c79a2412e7de56c75363293c182fdc311749b8bb251238'
if hashlib.sha256(entry.read_bytes()).hexdigest() != expected:
    raise SystemExit('Unreviewed DeepAgents CLI; refusing patch')

def replace(old, new, count=1):
    global source
    if source.count(old) != count:
        raise SystemExit('DeepAgents patch context mismatch')
    source = source.replace(old, new)

replace('import path from "node:path";',
        'import path from "node:path";\nimport { hostToolGuard } from "./shiju-guard.mjs";')
replace('return super.read(filePath, offset, limit);', 'throw new Error("Host file read unavailable or denied");', 2)
replace('return super.write(filePath, content);', 'throw new Error("Host file write unavailable or denied");', 2)
replace('import { hostToolGuard } from "./shiju-guard.mjs";',
        'import { hostToolGuard } from "./shiju-guard.mjs";\nimport { discoverMcp, mcpSelection } from "./shiju-mcp.mjs";\nimport { ChatOpenAI } from "@langchain/openai";')
replace('model: config.model,',
        'model: config.model?.startsWith("openai:") ? new ChatOpenAI({model: config.model.slice(7), useResponsesApi: false}) : config.model,')
replace('middleware: config.middleware,',
        'middleware: [...(config.middleware ?? []), hostToolGuard(this, agentName)],')
replace('const decision = await this.requestToolPermission(session, conn, toolCall);',
        'const decision = "allow";') # Actual authorization occurs inside wrapToolCall.
replace('if (optionId === "reject-once") return "reject";\n\t\t\treturn "allow";',
        'if (optionId === "allow-once") return "allow";\n\t\t\treturn "reject";')
replace('this.log("Permission request failed:", err);\n\t\t\treturn "allow";',
        'this.log("Permission request failed");\n\t\t\treturn "reject";')
# Recheck the host on every invocation, including after an "always" response.
replace('const cached = session.permissionDecisions.get(toolCall.name);\n\t\tif (cached === "allow_always") return "allow";\n\t\tif (cached === "reject_always") return "reject";', '')
replace('session.permissionDecisions.set(toolCall.name, "allow_always");', '')
replace('session.permissionDecisions.set(toolCall.name, "reject_always");', '')
replace('async handleNewSession(params, conn) {',
        'async handleNewSession(params, conn) {\n'
        '\t\tif (this.sessions.size) throw new Error("One workspace identity per ACP process");\n'
        '\t\tif (fs.realpathSync(params.cwd) !== this.workspaceRoot) throw new Error("ACP workspace differs from launch workspace");')
replace('this.sessions.set(sessionId, session);',
        'session.mcpSelection = mcpSelection(params.mcpServers);\n'
        '\t\tthis.sessions.set(sessionId, session);')
replace('if (!this.agents.has(agentName)) this.createAgent(agentName);',
        'try {\n'
        '\t\t\tsession.mcpTools = await discoverMcp(session.mcpSelection, this.workspaceRoot);\n'
        '\t\t\tif (!this.agents.has(agentName)) this.createAgent(agentName, session.mcpTools);\n'
        '\t\t} catch (error) {\n'
        '\t\t\tthis.sessions.delete(sessionId);\n'
        '\t\t\tthrow error;\n'
        '\t\t}')
replace('this.log("Loading session:", {',
        'if (JSON.stringify(mcpSelection(params.mcpServers)) !== JSON.stringify(session.mcpSelection)) {\n'
        '\t\t\tthrow new Error("DeepAgents MCP selection differs from current session");\n'
        '\t\t}\n'
        '\t\tthis.log("Loading session:", {')
replace('createAgent(agentName) {', 'createAgent(agentName, mcpTools) {')
replace('tools: config.tools,',
        'tools: [...(config.tools ?? []), ...mcpTools.values()],')
replace('const isError = message.status === "error" || typeof message.content',
        'const isError = message.status === "error" || !session.mcpTools?.has(toolCall.name) && typeof message.content')
replace('status: toolCall.status,\n\t\t\t\t...locations ? { locations } : {},',
        'status: toolCall.status,\n\t\t\t\trawInput: toolCall.args,\n\t\t\t\t...locations ? { locations } : {},')
replace('toolCall.status = isError ? "error" : "completed";',
        'toolCall.status = isError ? (session.mcpTools?.has(toolCall.name) ? "failed" : "error") : "completed";')
replace('status: "pending",\n\t\t\t\t\tinput: toolCall.args',
        'status: "pending",\n\t\t\t\t\trawInput: toolCall.args,\n\t\t\t\t\tinput: toolCall.args')
replace('update.output = resultText;',
        'update.output = resultText;\n\t\t\tupdate.rawOutput = resultText;')
replace('backend: new FilesystemBackend({ rootDir: workspaceRoot }),', '')
replace('async handleLoadSession(params, conn) {',
        'async handleLoadSession(params, conn) {\n'
        '\t\tif (fs.realpathSync(params.cwd) !== this.workspaceRoot) throw new Error("ACP workspace differs from launch workspace");')
replace('async handleCancel(params) {',
        'async handleCancel(params) {\n\t\tif (!this.sessions.has(params.sessionId)) return;')
replace('this.workspaceRoot = options.workspaceRoot ?? process.cwd();',
        'this.workspaceRoot = fs.realpathSync(options.workspaceRoot ?? process.cwd());')
replace('const defaultMemoryPaths = [path.join(workspaceRoot, ".deepagents", "AGENTS.md"), path.join(workspaceRoot, "AGENTS.md")];',
        'const defaultMemoryPaths = [];')
replace('const defaultSkillPaths = [path.join(workspaceRoot, ".deepagents", "skills"), path.join(workspaceRoot, "skills")];',
        'const defaultSkillPaths = [];')
replace('return new FilesystemBackend({ rootDir: this.workspaceRoot });',
        'throw new Error("DeepAgents requires host-mediated filesystem capabilities");')
entry.write_text(source)
shutil.copy2(Path(__file__).with_name('guard.mjs'), root / 'dist/shiju-guard.mjs')

for name in ('mcp.mjs', 'mcp.py'):
    shutil.copy2(Path(__file__).with_name(name), root / ('dist/shiju-' + name))
shutil.copy2(Path(__file__).parents[1] / 'cortex-code/mcp_bridge.py', root / 'dist/shiju-stdio.py')
