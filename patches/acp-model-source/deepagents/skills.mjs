import fs from "node:fs/promises";
import path from "node:path";
import { createSkillsMiddleware, MAX_SKILL_FILE_SIZE } from "deepagents";

export function skillMounts(meta) {
  const selection = meta?.["shiju.skillMounts"];
  if (selection === undefined) return [];
  if (selection?.version !== 1 || !Array.isArray(selection.skills)) {
    throw new Error("Invalid DeepAgents skill selection");
  }
  const roots = selection.skills.map(({ name, path: root }) => {
    if (typeof name !== "string" || typeof root !== "string" ||
        !path.isAbsolute(root) || path.normalize(root) !== root ||
        path.basename(root) !== name) {
      throw new Error("Invalid DeepAgents skill directory");
    }
    return root;
  });
  if (new Set(roots).size !== roots.length) throw new Error("Duplicate DeepAgents skill directory");
  return roots;
}

export async function nativeSkills(workspace, nativeRoots, mountedRoots, readText) {
  const roots = new Map();
  for (const root of mountedRoots) {
    if (!(await fs.lstat(root)).isSymbolicLink()) {
      throw new Error("DeepAgents requires a mounted session skill directory");
    }
    roots.set(root, await fs.realpath(root));
  }
  // DeepAgents resolves equal names in favor of the last source. Preserve its
  // native ordering and give an existing native skill precedence over a mount.
  for (const root of nativeRoots) {
    if (!root.startsWith(workspace + path.sep)) {
      throw new Error("DeepAgents native skills must be inside the workspace");
    }
    try {
      if (await fs.realpath(root) === root && (await fs.lstat(root)).isDirectory()) {
        roots.set(root, root);
      }
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
  }
  if (!roots.size) return [];

  const files = new Map();
  const backend = {
    async ls(source) {
      const root = path.normalize(source).replace(/\/$/, "");
      const target = roots.get(root);
      if (!target || await fs.realpath(root) !== target) return { files: [] };
      const entries = await fs.readdir(target, { withFileTypes: true });
      const listed = [];
      for (const entry of entries) {
        if (entry.isDirectory() && target === root) {
          const candidate = path.join(root, entry.name, "SKILL.md");
          files.set(candidate, candidate);
          listed.push({ path: path.join(root, entry.name) + "/", is_dir: true });
        } else if (entry.isFile() && entry.name === "SKILL.md") {
          const candidate = path.join(root, entry.name);
          files.set(candidate, path.join(target, entry.name));
          listed.push({ path: candidate, is_dir: false });
        }
      }
      return { files: listed };
    },
    async downloadFiles(paths) {
      return Promise.all(paths.map(async file => {
        const target = files.get(file);
        try {
          if (target && await fs.realpath(file) === target) {
            const stat = await fs.lstat(target);
            if (stat.isFile() && stat.size <= MAX_SKILL_FILE_SIZE) {
              const content = await readText(file);
              if (Buffer.byteLength(content) <= MAX_SKILL_FILE_SIZE) {
                return { path: file, content: Buffer.from(content), error: null };
              }
            }
          }
        } catch {
          // Missing files and rejected host reads do not enter the catalog.
        }
        return { path: file, content: null, error: "file_not_found" };
      }));
    },
  };
  return [createSkillsMiddleware({ backend, sources: [...roots.keys()] })];
}
