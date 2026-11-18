import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { isAbsolute } from "node:path";
import { fileURLToPath } from "node:url";
import { tool } from "@langchain/core/tools";
import { ToolMessage } from "@langchain/core/messages";

const entry = fileURLToPath(new URL("./shiju-mcp.py", import.meta.url));
const namePattern = /^[A-Za-z0-9]+(?:_[A-Za-z0-9]+)*$/;

export function mcpSelection(servers = []) {
  if (!servers.length) return [];
  if (process.platform !== "darwin" || servers.length > 16) {
    throw new Error("DeepAgents MCP requires the reviewed macOS stdio sandbox");
  }
  const names = new Set();
  return servers.map(({ type, name, command, args, env, _meta }) => {
    if ((type && type !== "stdio") || !namePattern.test(name ?? "") ||
        names.has(name) || typeof command !== "string" || !isAbsolute(command) ||
        !Array.isArray(args) || args.some(arg => typeof arg !== "string") ||
        !Array.isArray(env) || env.length) {
      throw new Error("DeepAgents MCP requires uniquely named absolute stdio servers without environment overrides");
    }
    names.add(name);
    const { allow = [], deny = [] } = _meta?.["shiju.deepagents.toolPolicy"] ?? {};
    if (![allow, deny].every(list => Array.isArray(list) &&
        list.every(value => typeof value === "string"))) {
      throw new Error("Invalid DeepAgents MCP tool policy");
    }
    return { name, command, args, allow, deny };
  });
}

function run(request, signal) {
  signal?.throwIfAborted();
  return new Promise((resolve, reject) => {
    const child = execFile("/usr/bin/python3", ["-I", entry], {
      env: { HOME: process.env.HOME, PATH: "/usr/bin:/bin" },
      maxBuffer: 4 * 1024 * 1024, timeout: 20000,
    }, (error, stdout) => {
      signal?.removeEventListener("abort", abort);
      if (error || signal?.aborted) {
        reject(new Error("Selected MCP operation failed or was cancelled"));
        return;
      }
      try {
        resolve(JSON.parse(stdout));
      } catch {
        reject(new Error("Invalid selected MCP response"));
      }
    });
    function abort() {
      child.stdin.end();
    }
    child.stdin.on("error", () => child.kill());
    signal?.addEventListener("abort", abort, { once: true });
    child.stdin.write(JSON.stringify(request) + "\n");
  });
}

function allowedTool(server, name, qualified) {
  let id = "mcp__" + qualified.toLowerCase();
  if (qualified !== qualified.toLowerCase() || id.length > 64) {
    const hash = createHash("sha256").update(server.name + "\0" + name).digest("hex").slice(0, 16);
    id = id.slice(0, 47) + "_" + hash;
  }
  const matches = list => list.includes(name) || list.includes(id);
  return !matches(server.deny) && (!server.allow.length || matches(server.allow));
}

export async function discoverMcp(selection, workspace) {
  const tools = new Map();
  for (const policy of selection) {
    const { name, command, args } = policy;
    const server = { name, command, args };
    const descriptors = await run({ operation: "list", server, workspace });
    if (!Array.isArray(descriptors) || !descriptors.length) {
      throw new Error("Selected MCP server did not expose tools");
    }
    for (const { name, description, inputSchema } of descriptors) {
      if (!namePattern.test(name ?? "")) throw new Error("Invalid selected MCP tool name");
      const nameForModel = server.name + "__" + name;
      if (!allowedTool(policy, name, nameForModel)) continue;
      if (nameForModel.length > 64 ||
          tools.size >= 256 || tools.has(nameForModel) || inputSchema?.type !== "object") {
        throw new Error("Invalid or duplicate selected MCP tool descriptor");
      }
      tools.set(nameForModel, tool(async (args, config) => {
        const result = await run({ operation: "call", server, workspace, name, arguments: args }, config.signal);
        return new ToolMessage({
          content: JSON.stringify(result),
          status: result.isError ? "error" : "success",
          tool_call_id: config.toolCall.id,
        });
      }, {
        name: nameForModel,
        description: `MCP server ${server.name}. ${description ?? name}`,
        schema: inputSchema,
      }));
    }
  }
  return tools;
}
