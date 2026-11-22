// Only the host projection may execute selected tools on Linux.
const namePattern = /^[A-Za-z0-9]+(?:_[A-Za-z0-9]+)*$/;
const maxBody = 4 * 1024 * 1024;

export function linuxSelection(servers) {
  const base = new URL(process.env.OPENAI_BASE_URL);
  if (!process.env.VW_ACP_LAUNCH_REFERENCE || base.protocol !== "http:" ||
      base.hostname !== "127.0.0.1" || !base.port || base.pathname !== "/v1" ||
      base.username || base.password || base.search || base.hash || servers.length > 16) {
    throw new Error("DeepAgents requires a managed loopback MCP projection");
  }
  const names = new Set();
  return servers.map(({type, name, url, headers}) => {
    const header = headers?.[0];
    if (type !== "http" || typeof name !== "string" || !namePattern.test(name) ||
        names.has(name) || url !== `${base.origin}/shiju-vtcode-mcp/${name}` ||
        !Array.isArray(headers) || headers.length !== 1 || header?.name !== "Authorization" ||
        typeof header.value !== "string" || !/^Bearer [0-9a-f]{64}$/.test(header.value) ||
        header.value === `Bearer ${process.env.OPENAI_API_KEY}`) {
      throw new Error("Invalid DeepAgents host MCP selection");
    }
    names.add(name);
    return {name, url, authorization: header.value};
  });
}

export async function runProjectedMcp(request, signal) {
  const {server, operation, name, arguments: args, callId} = request;
  const list = operation === "list";
  if (!list && (operation !== "call" || typeof callId !== "string" || !callId ||
      !args || typeof args !== "object" || Array.isArray(args))) {
    throw new Error("Invalid DeepAgents MCP call identity");
  }
  const params = list ? {} : {
    name, arguments: args, _meta: {"shiju.deepagents.toolCallId": callId},
  };
  const body = JSON.stringify({
    jsonrpc: "2.0", id: 1, method: list ? "tools/list" : "tools/call", params,
  });
  if (Buffer.byteLength(body) > maxBody) throw new Error("DeepAgents MCP request too large");
  const timeout = AbortSignal.timeout(20000);
  const response = await fetch(server.url, {
    method: "POST", redirect: "error",
    headers: {"Authorization": server.authorization, "Content-Type": "application/json"},
    body, signal: signal ? AbortSignal.any([signal, timeout]) : timeout,
  });
  if (!response.ok || !response.body) {
    await response.body?.cancel();
    throw new Error("DeepAgents MCP projection rejected the request");
  }
  const chunks = [];
  let bytes = 0;
  for await (const chunk of response.body) {
    bytes += chunk.byteLength;
    if (bytes > maxBody) throw new Error("DeepAgents MCP response too large");
    chunks.push(chunk);
  }
  const reply = JSON.parse(Buffer.concat(chunks, bytes).toString("utf8"));
  if (reply?.jsonrpc !== "2.0" || reply.id !== 1 || reply.error ||
      !reply.result || typeof reply.result !== "object" || Array.isArray(reply.result)) {
    throw new Error("Invalid DeepAgents MCP projection response");
  }
  return list ? reply.result.tools : reply.result;
}
