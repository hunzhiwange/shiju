import { createHash, randomUUID } from 'node:crypto';
import { constants } from 'node:fs';
import { lstat, realpath, open, mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, dirname, isAbsolute, win32, basename } from 'node:path';

const owned = new WeakMap();

function relativeFile(path) {
  if (typeof path !== 'string' || !path || isAbsolute(path) || win32.isAbsolute(path)
      || path.includes('\\') || path.includes('\0')
      || path.split('/').some(part => !part || part === '.' || part === '..')) {
    throw new Error('fixture files must be trusted relative file paths');
  }
}

async function identity(path) {
  const stat = await lstat(path);
  if (stat.isSymbolicLink()) throw new Error('fixture symlink is forbidden');
  return stat;
}

/** Copy only the caller's trusted public-input allowlist, never a whole fixture tree.
 * Exclude answers, verifiers and holdout material from that allowlist. This is
 * directory isolation, not a security sandbox; fixture sources must be trusted
 * and immutable during preparation.
 */
export async function prepare({ fixtureRoot, fixtureFiles }) {
  if (!isAbsolute(fixtureRoot)) throw new Error('fixtureRoot must be absolute');
  if (!Array.isArray(fixtureFiles)) throw new Error('fixtureFiles must be an allowlist');
  for (const file of fixtureFiles) relativeFile(file);
  if (new Set(fixtureFiles).size !== fixtureFiles.length) throw new Error('duplicate fixture file');
  await identity(fixtureRoot);
  const source = await realpath(fixtureRoot);
  const inputs = [];
  for (const file of [...fixtureFiles].sort()) {
    let path = source;
    for (const part of file.split('/')) {
      path = join(path, part);
      await identity(path);
    }
    const handle = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW);
    try {
      if (!(await handle.stat()).isFile()) throw new Error('fixture must be a regular file');
      inputs.push([file, await handle.readFile()]);
    } finally {
      await handle.close();
    }
  }
  const digest = createHash('sha256');
  for (const [file, bytes] of inputs) {
    digest.update(JSON.stringify([file, bytes.length]) + '\n').update(bytes);
  }
  const workRoot = await mkdtemp(join(await realpath(tmpdir()), 'vw-accuracy-'));
  try {
    for (const [file, bytes] of inputs) {
      await mkdir(dirname(join(workRoot, file)), { recursive: true });
      await writeFile(join(workRoot, file), bytes, { flag: 'wx' });
    }
    const stat = await lstat(workRoot);
    const processes = new Set();
    const workspace = Object.freeze({
      attemptId: basename(workRoot) + '-' + randomUUID(),
      workRoot,
      inputDigest: digest.digest('hex'),
      cleanupPolicy: Object.freeze({ success: 'after-process-exit', failure: 'after-process-exit', interruption: 'after-process-exit' }),
      // Every process using this directory must be tracked before cleanup.
      // Callers must also wait for any descendants before invoking cleanup.
      trackProcess(child) {
        if (owned.get(workspace)?.cleaned) throw new Error('workspace already cleaned');
        if (!Number.isInteger(child?.pid) || !('exitCode' in child) || !('signalCode' in child)) {
          throw new Error('trackProcess requires a spawned ChildProcess');
        }
        processes.add(child);
      },
    });
    owned.set(workspace, { workRoot, dev: stat.dev, ino: stat.ino, processes, cleaned: false });
    return workspace;
  } catch (error) {
    await rm(workRoot, { recursive: true, force: true });
    throw error;
  }
}

/** Explicit cleanup for success, failure and interruption, after all users exit.
 * Only exact handles created in this process authorize deletion; rm does not
 * follow symlinks inside the work tree. No automatic process-exit deletion.
 */
export async function cleanup(workspace) {
  const record = owned.get(workspace);
  if (!record) throw new Error('unknown workspace handle');
  if (record.cleaned) return;
  for (const child of record.processes) {
    if (child.exitCode === null && child.signalCode === null) throw new Error('workspace has an active process');
  }
  const stat = await lstat(record.workRoot);
  if (!stat.isDirectory() || stat.isSymbolicLink() || stat.dev !== record.dev || stat.ino !== record.ino) {
    throw new Error('workspace root was replaced');
  }
  await rm(record.workRoot, { recursive: true, force: false });
  record.cleaned = true;
}
