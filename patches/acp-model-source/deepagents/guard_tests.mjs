import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";
import { readFileSync } from "node:fs";
const root = process.argv[2];
const { hostToolGuard } = await import(pathToFileURL(root + "/dist/shiju-guard.mjs"));
const source = readFileSync(root + "/dist/cli.js", "utf8");
const start = source.indexOf("async requestToolPermission(session, conn, toolCall) {");
const end = source.indexOf("\n\tasync handleSlashCommand", start);
globalThis.formatToolCallTitle = () => "fixture";
globalThis.getToolCallKind = () => "edit";
const permission = Function("return ({" + source.slice(start, end).trim() + "})")().requestToolPermission;
for (const outcome of ["reject-once", "unknown", "cancelled", "rpc-error", "allow-once"]) {
  let executions = 0;
  const session = {agentName: "deepagents", threadId: "fixture", id: "fixture"};
  const controller = new AbortController();
  const server = {
    sessions: new Map([["fixture", session]]),
    currentPromptAbortController: controller,
    log() {},
    connection: {async requestPermission() {
      if (outcome === "rpc-error") throw new Error("fixture");
      return {outcome: outcome === "cancelled" ? {outcome: "cancelled"} :
        {outcome: "selected", optionId: outcome}};
    }},
    requestToolPermission: permission
  };
  const middleware = hostToolGuard(server, "deepagents");
  const request = {toolCall: {id: "tool", name: "write_file", args: {}},
    runtime: {configurable: {thread_id: "fixture"}}};
  await middleware.wrapToolCall(request, async () => { executions++; return "executed"; });
  assert.equal(executions, outcome === "allow-once" ? 1 : 0, outcome);
  controller.abort();
  await assert.rejects(() => middleware.wrapToolCall(request, async () => executions++));
}
console.log("DeepAgents guard: 5 permission outcomes and 5 cancellation checks passed");
