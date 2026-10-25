import { constants } from 'node:fs';
import { lstat, readdir, open, realpath } from 'node:fs/promises';
import { isAbsolute, join, parse, win32 } from 'node:path';

function configError() {
  throw new Error('artifact oracle configuration error');
}

function validPath(path) {
  return typeof path === 'string' && path.length > 0 && !isAbsolute(path)
    && !win32.isAbsolute(path) && !path.includes('\\') && !path.includes('\0')
    && path.split('/').every(part => part && part !== '.' && part !== '..');
}

/** Evaluator-only registry. Definitions contain trusted bytes, never model paths
 * or executable verifiers. Keep answer material outside the attempt workspace.
 * Run after all workspace writers exit: this is not a concurrent-process sandbox.
 */
export function createArtifactOracle(definitions) {
  if (!(definitions instanceof Map) || definitions.size === 0) configError();
  const registry = new Map();
  for (const [taskId, definition] of definitions) {
    if (typeof taskId !== 'string' || !taskId || !definition
        || typeof definition.allowExtraFiles !== 'boolean') configError();
    const files = new Map();
    for (const field of ['expectedFiles', 'unchangedFiles']) {
      const entries = definition[field];
      if (!entries || typeof entries !== 'object' || Array.isArray(entries)) configError();
      for (const [path, bytes] of Object.entries(entries)) {
        if (!validPath(path) || !Buffer.isBuffer(bytes) || files.has(path)) configError();
        files.set(path, { bytes: Buffer.from(bytes), kind: field });
      }
    }
    if (!Object.keys(definition.expectedFiles).length) configError();
    const directories = new Set();
    for (const path of files.keys()) {
      const parts = path.split('/');
      parts.pop();
      while (parts.length) {
        const directory = parts.join('/');
        if (files.has(directory)) configError();
        directories.add(directory);
        parts.pop();
      }
    }
    registry.set(taskId, { files, directories, allowExtraFiles: definition.allowExtraFiles });
  }
  return async function verifyArtifact({ taskId, workRoot }) {
    const definition = registry.get(taskId);
    if (!definition) configError();
    const checks = [];
    let failures = 0;
    let total = 0;
    function check(kind, pass) {
      total++;
      if (!pass) failures++;
      // No file contents, filesystem error messages or untrusted names in diagnostics.
      if (checks.length < 100) checks.push({ kind, pass });
    }
    try {
      if (typeof workRoot !== 'string' || !isAbsolute(workRoot)) throw new Error('root');
      // Reject symlinks in every component before enumerating or reading the tree.
      let current = parse(workRoot).root;
      for (const part of workRoot.slice(current.length).split('/').filter(Boolean)) {
        if (part === '.' || part === '..') throw new Error('root');
        current = join(current, part);
        const stat = await lstat(current);
        if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error('root');
      }
      const root = await realpath(workRoot);
      const found = new Set();
      async function visit(directory, prefix = '') {
        for (const name of await readdir(directory)) {
          const relative = prefix ? `${prefix}/${name}` : name;
          const path = join(directory, name);
          const stat = await lstat(path);
          if (stat.isSymbolicLink() || (!stat.isFile() && !stat.isDirectory())) {
            check('unsafe_entry', false);
          } else if (stat.isDirectory()) {
            check('directory', definition.allowExtraFiles || definition.directories.has(relative));
            await visit(path, relative);
          } else {
            found.add(relative);
            const expected = definition.files.get(relative);
            if (!expected) {
              check('extra_file', definition.allowExtraFiles);
              continue;
            }
            const handle = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
            try {
              const opened = await handle.stat();
              if (!opened.isFile() || opened.dev !== stat.dev || opened.ino !== stat.ino) throw new Error('changed');
              // Compare in bounded chunks; never retain actual file contents in results.
              let equal = opened.size === expected.bytes.length;
              const chunk = Buffer.alloc(65536);
              for (let offset = 0; equal && offset < opened.size;) {
                const { bytesRead } = await handle.read(chunk, 0, Math.min(chunk.length, opened.size - offset), offset);
                if (!bytesRead || !chunk.subarray(0, bytesRead).equals(expected.bytes.subarray(offset, offset + bytesRead))) equal = false;
                offset += bytesRead;
              }
              check(expected.kind, equal);
            } finally {
              await handle.close();
            }
          }
        }
      }
      await visit(root);
      for (const path of definition.files.keys()) if (!found.has(path)) check('missing_file', false);
    } catch {
      check('filesystem_boundary_or_read', false);
    }
    return { pass: failures === 0, checks, reason: `${failures}/${total} checks failed` };
  };
}
