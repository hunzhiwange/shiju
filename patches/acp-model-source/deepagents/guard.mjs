import { createMiddleware } from "langchain";
import { ToolMessage } from "@langchain/core/messages";

// Run inside the tool invocation, before the handler can perform any side effect.
// Stream notifications are informational and cannot be an authorization boundary.
export function hostToolGuard(server, agentName) {
  return createMiddleware({
    name: "ShiJuHostPermission",
    async wrapToolCall(request, handler) {
      const session = [...server.sessions.values()].find(
        s => s.agentName === agentName &&
          s.threadId === request.runtime.configurable?.thread_id
      );
      const prompt = server.currentPromptAbortController;
      if (!session || !server.connection || prompt?.signal.aborted) {
        throw new Error("No active authorized ACP session");
      }
      // Inherited edit/search/shell backends access local files directly. Only
      // reviewed ACP read/write operations may reach a supplier handler.
      const selectedMcp = session.mcpTools?.has(request.toolCall.name);
      if (selectedMcp && !prompt) throw new Error("No active authorized ACP prompt");
      if (!selectedMcp && !["read_file", "write_file"].includes(request.toolCall.name)) {
        return new ToolMessage({content: "Tool unavailable in reviewed DeepAgents runtime",
          tool_call_id: request.toolCall.id});
      }
      const decision = await server.requestToolPermission(
        session, server.connection, request.toolCall);
      if (decision !== "allow" || prompt?.signal.aborted ||
          server.currentPromptAbortController !== prompt) {
        return new ToolMessage({content: "Tool execution denied", ...(selectedMcp && {status: "error"}),
          tool_call_id: request.toolCall.id});
      }
      try {
        return await handler(request);
      } catch (error) {
        if (prompt?.signal.aborted) throw error;
        // A missing file or denied host RPC is a tool failure, never permission
        // to fall back locally. Do not expose private host error details.
        return new ToolMessage({
          content: selectedMcp ? "Selected MCP operation failed" : "Host file operation failed or was denied. read_file requires a regular file path, not a directory. No local fallback was attempted.",
          status: "error", tool_call_id: request.toolCall.id
        });
      }
    }
  });
}
