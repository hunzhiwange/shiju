import assert from "node:assert/strict";
import test from "node:test";
import { guardMcpConnections } from "./linux-mcp-boundary.mjs";
import { requireMcpApproval } from "./session-boundary.mjs";

test("connects only the approved projection and rejects selection replacement", async () => {
  const url = "http://127.0.0.1:12345/shiju-vtcode-mcp/project";
  const authorization = "Bearer fixture";
  const config = { name: "project", transport: "http", url, headers: { Authorization: authorization } };
  const projected = {
    name: "project", type: "http", url, headers: [{ name: "Authorization", value: authorization }]
  };
  const attempts = [];
  const manager = {
    async connect(candidate) { attempts.push(candidate); },
    async callTool() { throw new Error("unexpected tool invocation"); }
  };
  const select = guardMcpConnections(manager);
  const approve = requireMcpApproval({ getMcpManager: () => manager }, {}, "session");
  const denied = /outside the host selection/;

  await assert.rejects(manager.connect({ name: "project", transport: "stdio" }), denied);
  await assert.rejects(manager.connect(config), denied);
  assert.throws(() => select([projected, projected], approve), /Invalid AutoHand host MCP selection/);
  assert.throws(() => select([{ ...projected, url: "https://example.invalid" }], approve), /selected host MCP relay/);
  await assert.rejects(manager.connect(config), denied);
  assert.deepEqual(attempts, []);

  select([projected], approve);
  await assert.rejects(manager.connect({ ...config, transport: "stdio" }), denied);
  await assert.rejects(manager.connect({ ...config, headers: {} }), denied);
  assert.throws(() => select([], approve), /requires a new managed session/);
  await manager.connect(config);
  assert.deepEqual(attempts, [config]);
});
