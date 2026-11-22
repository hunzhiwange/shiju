import assert from "node:assert/strict";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { test } from "node:test";

const packageRoot = process.env.GEMINI_REVIEWED_PACKAGE;
assert.ok(packageRoot, "set GEMINI_REVIEWED_PACKAGE to the Linux v4 npm package directory");
process.argv.push("--shiju-model-source");
const {
  setMountedServers, validateMountedServers, describeMcpTool,
  authorizeMcpInvocation, consumeMcpApproval
} = await import(pathToFileURL(resolve(packageRoot, "bundle/shiju-linux-mcp.mjs")).href);

function invocation(config, serverName) {
  const tool = {serverName, serverToolName: "read_probe"};
  const call = {...tool, params: {key: "fixture"}};
  return [call, describeMcpTool(config, call, tool)];
}

test("classifies native and mounted tools and rejects name collisions", () => {
  const config = {};
  const mounts = [{name: "session_probe"}];
  setMountedServers(config, mounts);
  assert.equal(invocation(config, "session_probe")[1].source.origin, "mounted");
  assert.equal(invocation(config, "native_probe")[1].source.origin, "native");
  assert.throws(() => validateMountedServers(mounts, {session_probe: {}}));
  assert.throws(() => validateMountedServers(mounts, {}, {session_probe: {}}));
  assert.throws(() => validateMountedServers([...mounts, ...mounts], {}));
  assert.throws(() => validateMountedServers([{name: "mcp"}], {}, {}, "configured-command"));
  assert.throws(() => describeMcpTool(config, {params: {}}, {
    serverName: "session_probe", serverToolName: "read_probe", extensionName: "extension"
  }));
});

test("consumes approval only for its current invocation, signal and arguments", () => {
  const config = {};
  setMountedServers(config, []);
  const [call, approval] = invocation(config, "native_probe");
  const controller = new AbortController();
  assert.throws(() => consumeMcpApproval(call, controller.signal));

  authorizeMcpInvocation(call, approval, controller.signal);
  assert.throws(() => consumeMcpApproval({...call}, controller.signal));
  assert.doesNotThrow(() => consumeMcpApproval(call, controller.signal));
  assert.throws(() => consumeMcpApproval(call, controller.signal));

  authorizeMcpInvocation(call, approval, controller.signal);
  call.params.key = "changed";
  assert.throws(() => consumeMcpApproval(call, controller.signal));

  call.params.key = "fixture";
  authorizeMcpInvocation(call, approval, controller.signal);
  assert.throws(() => consumeMcpApproval(call, new AbortController().signal));
  authorizeMcpInvocation(call, approval, controller.signal);
  controller.abort();
  assert.throws(() => consumeMcpApproval(call, controller.signal));
});
