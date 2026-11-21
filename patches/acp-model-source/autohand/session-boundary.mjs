// Imported only by the pinned ACP entry. The host owns and revokes this launch directory.
import { lstat, readdir, realpath } from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";

export async function attachSkillView(root, agent, files, createReader) {
  if (!root) return;
  const directory = await realpath(root);
  const metadata = await lstat(root);
  if (!metadata.isDirectory() || (metadata.mode & 0o077) !== 0) {
    throw new Error("AutoHand session skill view is not private");
  }

  const registry = agent.getSkillsRegistry();
  const nativeNames = new Set(registry.listSkills().map((skill) => skill.name.toLowerCase()));
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (!entry.isDirectory()) throw new Error("Invalid AutoHand skill view entry");
    const definition = path.join(directory, entry.name, "SKILL.md");
    await checkedFile(definition);
    const parsed = await registry.parser.parseFile(definition, "autohand-user");
    const skill = parsed.skill;
    if (!parsed.success || !skill || skill.name !== entry.name || nativeNames.has(skill.name.toLowerCase())) {
      throw new Error("Invalid or ambiguous AutoHand session skill name");
    }
    nativeNames.add(skill.name.toLowerCase());
    registry.registerSkill(skill, directory);
  }

  // Only the native bounded reader uses this manager. Editing, shell, and other
  // filesystem operations keep the original workspace manager and its roots.
  const reader = createReader(directory);
  const inspectWorkspace = files.inspectReadFile.bind(files);
  const readWorkspace = files.readFileWindow.bind(files);
  const inView = (target) => typeof target === "string" &&
    path.isAbsolute(target) && path.resolve(target).startsWith(directory + path.sep);
  files.inspectReadFile = async (target) => {
    if (!inView(target)) return inspectWorkspace(target);
    await checkedFile(path.resolve(target));
    return reader.inspectReadFile(target);
  };
  files.readFileWindow = async (target, options, inspection) => {
    if (!inView(target)) return readWorkspace(target, options, inspection);
    await checkedFile(path.resolve(target));
    return reader.readFileWindow(target, options);
  };

  async function checkedFile(file) {
    if (await realpath(file) !== file) throw new Error("AutoHand session skill symlink rejected");
    const stat = await lstat(file);
    if (!stat.isFile() || stat.nlink !== 1) throw new Error("AutoHand session skill resource is not a regular file");
  }
}

export function requireMcpApproval(agent, connection, sessionId) {
  const manager = agent.getMcpManager();
  const call = manager.callTool.bind(manager);
  const selected = new Map();
  const validConnection = (server) => {
    const expected = selected.get(server);
    const config = manager.servers.get(server)?.config;
    return expected && config?.transport === "http" && config.url === expected.url &&
      config.headers?.Authorization === expected.authorization;
  };
  manager.callTool = async (server, tool, args, options) => {
    if (!validConnection(server)) throw new Error("AutoHand MCP server is outside the host selection");
    const input = { ...args };
    // The native action envelope injects its internal action discriminator.
    delete input.type;
    const title = `mcp__${server}__${tool}`;
    const result = await connection.requestPermission({
      sessionId,
      toolCall: { toolCallId: randomUUID(), title, kind: "other", status: "pending", rawInput: input },
      options: [
        { optionId: "allow_once", name: "Allow once", kind: "allow_once" },
        { optionId: "reject_once", name: "Deny", kind: "reject_once" }
      ]
    });
    options?.signal?.throwIfAborted();
    if (result.outcome.outcome !== "selected" || result.outcome.optionId !== "allow_once") {
      throw new Error("Tool execution skipped by user.");
    }
    if (!validConnection(server)) throw new Error("AutoHand MCP connection changed during approval");
    return call(server, tool, input, options);
  };
  return (servers) => {
    for (const server of servers ?? []) {
      const url = new URL(server.url);
      const authorization = server.headers?.find(({ name }) => name === "Authorization")?.value;
      if (server.type !== "http" || url.protocol !== "http:" || url.hostname !== "127.0.0.1" ||
          !url.port || url.username || url.password || url.search || url.hash ||
          !authorization?.startsWith("Bearer ") || url.pathname !== `/shiju-vtcode-mcp/${server.name}` || !/^[A-Za-z0-9_-]+$/.test(server.name)) {
        throw new Error("AutoHand requires the selected host MCP relay");
      }
      selected.set(server.name, { url: server.url, authorization });
    }
  };
}
