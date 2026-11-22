import { dirname, isAbsolute } from "node:path";

export function modelOnlyOptions(params, creation, env = process.env, platform = process.platform) {
  const settings = env.VW_ACP_CLAUDE_SETTINGS;
  const engine = env.CLAUDE_CODE_EXECUTABLE;
  const model = env.ANTHROPIC_MODEL;
  if (platform !== "linux" || env.VW_ACP_CLAUDE_NATIVE_MESSAGES !== "1"
      || !settings || !isAbsolute(settings) || !engine || !isAbsolute(engine)
      || !model || !env.ANTHROPIC_AUTH_TOKEN
      || !/^http:\/\/127\.0\.0\.1:[1-9][0-9]{0,4}$/.test(env.ANTHROPIC_BASE_URL ?? "")) {
    throw new Error("Claude model-only requires the managed Linux Messages launch");
  }
  if (creation.resume !== undefined || creation.forkSession
      || (params.mcpServers !== undefined
        && (!Array.isArray(params.mcpServers) || params.mcpServers.length !== 0))) {
    throw new Error("Claude model-only rejects history restoration and MCP mounts");
  }
  const cwd = dirname(settings);
  const sdkEnv = {
    HOME: cwd,
    TMPDIR: cwd,
    XDG_CONFIG_HOME: cwd,
    CLAUDE_CONFIG_DIR: cwd,
    CLAUDE_CODE_PROVIDER_MANAGED_BY_HOST: "1",
    CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
    DISABLE_AUTOUPDATER: "1",
    ANTHROPIC_BASE_URL: env.ANTHROPIC_BASE_URL,
    ANTHROPIC_AUTH_TOKEN: env.ANTHROPIC_AUTH_TOKEN,
    ANTHROPIC_MODEL: model,
  };
  for (const key of ["LANG", "LC_ALL", "MAX_THINKING_TOKENS", "CLAUDE_CODE_MAX_OUTPUT_TOKENS"]) {
    if (env[key] !== undefined) sdkEnv[key] = env[key];
  }
  return {
    cwd,
    model,
    settings,
    executable: process.execPath,
    pathToClaudeCodeExecutable: engine,
    env: sdkEnv,
    settingSources: [],
    tools: [],
    mcpServers: {},
    strictMcpConfig: true,
    permissionMode: "default",
    allowDangerouslySkipPermissions: false,
    canUseTool: async () => ({ behavior: "deny", message: "Claude model-only disables tools" }),
    persistSession: false,
    includePartialMessages: true,
    extraArgs: { bare: null, "disable-slash-commands": null, "replay-user-messages": null },
  };
}

export function checkPrompt(params) {
  if (!Array.isArray(params.prompt) || params.prompt.length === 0
      || params.prompt.some((part) => part?.type !== "text" || typeof part.text !== "string")
      || params.prompt.map((part) => part.text).join("").trimStart().startsWith("/")) {
    throw new Error("Claude model-only accepts plain text prompts");
  }
}

export function rejectMutation() {
  throw new Error("Claude model-only locks the model and permission mode for the session");
}
