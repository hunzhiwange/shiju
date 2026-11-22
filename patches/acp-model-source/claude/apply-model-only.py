"""Build the model-only candidate from the exact Claude ACP 0.26.0 npm source."""

import hashlib
from pathlib import Path
import sys


def replace_once(source, before, after):
    if source.count(before) != 1:
        raise ValueError("unexpected Claude ACP source anchor")
    return source.replace(before, after)


def apply(package: Path):
    dist = package / "dist"
    adapter = dist / "acp-agent.js"
    source = adapter.read_bytes()
    if hashlib.sha256(source).hexdigest() != "00e66a056cc39ba087dffe63da3b62d27fcff6694e6eee5fe3b25961e6218ddd":
        raise ValueError("Claude model-only requires the exact 0.26.0 ACP adapter")

    source = source.decode()
    start = "        const input = new Pushable();\n        const settingsManager"
    end = "        const q = query({\n"
    if source.count(start) != 1 or source.count(end) != 1:
        raise ValueError("unexpected Claude SDK session construction")
    begin = source.index(start)
    finish = source.index(end, begin)
    source = source[:begin] + """        const input = new Pushable();
        const settingsManager = new SettingsManager(params.cwd);
        const abortController = new AbortController();
        const options = modelOnlyOptions(params, creationOpts);
        options.sessionId = sessionId;
        options.abortController = abortController;
        const permissionMode = options.permissionMode;
""" + source[finish:]

    start = "            agentCapabilities: {\n"
    end = "            agentInfo: {\n"
    if source.count(start) != 1 or source.count(end) != 1:
        raise ValueError("unexpected Claude ACP initialize response")
    begin = source.index(start)
    source = source[:begin] + "            agentCapabilities: { sessionCapabilities: { close: {} } },\n" + source[source.index(end, begin):]
    source = replace_once(
        source,
        "            authMethods: [...terminalAuthMethods, ...(supportsGatewayAuth ? [gatewayAuthMethod] : [])],",
        "            authMethods: [],",
    )
    source = replace_once(
        source,
        "        const models = await getAvailableModels(q, initializationResult.models, settingsManager);",
        """        let services;
        try {
            services = await q.mcpServerStatus();
        } catch (error) {
            abortController.abort();
            throw error;
        }
        if (!Array.isArray(services) || services.length !== 0) {
            abortController.abort();
            throw new Error("Claude model-only engine reported an MCP server");
        }
        const models = {
            currentModelId: options.model,
            availableModels: [{ modelId: options.model, name: options.model }],
        };""",
    )
    source = replace_once(
        source,
        "        const commands = await session.query.supportedCommands();",
        "        const commands = [];",
    )
    source = replace_once(
        source, "    async authenticate(_params) {",
        "    async disabledAuthenticate(_params) {",
    )
    for method in ("authenticate", "listSessions", "unstable_forkSession",
                   "unstable_resumeSession", "loadSession"):
        anchor = f"    async {method}("
        if method != "authenticate":
            source = replace_once(source, anchor, f"    async disabled_{method}(")
        source = replace_once(
            source, "    async createSession(params, creationOpts = {}) {",
            f'    async {method}() {{ throw new Error("Claude model-only operation unavailable"); }}\n'
            "    async createSession(params, creationOpts = {}) {",
        )
    source = replace_once(
        source, "    async prompt(params) {", "    async prompt(params) {\n        checkPrompt(params);",
    )
    for method in ("unstable_setSessionModel", "setSessionConfigOption", "setSessionMode"):
        anchor = f"    async {method}(params) {{"
        source = replace_once(
            source, anchor,
            anchor + '\n        rejectMutation(params);',
        )
    source = replace_once(
        source, "            availableModes,",
        '            availableModes: availableModes.filter((mode) => mode.id === "default"),',
    )
    source = replace_once(
        source, '        if (!this.gatewayAuthMeta &&\n            fs.existsSync(path.resolve(os.homedir(), ".claude.json.backup")) &&\n            !fs.existsSync(path.resolve(os.homedir(), ".claude.json"))) {\n            throw RequestError.authRequired();\n        }\n',
        "",
    )
    helper = Path(__file__).with_name("model-only.mjs")
    (dist / helper.name).write_bytes(helper.read_bytes())
    adapter.write_text(
        'import { checkPrompt, modelOnlyOptions, rejectMutation } from "./model-only.mjs";\n' + source,
        encoding="utf-8",
    )


if __name__ == "__main__":
    apply(Path(sys.argv[1]))
