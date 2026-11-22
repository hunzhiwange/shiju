import { strict as assert } from "node:assert";
import { modelOnlyOptions, checkPrompt, rejectMutation } from "./model-only.mjs";

const env = {
  VW_ACP_CLAUDE_NATIVE_MESSAGES: "1",
  VW_ACP_CLAUDE_SETTINGS: "/private/launch/settings.json",
  CLAUDE_CODE_EXECUTABLE: "/reviewed/sdk/cli.js",
  ANTHROPIC_MODEL: "test-model",
  ANTHROPIC_BASE_URL: "http://127.0.0.1:12345",
  ANTHROPIC_AUTH_TOKEN: "test-relay-token",
  BASH_ENV: "/host/startup",
};
const params = {
  cwd: "/host/project",
  mcpServers: [],
  _meta: { claudeCode: { options: { tools: ["Bash"], extraArgs: { chrome: null },
    model: "unbound", plugins: [{ path: "/host/plugin" }] } } },
};
const options = modelOnlyOptions(params, {}, env, "linux");
assert.deepEqual(options.tools, []);
assert.deepEqual(options.mcpServers, {});
assert.deepEqual(options.settingSources, []);
assert.equal(options.strictMcpConfig, true);
assert.equal(options.cwd, "/private/launch");
assert.equal(options.model, "test-model");
assert.equal(options.permissionMode, "default");
assert.equal(options.extraArgs.bare, null);
assert.equal(options.extraArgs["disable-slash-commands"], null);
assert.equal(options.extraArgs.chrome, undefined);
assert.equal(options.env.BASH_ENV, undefined);
assert.equal(options.plugins, undefined);
assert.equal((await options.canUseTool("Read", {})).behavior, "deny");
assert.throws(() => modelOnlyOptions({ ...params, mcpServers: [{ command: "/bad" }] }, {}, env, "linux"));
assert.throws(() => modelOnlyOptions(params, { resume: "old" }, env, "linux"));
assert.throws(() => modelOnlyOptions(params, {}, { ...env, VW_ACP_CLAUDE_NATIVE_MESSAGES: "0" }, "linux"));
assert.throws(() => modelOnlyOptions(params, {}, { ...env, ANTHROPIC_BASE_URL: "https://example.invalid" }, "linux"));
assert.doesNotThrow(() => checkPrompt({ prompt: [{ type: "text", text: "hello" }] }));
assert.throws(() => checkPrompt({ prompt: [{ type: "text", text: " /mcp" }] }));
assert.throws(() => checkPrompt({ prompt: [{ type: "resource" }] }));
assert.throws(() => rejectMutation({ modeId: "bypassPermissions" }));
