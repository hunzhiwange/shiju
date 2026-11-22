import assert from "node:assert/strict";
import test from "node:test";
import { linuxSelection, runProjectedMcp } from "./linux-mcp.mjs";

const endpoint = "http://127.0.0.1:43210";
const server = {
  type: "http",
  name: "fixture",
  url: endpoint + "/shiju-vtcode-mcp/fixture",
  headers: [{name: "Authorization", value: "Bearer " + "a".repeat(64)}],
};
process.env.VW_ACP_LAUNCH_REFERENCE = "fixture";
process.env.OPENAI_BASE_URL = endpoint + "/v1";
process.env.OPENAI_API_KEY = "b".repeat(64);

test("DeepAgents accepts only the host projection", () => {
  assert.equal(linuxSelection([server])[0].url, server.url);
  for (const invalid of [
    {...server, type: "stdio"},
    {...server, url: server.url.replace("43210", "43211")},
    {...server, url: server.url + "?override=1"},
    {...server, url: server.url.replace("/fixture", "/another")},
    {...server, headers: [{name: "Authorization", value: "Bearer " + process.env.OPENAI_API_KEY}]},
    {...server, headers: [...server.headers, {name: "Cookie", value: "unexpected"}]},
  ]) {
    assert.throws(() => linuxSelection([invalid]));
  }
  assert.throws(() => linuxSelection([server, server]));
});

test("DeepAgents transports the original ID and fails closed on rejection", async (t) => {
  const [selected] = linuxSelection([server]);
  const fetch = t.mock.method(globalThis, "fetch", async (_, options) => {
    const message = JSON.parse(options.body);
    if (message.method === "tools/list") {
      return Response.json({jsonrpc: "2.0", id: 1, result: {tools: []}});
    }
    return new Response(null, {status: 403});
  });
  assert.deepEqual(await runProjectedMcp({server: selected, operation: "list"}), []);
  const call = {server: selected, operation: "call", name: "lookup", arguments: {query: "fixture"}};
  await assert.rejects(runProjectedMcp(call), /call identity/);
  assert.equal(fetch.mock.callCount(), 1);
  await assert.rejects(runProjectedMcp({...call, callId: "original-id"}), /projection rejected/);
  assert.equal(fetch.mock.callCount(), 2);
  const [url, options] = fetch.mock.calls[1].arguments;
  assert.equal(url, server.url);
  assert.equal(options.redirect, "error");
  assert.equal(options.headers.Authorization, server.headers[0].value);
  assert.deepEqual(JSON.parse(options.body).params, {
    name: "lookup", arguments: {query: "fixture"},
    _meta: {"shiju.deepagents.toolCallId": "original-id"},
  });
});
