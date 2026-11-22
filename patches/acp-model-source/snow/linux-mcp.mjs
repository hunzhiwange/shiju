// Consume a request-owned directory. Snow does not start MCP child processes here.
import { request } from "node:http";

const limit = 4 * 1024 * 1024;
const namePattern = /^[a-zA-Z0-9]+(?:_[a-zA-Z0-9]+)*$/;

export async function discoverProjectedSnowMcp(servers, config, signal) {
  if (!Array.isArray(servers) || servers.length > 16) {
    throw new Error("Invalid Snow MCP projection size");
  }
  const tools = new Map();
  if (!servers.length) return tools;
  const model = new URL(config.baseUrl);
  if (model.protocol !== "http:" || model.hostname !== "127.0.0.1" || !model.port ||
      !["/v1", "/v1beta"].includes(model.pathname) || model.username || model.password ||
      model.search || model.hash || !config.apiKey) {
    throw new Error("Snow requires a host-owned loopback MCP projection");
  }
  const names = new Set();
  for (const { type, name, url, headers } of servers) {
    const header = headers?.[0];
    if (type !== "http" || typeof name !== "string" || name.length > 64 ||
        !namePattern.test(name) || names.has(name.toLowerCase()) ||
        url !== `${model.origin}/shiju-vtcode-mcp/${name}` || headers?.length !== 1 ||
        header?.name !== "Authorization" || typeof header.value !== "string" ||
        !/^Bearer [a-f0-9]{64}$/.test(header.value) || header.value === `Bearer ${config.apiKey}`) {
      throw new Error("Invalid Snow host MCP selection");
    }
    names.add(name.toLowerCase());
    const send = (method, params, turnSignal) => sendProjected(url, header.value, method, params, turnSignal);
    const catalog = await send("tools/list", {}, signal);
    if (!Array.isArray(catalog.tools)) throw new Error("Invalid Snow MCP tool catalog");
    for (const tool of catalog.tools) {
      const alias = `mcp__${name}__${tool.name}`;
      if (typeof tool.name !== "string" || !namePattern.test(tool.name) || alias.length > 64 ||
          tools.has(alias) || !tool.inputSchema || typeof tool.inputSchema !== "object" ||
          Array.isArray(tool.inputSchema)) {
        throw new Error("Invalid or duplicate Snow MCP tool");
      }
      tools.set(alias, {
        serviceName: name,
        definition: {
          type: "function",
          function: { name: alias, description: tool.description, parameters: tool.inputSchema },
        },
        call(args, callId, turnSignal) {
          return send("tools/call", {
            name: tool.name, arguments: args, _meta: { "shiju.snow.toolCallId": callId },
          }, turnSignal);
        },
      });
    }
  }
  return tools;
}

async function sendProjected(url, authorization, method, params, signal) {
  signal.throwIfAborted();
  const data = JSON.stringify({ jsonrpc: "2.0", id: 1, method, params });
  if (Buffer.byteLength(data) > limit) throw new Error("Snow MCP request too large");
  const deadline = AbortSignal.any([signal, AbortSignal.timeout(20000)]);
  const response = await new Promise((resolve, reject) => {
    const call = request(url, {
      agent: false, method: "POST", signal: deadline,
      headers: {
        Authorization: authorization, "Content-Type": "application/json",
        "Content-Length": Buffer.byteLength(data),
      },
    }, resolve);
    call.once("error", reject);
    call.end(data);
  });
  if (response.statusCode !== 200) {
    response.destroy();
    throw new Error("Snow MCP projection rejected the request");
  }
  const chunks = [];
  let bytes = 0;
  for await (const chunk of response) {
    bytes += chunk.length;
    if (bytes > limit) throw new Error("Snow MCP response too large");
    chunks.push(chunk);
  }
  deadline.throwIfAborted();
  const reply = JSON.parse(Buffer.concat(chunks, bytes).toString("utf8"));
  if (reply?.jsonrpc !== "2.0" || reply.id !== 1 || reply.error ||
      !reply.result || typeof reply.result !== "object" || Array.isArray(reply.result)) {
    throw new Error("Invalid Snow MCP projection response");
  }
  return reply.result;
}
