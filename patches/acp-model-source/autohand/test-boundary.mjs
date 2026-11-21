import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { attachSkillView, requireMcpApproval } from "./session-boundary.mjs";

const bundle = pathToFileURL(path.resolve(process.argv[2]) + path.sep);
const { FileActionManager } = await import(new URL("chunk-QKO3OXCQ.js", bundle));
const { SkillsRegistry } = await import(new URL("chunk-7AK6VXNJ.js", bundle));
const temp = await realpath(await mkdtemp(path.join(tmpdir(), "autohand-boundary-")));
try {
  const workspace = path.join(temp, "workspace");
  const selected = path.join(temp, "selected");
  const external = path.join(temp, "outside");
  const definition = path.join(selected, "fixture", "SKILL.md");
  const source = "---\nname: fixture\ndescription: test\n---\nSelected fixture.";
  await mkdir(workspace);
  await mkdir(selected, { mode: 0o700 });
  await mkdir(path.dirname(definition));
  await writeFile(definition, source);
  await writeFile(external, "out of scope");
  const registry = new SkillsRegistry(path.join(temp, "native"));
  const files = new FileActionManager(workspace);
  await attachSkillView(selected, { getSkillsRegistry: () => registry }, files,
    (root) => new FileActionManager(root));
  assert.equal(registry.getSkill("fixture").path, definition);
  assert.equal((await files.inspectReadFile(definition)).resolvedPath, definition);
  await assert.rejects(files.inspectReadFile(external), /escapes the allowed directories/);
  await assert.rejects(files.writeFile(definition, "unwanted"), /escapes the allowed directories/);
  assert.equal(await readFile(definition, "utf8"), source);
  const escape = path.join(selected, "fixture", "escape");
  await symlink(external, escape);
  await assert.rejects(files.inspectReadFile(escape), /symlink rejected/);

  const url = "http://127.0.0.1:12345/shiju-vtcode-mcp/test";
  const headers = { Authorization: "Bearer fixture" };
  const server = { config: { transport: "http", url, headers } };
  let invoked = 0;
  let outcome = "reject_once";
  const manager = {
    servers: new Map([["test", server]]),
    async callTool() { invoked += 1; }
  };
  const select = requireMcpApproval({ getMcpManager: () => manager }, {
    async requestPermission(request) {
      assert.equal(request.sessionId, "test-session");
      assert.equal(request.toolCall.title, "mcp__test__read");
      assert.deepEqual(request.toolCall.rawInput, { filename: "example" });
      return { outcome: { outcome: "selected", optionId: outcome } };
    }
  }, "test-session");
  select([{ name: "test", type: "http", url, headers: [
    { name: "Authorization", value: headers.Authorization }
  ] }]);
  const args = { type: "mcp__test__read", filename: "example" };
  await assert.rejects(manager.callTool("test", "read", args), /skipped by user/);
  assert.equal(invoked, 0);
  outcome = "allow_once";
  await manager.callTool("test", "read", args);
  assert.equal(invoked, 1);
  server.config = { ...server.config, url: url.replace("12345", "12346") };
  await assert.rejects(manager.callTool("test", "read", args), /outside the host selection/);
  assert.equal(invoked, 1);
  console.log("AutoHand scoped reads, unchanged writes, and MCP approvals passed");
} finally {
  await rm(temp, { recursive: true, force: true });
}
