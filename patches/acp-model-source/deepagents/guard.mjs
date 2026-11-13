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
      if (!session || !server.connection ||
          server.currentPromptAbortController?.signal.aborted) {
        throw new Error("No active authorized ACP session");
      }
      // Nested agents have independent tool execution and are not authorized here.
      if (request.toolCall.name === "task") {
        return new ToolMessage({content: "Subagents are unavailable",
          tool_call_id: request.toolCall.id});
      }
      const decision = await server.requestToolPermission(
        session, server.connection, request.toolCall);
      if (decision !== "allow" ||
          server.currentPromptAbortController?.signal.aborted) {
        return new ToolMessage({content: "Tool execution denied",
          tool_call_id: request.toolCall.id});
      }
      return handler(request);
    }
  });
}
