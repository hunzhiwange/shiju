"""Install only reviewed CLI inputs into a disposable directory; never patch global CLIs."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import urllib.request
import os

assets, runtime, agent = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
contracts = json.loads((assets / 'contracts.json').read_text())
contract = contracts[agent]

def run(args, cwd=None):
    subprocess.run(args, cwd=cwd, check=True, stdin=subprocess.DEVNULL)

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def check_files(root, files):
    for name, expected in files.items():
        if digest(root / name) != expected:
            raise RuntimeError('Reviewed artifact checksum mismatch: ' + name)

if contract.get('layout') == 'standalone':
    import platform
    import zipfile
    if agent != 'kiro' or platform.system() != 'Darwin' or platform.machine() != 'arm64':
        raise RuntimeError('Kiro reviewed runtime is Darwin-arm64 only')
    archive = assets / 'kiro.zip'
    with urllib.request.urlopen(contract['artifact_url'], timeout=120) as response, archive.open('wb') as output:
        shutil.copyfileobj(response, output)
    if digest(archive) != contract['archive_sha256']:
        raise RuntimeError('Kiro fixed archive checksum mismatch')
    runtime.mkdir()
    native = runtime / 'native/kiro-cli'
    native.parent.mkdir()
    with zipfile.ZipFile(archive) as package:
        native.write_bytes(package.read('kirocli/bin/kiro-cli'))
    native.chmod(0o700)
    if digest(native) != contract['files']['native/kiro-cli']:
        raise RuntimeError('Kiro fixed executable checksum mismatch')
    run([str(native), '_', 'extract-runtime', '--dir', str(runtime / 'runtime')])
    server = runtime / 'runtime/3.0.0-96d7248d901e42e1/kas/node_modules/@kiro/agent/dist/server/acp-server.js'
    run([sys.executable, str(assets / 'kiro/apply.py'), str(server)])
    shutil.copy2(assets / 'kiro/launcher.py', runtime / 'kiro-cli')
    (runtime / 'kiro-cli').chmod(0o700)
    check_files(runtime, contract['files'])
    raise SystemExit(0)
if contract.get('layout') == 'python':
    uv = sys.argv[4] if len(sys.argv) > 4 and sys.argv[4] else shutil.which('uv') or str(Path.home() / '.local/bin/uv')
    run([uv, 'venv', '--python', '3.12', '--relocatable', str(runtime)])
    python = runtime / 'bin/python'
    if agent == 'vibe':
        wheel_url = contract['wheel_url']
    else:
        with urllib.request.urlopen('https://pypi.org/pypi/hermes-agent/0.19.0/json', timeout=30) as response:
            release = json.load(response)
        wheel_url = next(row['url'] for row in release['urls'] if row['digests']['sha256'] == contract['wheel_sha256'])
    wheel = assets / wheel_url.rsplit('/', 1)[1]
    with urllib.request.urlopen(wheel_url, timeout=60) as response, wheel.open('wb') as output:
        shutil.copyfileobj(response, output)
    if digest(wheel) != contract['wheel_sha256']:
        raise RuntimeError('Pinned Python wheel checksum mismatch')
    package = str(wheel) + ('[acp,cli]' if agent == 'hermes' else '')
    run([uv, 'pip', 'install', '--python', str(python), '--index-url', 'https://pypi.org/simple', package])
    site = runtime / 'lib/python3.12/site-packages'
    if agent == 'vibe':
        run([str(python), str(assets / 'vibe/apply_patch.py')])
    else:
        run([str(python), str(assets / 'hermes/apply_patch.py'), '--package-dir', str(site)])
    check_files(site, contract['files'])
    raise SystemExit(0)
if contract.get('layout') == 'native':
    # This is the real upstream Rust CLI, built from the fixed, reviewed source.
    # Cargo inherits the caller's shared-slot directories when run from this repository.
    archive = assets / 'goose.tar.gz'
    with urllib.request.urlopen(contract['source_url'], timeout=60) as response, archive.open('wb') as output:
        shutil.copyfileobj(response, output)
    if digest(archive) != contract['source_sha256'] or digest(assets / 'goose.patch') != contract['patch_sha256']:
        raise RuntimeError('Goose reviewed source/patch checksum mismatch')
    source = assets / 'goose-source'
    source.mkdir()
    with tarfile.open(archive) as tar:
        members = []
        for member in tar.getmembers():
            name = Path(member.name)
            if name.is_absolute() or '..' in name.parts:
                raise RuntimeError('Unsafe Goose source archive entry')
            # Upstream bin/ contains Hermit developer-tool symlinks, not CLI sources.
            # Build with the gateway toolchain and never extract or execute these shims.
            if member.issym() and name.parts[:2] == ('goose-1.57.0', 'bin'):
                continue
            if not (member.isfile() or member.isdir()):
                raise RuntimeError('Unsafe Goose source archive entry')
            members.append(member)
        tar.extractall(source, members=members)
    source = source / 'goose-1.57.0'
    run(['patch', '--batch', '--forward', '-p1', '-d', str(source), '-i', str(assets / 'goose.patch')])
    target = Path(os.environ.get('CARGO_TARGET_DIR', str(assets / 'cargo-target'))).absolute()
    build_env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_BUILD_DIR=str(target))
    subprocess.run(['cargo', 'build', '-p', 'goose-cli', '--bin', 'goose', '--no-default-features', '--features', 'rustls-tls,bundled-mcp', '--locked'], cwd=source, env=build_env, check=True)
    runtime.mkdir()
    shutil.copy2(target / 'debug/goose', runtime / 'goose')
    (runtime / 'build-receipt.json').write_text(json.dumps({
        'version': contract['version'], 'source_sha256': contract['source_sha256'],
        'patch_sha256': contract['patch_sha256'], 'sha256': digest(runtime / 'goose')}))
    raise SystemExit(0)

packages = [contract['package'] + '@' + contract['version']]
if 'engine' in contract:
    packages.append(contract['engine'] + '@' + contract['engine_version'])
if agent == 'iflow':
    runtime.mkdir()
    lock_input = json.loads((assets / 'iflow-lock.json').read_text())
    (runtime / 'package.json').write_text(json.dumps({'private': True, 'dependencies': lock_input['packages']['']['dependencies']}))
    shutil.copy2(assets / 'iflow-lock.json', runtime / 'package-lock.json')
    run(['npm', 'ci', '--prefix', str(runtime), '--registry=https://registry.npmjs.org', '--ignore-scripts', '--prefer-offline', '--no-audit', '--no-fund', '--fetch-timeout=20000', '--fetch-retries=2'])
else:
    run(['npm', 'install', '--prefix', str(runtime), '--registry=https://registry.npmjs.org',
         '--ignore-scripts', '--prefer-offline', '--fetch-timeout=20000', '--fetch-retries=2', '--fetch-retry-mintimeout=1000', '--fetch-retry-maxtimeout=5000', '--no-audit', '--no-fund', '--save-exact', *packages])
lock = json.loads((runtime / 'package-lock.json').read_text())['packages']
if lock['node_modules/' + contract['package']]['integrity'] != contract['integrity']:
    raise RuntimeError('Supplier archive integrity mismatch')
root = runtime / 'node_modules' / contract['package']
package = json.loads((root / 'package.json').read_text())
if (package['name'], package['version']) != (contract['package'], contract['version']):
    raise RuntimeError('Supplier package identity mismatch')
if agent == 'kilocode':
    import platform
    if platform.system() != 'Darwin' or platform.machine() != 'arm64':
        raise RuntimeError('Kilocode reviewed skill runtime is Darwin-arm64 only')
    native = runtime / 'node_modules/@kilocode/cli-darwin-arm64/bin/kilo'
    if digest(native) != contract['native_sha256']:
        raise RuntimeError('Kilocode fixed executable checksum mismatch')
    reviewed = root / 'reviewed/kilocode'
    reviewed.parent.mkdir()
    shutil.copy2(native, reviewed)
    run([sys.executable, str(assets / 'kilocode/apply.py'), str(reviewed)])
    for name in ('kilo', 'kilocode'):
        link = runtime / 'node_modules/.bin' / name
        link.unlink()
        link.symlink_to(Path('../@kilocode/cli/reviewed/kilocode'))
if agent == 'copilot':
    # The fixed SEA exposes its bundled JS through the vendor's extraction mode.
    # Extract only into this disposable installation, never the user's CLI cache.
    program = "console.log(require('node:module').createRequire(process.argv[1]).resolve('@github/copilot-darwin-arm64'))"
    native = Path(subprocess.check_output(['node', '-e', program, str(root / 'npm-loader.js')], text=True).strip())
    if digest(native) != contract['native_sha256']:
        raise RuntimeError('Copilot fixed SEA checksum mismatch')
    cache = runtime / 'sea-cache'
    extraction_env = dict(os.environ, COPILOT_SEA_EXTRACT_ONLY='1', COPILOT_PKG_CACHE_HOME=str(cache))
    subprocess.run([str(native), '--binary-version'], env=extraction_env, check=True, stdin=subprocess.DEVNULL)
    extracted = cache / 'pkg/darwin-arm64' / contract['version']
    shutil.move(str(extracted), root / 'embedded')
    run([sys.executable, str(assets / 'copilot/apply.py'), str(root / 'embedded')])
    link = runtime / 'node_modules/.bin/copilot'
    link.unlink()
    link.symlink_to(Path('../@github/copilot/embedded/index.js'))
if agent in ('codex', 'gemini'):
    check_files(root, contract['original'])
    run(['patch', '--batch', '--forward', '-p1', '-d', str(root), '-i', str(assets / (agent + '.patch'))])
elif agent in ('iflow', 'qwen', 'codebuddy', 'snow'):
    run([sys.executable, str(assets / agent / 'apply.py'), str(root)])
elif agent == 'pi':
    source = runtime / 'pi-source'
    run(['git', 'clone', '--no-checkout', 'https://github.com/svkozak/pi-acp.git', str(source)])
    run(['git', 'checkout', '--detach', '8be5f22769f71d759aa48d798416c56e8be101ef'], cwd=source)
    if digest(source / 'package-lock.json') != 'e257ee4cbe345ee0de44714d34401ed2ed14de41abc2c6ed7511b1a3d923c606':
        raise RuntimeError('Pi source lock checksum mismatch')
    run(['git', 'apply', '--unidiff-zero', str(assets / 'pi.patch')], cwd=source)
    run(['npm', 'ci', '--ignore-scripts', '--registry=https://registry.npmjs.org', '--no-audit', '--no-fund'], cwd=source)
    run(['npm', 'run', 'build'], cwd=source)
    shutil.copy2(source / 'dist/index.js', root / 'dist/index.js')
check_files(root, contract['files'])
if agent == 'pi':
    engine = runtime / 'node_modules/@mariozechner/pi-coding-agent/dist/cli.js'
    if digest(engine) != 'e959f463b06ddd15ed882783ac02f39ecaeef950ef967da86380f39dda6595fa':
        raise RuntimeError('Pi native engine checksum mismatch')
