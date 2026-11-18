// Pi RPC editor requests carry results from the owning ACP session.
export default function register(pi) {
  const tools = JSON.parse(process.env.VW_PI_MCP_CATALOG);
  delete process.env.VW_PI_MCP_CATALOG;
  for (const { name, description, inputSchema } of tools) {
    pi.registerTool({
      name,
      label: name,
      description,
      parameters: inputSchema,
      async execute(toolCallId, args, signal, _update, ctx) {
        signal?.throwIfAborted();
        const raw = await ctx.ui.editor("ShiJu session MCP", JSON.stringify({ name, toolCallId, args }));
        signal?.throwIfAborted();
        if (raw === undefined) throw new Error("MCP operation denied or cancelled");
        const result = JSON.parse(raw);
        if (result.isError) throw new Error(JSON.stringify(result));
        const content = (result.content ?? []).filter(block => block.type === "text" || block.type === "image");
        if (result.structuredContent !== undefined || content.length !== (result.content ?? []).length) {
          content.push({ type: "text", text: JSON.stringify(result) });
        }
        return { content, details: result };
      },
    });
  }
  pi.on("session_start", () => {
    pi.setActiveTools([...new Set([...pi.getActiveTools(), ...tools.map(tool => tool.name)])]);
  });
  pi.registerCommand("shiju-session-mcp", {
    description: "Check session MCP registration",
    handler: async () => {},
  });
}
