// A managed Linux agent can only connect to the host's session projection.
export function guardMcpConnections(manager) {
  const connect = manager.connect.bind(manager);
  let selected;
  manager.connect = async (config) => {
    const expected = selected?.get(config?.name);
    if (!expected || config.transport !== "http" || config.url !== expected.url ||
        Object.keys(config.headers ?? {}).length !== 1 ||
        config.headers?.Authorization !== expected.authorization) {
      throw new Error("AutoHand MCP connection is outside the host selection");
    }
    return connect(config);
  };
  return (servers, approveSelection) => {
    if (selected) throw new Error("AutoHand MCP selection requires a new managed session");
    const next = new Map();
    for (const server of servers ?? []) {
      const [header] = server.headers ?? [];
      if (server.type !== "http" || server.headers?.length !== 1 ||
          header?.name !== "Authorization" || next.has(server.name)) {
        throw new Error("Invalid AutoHand host MCP selection");
      }
      next.set(server.name, { url: server.url, authorization: header.value });
    }
    // Reuse the existing relay URL and bearer validation before publishing.
    approveSelection(servers);
    selected = next;
  };
}
