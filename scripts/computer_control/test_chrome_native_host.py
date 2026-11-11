import importlib.util
import json
import os
from pathlib import Path
import shutil
import socket
import struct
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).with_name("chrome_native_host.py")
spec = importlib.util.spec_from_file_location("chrome_native_host", SOURCE)
host = importlib.util.module_from_spec(spec)
spec.loader.exec_module(host)


class NativeHostTests(unittest.TestCase):
    def test_split_and_coalesced_native_frames(self):
        parser = host.Frames("=")
        frame = struct.pack("=I", 2) + b"{}"
        self.assertEqual(parser.feed(frame[:1]), [])
        self.assertEqual(parser.feed(frame[1:5]), [])
        self.assertEqual(parser.feed(frame[5:] + frame), [b"{}", b"{}"])

    def test_invalid_lengths_rejected_before_body(self):
        for size in (0, host.MAX_FRAME + 1, 0xFFFFFFFF):
            with self.assertRaises(ValueError):
                host.Frames("=").feed(struct.pack("=I", size))

    def test_install_remove_only_owned_user_registration(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(Path, "home", return_value=Path(temp)):
            directory = Path(temp) / ".shiju" / "computer-control"
            manifest = host.install(directory)
            self.assertEqual(json.loads(manifest.read_text())["allowed_origins"], [host.ORIGIN])
            self.assertEqual((directory / "chrome_native_host.py").stat().st_mode & 0o777, 0o700)
            with self.assertRaises(FileExistsError):
                host.install(directory)
            self.assertTrue(manifest.exists())
            unrelated = manifest.with_name("unrelated.json")
            unrelated.write_text("preserve")
            host.uninstall(directory)
            self.assertFalse(manifest.exists())
            self.assertEqual(unrelated.read_text(), "preserve")
            with self.assertRaises(ValueError):
                host.install(Path(temp) / "elsewhere")

    def test_symlink_install_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(Path, "home", return_value=Path(temp)):
            root = Path(temp) / ".shiju"
            root.mkdir()
            (root / "computer-control").symlink_to(Path(temp), target_is_directory=True)
            with self.assertRaises(ValueError):
                host.install(root / "computer-control")

    def test_real_subprocess_frames_and_disconnect(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            shutil.copyfile(SOURCE, root / SOURCE.name)
            with socket.socket(socket.AF_UNIX) as server:
                server.bind(str(root / "chrome.sock"))
                (root / "chrome.sock").chmod(0o600)
                server.listen(1)
                server.settimeout(3)
                process = subprocess.Popen([os.sys.executable, str(root / SOURCE.name), host.ORIGIN],
                                           stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                try:
                    with server.accept()[0] as connection:
                        connection.settimeout(3)
                        native = struct.pack("=I", 2) + b"{}"
                        process.stdin.write(native[:2]); process.stdin.flush()
                        process.stdin.write(native[2:]); process.stdin.flush()
                        stream = connection.makefile("rb")
                        self.assertEqual(stream.read(6), struct.pack(">I", 2) + b"{}")
                        connection.sendall(struct.pack(">I", 2) + b"{}")
                        self.assertEqual(process.stdout.read(6), native)
                        stream.close()
                    self.assertEqual(process.wait(timeout=3), 0)
                finally:
                    process.kill() if process.poll() is None else None
                    process.communicate(timeout=3)

    def test_real_subprocess_oversize_native_frame_exits(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            shutil.copyfile(SOURCE, root / SOURCE.name)
            with socket.socket(socket.AF_UNIX) as server:
                server.bind(str(root / "chrome.sock"))
                (root / "chrome.sock").chmod(0o600)
                server.listen(1)
                server.settimeout(3)
                process = subprocess.Popen([os.sys.executable, str(root / SOURCE.name), host.ORIGIN],
                                           stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                try:
                    with server.accept()[0]:
                        out, error = process.communicate(struct.pack("=I", host.MAX_FRAME + 1), timeout=3)
                        self.assertEqual(process.returncode, 1)
                        self.assertEqual(out, b"")
                        self.assertEqual(error, b"")
                finally:
                    process.kill() if process.poll() is None else None
                    process.communicate(timeout=3)

    def test_wrong_origin_has_no_stdout(self):
        result = subprocess.run([os.sys.executable, str(SOURCE), "chrome-extension://wrong/"],
                                capture_output=True, timeout=3)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, b"")
        self.assertEqual(result.stderr, b"")

    def test_unsafe_socket_directory_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            root.chmod(0o755)
            with self.assertRaises(ValueError):
                host.relay(host.ORIGIN, root)


# Real browser fixture for the opt-in Rust test. Only CDP launches Chrome, loads
# the unchanged extension and grants activeTab. Native Messaging is never mocked.
LIVE_DRIVER = r"""
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { readFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
const profile = process.env.HOME + '/.shiju/computer-control/chrome-profile';
await rm(join(profile, 'DevToolsActivePort'), {force:true});
const chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
  `--user-data-dir=${profile}`, '--remote-debugging-port=0', '--enable-unsafe-extension-debugging',
  '--no-first-run', '--no-default-browser-check', '--headless=new', 'about:blank',
], { stdio: ['ignore', 'ignore', 'ignore'] });
const server = createServer((req, res) => {
  res.setHeader('Content-Type', 'text/html; charset=utf-8');
  res.end('<!doctype html><title>ShiJu CC08 fixture</title><input id="name"><button id="button" onclick="document.getElementById(\'out\').textContent=\'clicked\'">Submit</button><p id="out">ready</p>');
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const fixtureUrl = `http://127.0.0.1:${server.address().port}/fixture`;
let socket;
try {
  let address;
  for(let i=0;i<100;i++) {
    try { address = (await readFile(join(profile,'DevToolsActivePort'),'utf8')).trim().split('\n'); break; } catch {}
    await new Promise(r=>setTimeout(r,100));
  }
  if(!address) throw new Error('DevTools startup failed');
  socket = new WebSocket(`ws://127.0.0.1:${address[0]}${address[1]}`);
  await new Promise((resolve,reject)=>{socket.onopen=resolve;socket.onerror=reject;});
  let next=0;const waiting=new Map();
  socket.onmessage = event => { const m=JSON.parse(event.data); if(m.id){const p=waiting.get(m.id);waiting.delete(m.id);m.error?p.reject(new Error(JSON.stringify(m.error))):p.resolve(m.result);} };
  const send=(method,params={},sessionId)=>new Promise((resolve,reject)=>{const id=++next;waiting.set(id,{resolve,reject});socket.send(JSON.stringify({id,method,params,...(sessionId?{sessionId}:{})}));});
  const version=await send('Browser.getVersion');
  const schema=await (await fetch(`http://127.0.0.1:${address[0]}/json/protocol`)).json();
  console.log(JSON.stringify(schema.domains.find(d=>d.domain==='Extensions').commands.find(c=>c.name==='triggerAction')));
  const loaded=await send('Extensions.loadUnpacked',{path:process.env.SHIJU_CC09_EXTENSION});
  let worker;
  for(let i=0;i<50;i++){
    worker=(await send('Target.getTargets')).targetInfos.find(t=>t.type==='service_worker'&&t.url.includes(loaded.id));
    if(worker)break;await new Promise(r=>setTimeout(r,100));
  }
  if(!worker)throw new Error('No extension service worker');
  const session=(await send('Target.attachToTarget',{targetId:worker.targetId,flatten:true})).sessionId;
  await new Promise(r=>setTimeout(r,300));
  const inspect=await send('Runtime.evaluate',{expression:'JSON.stringify({id:chrome.runtime.id,manifest:chrome.runtime.getManifest(),nativeMessaging:typeof chrome.runtime.connectNative,scripting:typeof chrome.scripting.executeScript})',returnByValue:true},session);
  if(inspect.exceptionDetails)throw new Error(JSON.stringify(inspect.exceptionDetails));
  const pageTarget = await send('Target.createTarget', {url:fixtureUrl});
  const pageSession = (await send('Target.attachToTarget',{targetId:pageTarget.targetId,flatten:true})).sessionId;
  await new Promise(r=>setTimeout(r,300));
  await send('Extensions.triggerAction',{id:loaded.id,targetId:(await send('Target.getTargets',{filter:[{type:'tab',exclude:false}]})).targetInfos.find(t=>t.url===fixtureUrl).targetId});
  await new Promise(r=>setTimeout(r,200));
  console.log(JSON.stringify({version,extensionId:loaded.id,profile,worker:worker.url}));
  await new Promise(r=>setTimeout(r,4000));
  await send('Extensions.uninstall',{id:loaded.id});
  await send('Browser.close');
} finally {
  socket?.close();chrome.kill('SIGTERM');
  await new Promise(resolve=>{if(chrome.exitCode!==null)resolve();else chrome.once('exit',resolve);});

  await new Promise(resolve=>server.close(resolve));
}
"""


if __name__ == "__main__":
    if os.sys.argv[1:] == ["--live-driver"]:
        environment = dict(os.environ, SHIJU_CC09_EXTENSION=str(SOURCE.parents[2] / "plugins/shiju-computer-control/chrome"))
        raise SystemExit(subprocess.run(["node", "--input-type=module", "-e", LIVE_DRIVER], env=environment).returncode)
    else:
        unittest.main()
