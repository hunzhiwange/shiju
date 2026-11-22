import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { runInNewContext } from "node:vm";

const contract = JSON.parse(readFileSync(new URL("./linux.json", import.meta.url), "utf8"));
const relay = { type: "http", url: `http://127.0.0.1:9321/mcp/${"a".repeat(43)}`, headers: [] };

for (const [arch, target] of Object.entries(contract.targets)) {
  const [original, patched] = target.edits.find(([, after]) => after.includes("SHIJU_KILO_BRIDGE_PORT"));
  const body = original.indexOf("{") + 1;
  const guard = patched.slice(body, patched.indexOf(original.slice(body)));
  function check(servers, env) {
    // Evaluate just the inserted preflight, before the real client's MCP add.
    runInNewContext(guard, { s: servers, process: { env } });
  }

  test(`${arch} managed ACP MCP accepts the owned bridge only`, () => {
    const env = { SHIJU_KILO_RESTRICTED: "1", SHIJU_KILO_BRIDGE_PORT: "9321" };
    check([], env);
    check([relay], env);
    for (const server of [
      { type: "stdio", command: "unused" },
      { ...relay, type: "sse" },
      { ...relay, url: relay.url.replace("9321", "9322") },
      { ...relay, url: relay.url.replace("127.0.0.1", "localhost") },
      { ...relay, url: relay.url.replace("127.0.0.1", "guest@127.0.0.1") },
      { ...relay, url: relay.url + "?a=b" },
      { ...relay, url: relay.url + "\n" },
      { ...relay, headers: {} },
      { ...relay, headers: [{ name: "x-test", value: "unused" }] },
    ]) {
      assert.throws(() => check([relay, server], env), /Invalid managed MCP/);
    }
    assert.throws(() => check([], { SHIJU_KILO_RESTRICTED: "1" }), /Invalid managed MCP/);
    check([{ type: "stdio", command: "unused" }], {});
  });
}
