import { lstatSync, realpathSync, writeFileSync } from 'node:fs';
import { isAbsolute, join } from 'node:path';

const sessionId = /^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/;

function mountRecord(id) {
  const root = process.env.DIRAC_DATA_DIR;
  if (!process.env.VW_ACP_LAUNCH_REFERENCE || !root || !isAbsolute(root) || !sessionId.test(id)) {
    throw new Error('Dirac mounts require a managed session with private state');
  }
  const metadata = lstatSync(root);
  if (!metadata.isDirectory() || metadata.uid !== process.geteuid() ||
      (metadata.mode & 0o777) !== 0o700 || realpathSync(root) !== root) {
    throw new Error('Dirac mounts require a managed session with private state');
  }
  return join(root, 'shiju-mounted-' + id);
}

export function recordMount(id) {
  // Save only the fact that a mount existed. The tool and skill state never survives a restart.
  writeFileSync(mountRecord(id), '', { flag: 'wx', mode: 0o600 });
}

export function rejectMountedRestore(id) {
  if (!process.env.VW_ACP_LAUNCH_REFERENCE) return;
  const path = mountRecord(id);
  try {
    lstatSync(path);
  } catch (error) {
    if (error.code === 'ENOENT') return;
    throw error;
  }
  throw new Error('Dirac mounts support only new sessions');
}
