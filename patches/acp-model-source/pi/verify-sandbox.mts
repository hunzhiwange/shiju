import { mkdtemp, mkdir, writeFile, rm, readFile } from 'node:fs/promises'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawnSync } from 'node:child_process'
import { pathToFileURL } from 'node:url'
const { restrictedLaunch } = await import(pathToFileURL(process.argv[2]).href)
const root = await mkdtemp(join(tmpdir(), 'pi-boundary-'))
const server = createServer(() => { throw new Error('unapproved loopback reached') })
const bind = () => new Promise<void>(ok => server.listen(0, '127.0.0.1', ok))
await bind()
try {
  const workspace = join(root,'project')
  const native = join(root,'native')
  const sessions = join(root,'sessions')
  for (const path of [workspace,native,sessions]) await mkdir(path)
  const foreign = join(root,'foreign.txt')
  const own = join(workspace,'own.txt')
  await writeFile(foreign,'foreign')
  await writeFile(own,'own')
  await writeFile(join(native,'settings.json'),JSON.stringify({sessionDir:sessions}))
  const address = server.address() as { port: number }
  await writeFile(join(native,'models.json'),JSON.stringify({providers:{'session-provider':{
    baseUrl:`http://127.0.0.1:${address.port === 48193 ? 48194 : 48193}/v1`}}}))
  process.env.PI_CODING_AGENT_DIR = native
  process.env.VW_PI_RELAY_TOKEN = 'fixture'
  const launch = await restrictedLaunch(workspace, process.argv[3], '', [])
  const source = `
    const fs = require('node:fs');
    const cp = require('node:child_process');
    const net = require('node:net');
    const assert = require('node:assert/strict');
    assert.equal(fs.readFileSync(${JSON.stringify(own)}, 'utf8'), 'own');
    assert.throws(() => fs.readFileSync(${JSON.stringify(foreign)}), {code:'EPERM'});
    assert.throws(() => fs.writeFileSync(${JSON.stringify(own)}, 'forbidden'), {code:'EPERM'});
    assert.throws(() => cp.execFileSync('/bin/sh', ['-c', 'exit 0']), {code:'EPERM'});
    net.connect({host:'127.0.0.1',port:${address.port}})
      .once('connect', () => process.exit(1))
      .once('error', (error) => {
        assert.equal(error.code, 'EPERM');
        console.log(JSON.stringify(['own_read','foreign_read_denied','workspace_write_denied','spawn_denied','network_denied']));
      });
  `
  const result = spawnSync(launch.cmd, [...launch.args.slice(0,3), '-e', source],
     { cwd:workspace, env:launch.env, encoding:'utf8',timeout:10000 })
  if (result.status !== 0) throw new Error(result.stderr)
  if (await readFile(own,'utf8') !== 'own') throw new Error('workspace was written')
  console.log(result.stdout)
} finally {
  server.close()
  await rm(root,{recursive:true,force:true})
}
