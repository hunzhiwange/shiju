"""Install only reviewed CLI inputs into a disposable directory; never patch global CLIs."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import time
import urllib.error
import urllib.request
import os
import re

assets, runtime, agent = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
def run(args, cwd=None):
    subprocess.run(args, cwd=cwd, check=True, stdin=subprocess.DEVNULL)

def digest(path):
    result = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            result.update(block)
    return result.hexdigest()

def install_linux_cli(url, archive_sha256, member_name, bin_name, executable_sha256):
    download = assets / 'linux-download'
    headers = ['--header', 'Accept: application/octet-stream'] if agent == 'corust-agent' else []
    run(['/usr/bin/curl', '--disable', '--fail', '--silent', '--show-error', '--location',
         '--proto', '=https', '--proto-redir', '=https', '--retry', '2',
         '--max-time', '240', *headers, '--output', str(download), url])
    if agent == 'corust-agent':
        from install_linux import install_native
        install_native(download, runtime)
        return
    if agent == 'trae':
        from install_linux_native import install
        install(download, runtime)
        return
    runtime.mkdir(mode=0o700)
    executable = runtime / bin_name
    if archive_sha256:
        if digest(download) != archive_sha256:
            raise RuntimeError('Fixed Linux CLI archive checksum mismatch')
        with tarfile.open(download, 'r:gz') as archive:
            members = [entry for entry in archive if entry.name == member_name]
            if len(members) != 1 or not members[0].isfile():
                raise RuntimeError('Unexpected fixed Linux CLI archive layout')
            with archive.extractfile(members[0]) as source, executable.open('xb') as output:
                shutil.copyfileobj(source, output)
    else:
        download.rename(executable)
    if digest(executable) != executable_sha256:
        raise RuntimeError('Fixed Linux CLI executable checksum mismatch')
    executable.chmod(0o700)

if sys.argv[4:5] == ['--linux-cli']:
    install_linux_cli(*sys.argv[5:])
    raise SystemExit(0)

contracts = json.loads((assets / 'contracts.json').read_text())
contract = contracts[agent]

if agent == 'amp-acp' and (assets / 'amp-acp/linux-runtime.json').is_file():
    amp_assets = assets / 'amp-acp'
    reviewed = json.loads((amp_assets / 'linux-runtime.json').read_text())
    archives = []
    for name in ('amp-acp', '@sourcegraph/amp'):
        archive = amp_assets / (name.rsplit('/', 1)[-1] + '.tgz')
        run(['/usr/bin/curl', '--disable', '--fail', '--silent', '--show-error', '--location',
             '--proto', '=https', '--proto-redir', '=https', '--retry', '2',
             '--max-time', '120', '--output', str(archive), reviewed['archives'][name]['url']])
        archives.append(str(archive))
    run([sys.executable, '-I', str(amp_assets / 'install_linux.py'), str(runtime), *archives])
    raise SystemExit(0)

def check_files(root, files):
    for name, expected in files.items():
        if digest(root / name) != expected:
            raise RuntimeError('Reviewed artifact checksum mismatch: ' + name)

def apply_glm_patch(root, patch, reviewed_files):
    # The pinned inputs and outputs are checked on both sides of an exact patch.
    # Applying hunks by source line avoids a dependency on a Unix patch binary.
    for section in re.split(r"(?m)(?=^--- a/)", patch.read_text(encoding="utf-8")):
        if not section:
            continue
        old, new, *lines = section.splitlines(keepends=True)
        name = old.strip().removeprefix("--- a/")
        if name not in reviewed_files or new.strip() != "+++ b/" + name:
            raise RuntimeError("Unexpected GLM patch path")
        path = root / name
        source = path.read_bytes().decode("utf-8").splitlines(keepends=True)
        result = []
        cursor = 0
        hunks = re.split(r"(?m)(?=^@@ )", "".join(lines))
        if not hunks or hunks.pop(0):
            raise RuntimeError("Malformed GLM patch")
        for hunk in hunks:
            header, *changes = hunk.splitlines(keepends=True)
            match = re.fullmatch(r"@@ -(\d+)(?:,\d+)? \+\d+(?:,\d+)? @@(?: .*)?\n", header)
            if not match or not changes or any(line[0] not in " +-" for line in changes):
                raise RuntimeError("Malformed GLM patch hunk")
            start = int(match[1]) - 1
            before = [line[1:] for line in changes if line[0] in " -"]
            if start < cursor or source[start:start + len(before)] != before:
                raise RuntimeError("GLM patch does not match reviewed source")
            result.extend(source[cursor:start])
            result.extend(line[1:] for line in changes if line[0] in " +")
            cursor = start + len(before)
        result.extend(source[cursor:])
        path.write_bytes("".join(result).encode("utf-8"))

if agent == 'dimcode':
    import platform
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise RuntimeError('DimCode reviewed runtime is Darwin-arm64 only')
    archive = assets / 'dimcode.tgz'
    run(['/usr/bin/curl', '--disable', '--fail', '--silent', '--show-error', '--location',
         '--proto', '=https', '--proto-redir', '=https', '--retry', '2',
         '--max-time', '80', '--output', str(archive), contract['artifact_url']])
    if digest(archive) != contract['archive_sha256']:
        raise RuntimeError('DimCode fixed archive checksum mismatch')
    native = assets / 'dimcode-native'
    with tarfile.open(archive) as package:
        member = package.getmember('package/bin/dimcode')
        if not member.isfile():
            raise RuntimeError('Unexpected DimCode fixed archive layout')
        with package.extractfile(member) as source, native.open('xb') as output:
            shutil.copyfileobj(source, output)
    runtime.mkdir()
    executable = runtime / contract['bin']
    run([sys.executable, str(assets / 'dimcode/apply.py'), str(native), str(executable)])
    run(['/usr/bin/codesign', '--force', '--sign', '-', '--identifier', 'dim',
         '--timestamp=none', str(executable)])
    run(['/usr/bin/codesign', '--verify', '--strict', str(executable)])
    check_files(runtime, contract['files'])
    raise SystemExit(0)

if agent == 'cortex-code':
    import platform
    import runpy
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise RuntimeError('Cortex Code reviewed runtime is Darwin-arm64 only')
    archive = assets / 'cortex-code.tar.gz'
    if not archive.is_file():
        for _ in range(2):
            result = subprocess.run([
                '/usr/bin/curl', '--disable', '--fail', '--silent', '--show-error', '--location',
                '--proto', '=https', '--proto-redir', '=https', '--max-time', '270',
                '--continue-at', '-', '--output', str(archive), contract['artifact_url'],
            ], stdin=subprocess.DEVNULL)
            if result.returncode == 0:
                break
        else:
            raise RuntimeError('Cortex Code fixed archive download failed')
    if digest(archive) != contract['archive_sha256']:
        raise RuntimeError('Cortex Code fixed archive checksum mismatch')
    prefix = f"coco-{contract['version']}-darwin-arm64"
    runtime.mkdir()
    with tarfile.open(archive) as package:
        for member in package:
            path = Path(member.name)
            if (path.parts[:1] != (prefix,) or '..' in path.parts
                    or not (member.isfile() or member.isdir())):
                raise RuntimeError('Unsafe Cortex Code archive entry')
            destination = runtime.joinpath(*path.parts[1:])
            if member.isdir():
                destination.mkdir(parents=True, exist_ok=True)
                continue
            destination.parent.mkdir(parents=True, exist_ok=True)
            with package.extractfile(member) as source, destination.open('xb') as output:
                shutil.copyfileobj(source, output)
            destination.chmod(0o700 if member.mode & 0o111 else 0o600)
    native = runtime / 'cortex-native'
    (runtime / 'cortex').rename(native)
    run([sys.executable, str(assets / 'cortex-code/apply.py'), str(native), str(runtime / 'cortex-local')])
    launcher = runtime / 'cortex'
    shutil.copyfile(assets / 'cortex-code/launcher.py', launcher)
    launcher.chmod(0o700)
    # Scope Cortex's credential isolation to its installed copy of the shared stdio bridge.
    denials = runpy.run_path(str(launcher))['CREDENTIAL_DENIALS']
    bridge = (assets / 'cortex-code/mcp_bridge.py').read_bytes()
    (runtime / 'mcp_bridge.py').write_bytes(bridge + f'\nMCP_SANDBOX += {denials!r}\n'.encode())
    check_files(runtime, contract['files'])
    raise SystemExit(0)

if agent == 'omp':
    import platform
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise RuntimeError('OMP reviewed runtime is Darwin-arm64 only')
    runtime.mkdir()
    executable = runtime / contract['bin']
    # Give the large release time to transfer, but resume promptly after a stall.
    for _ in range(3):
        result = subprocess.run([
            '/usr/bin/curl', '--disable', '--fail', '--silent', '--show-error', '--location',
            '--proto', '=https', '--proto-redir', '=https', '--connect-timeout', '20',
            '--speed-limit', '1024', '--speed-time', '30', '--max-time', '300',
            '--continue-at', '-', '--output', str(executable), contract['artifact_url'],
        ], stdin=subprocess.DEVNULL)
        if result.returncode == 0:
            break
        if result.returncode == 33:
            executable.unlink(missing_ok=True)
    else:
        raise RuntimeError(f'OMP release download failed after 3 attempts (curl exit {result.returncode})')
    check_files(runtime, contract['files'])
    executable.chmod(0o700)
    raise SystemExit(0)
if agent == 'stakpak':
    import platform
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise RuntimeError('Stakpak reviewed runtime is Darwin-arm64 only')
    archive = assets / 'stakpak.tar.gz'
    run(['/usr/bin/curl', '--disable', '--fail', '--silent', '--show-error', '--location',
         '--proto', '=https', '--proto-redir', '=https', '--retry', '2',
         '--max-time', '60', '--output', str(archive), contract['source_url']])
    patch = assets / 'stakpak/skills.patch'
    if digest(archive) != contract['source_sha256'] or digest(patch) != contract['patch_sha256']:
        raise RuntimeError('Stakpak source or patch checksum mismatch')
    source = assets / f"agent-{contract['version']}"
    with tarfile.open(archive) as package:
        members = package.getmembers()
        if not members or any(
            not Path(member.name).parts or Path(member.name).parts[0] != source.name
            or Path(member.name).is_absolute() or '..' in Path(member.name).parts
            or not (member.isdir() or member.isfile()) for member in members
        ):
            raise RuntimeError('Unexpected Stakpak fixed source layout')
        package.extractall(assets, members=members)
    run(['patch', '--batch', '--forward', '-p1', '-d', str(source), '-i', str(patch)])
    target = Path(os.environ.get('CARGO_TARGET_DIR', str(assets / 'cargo-target'))).absolute()
    build_env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_BUILD_DIR=str(target))
    subprocess.run(
        ['cargo', '+1.94.1', 'build', '--locked', '-j', '1', '-p', 'stakpak', '--bin', 'stakpak'],
        cwd=source, env=build_env, check=True, stdin=subprocess.DEVNULL,
    )
    runtime.mkdir(mode=0o700)
    native = runtime / 'stakpak-native'
    shutil.copyfile(target / 'debug/stakpak', native)
    native.chmod(0o700)
    launcher = runtime / 'stakpak'
    shutil.copyfile(assets / 'stakpak/launcher.py', launcher)
    shutil.copyfile(assets / 'stakpak/mcp.py', runtime / 'mcp.py')
    shutil.copyfile(assets / 'poolside/mcp_bridge.py', runtime / 'mcp_bridge.py')
    shutil.copyfile(assets / 'cortex-code/mcp_bridge.py', runtime / 'shiju-stdio.py')
    launcher.chmod(0o700)
    check_files(runtime, contract['files'])
    (runtime / 'build-receipt.json').write_text(json.dumps({
        'version': contract['version'], 'source_sha256': contract['source_sha256'],
        'patch_sha256': contract['patch_sha256'], 'sha256': digest(native),
    }))
    raise SystemExit(0)
if agent == 'mimo-code':
    import platform
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise RuntimeError('MiMo Code reviewed runtime is Darwin-arm64 only')

    def download(url, destination, sha256):
        for _ in range(3):
            result = subprocess.run([
                '/usr/bin/curl', '--fail', '--silent', '--show-error', '--location',
                '--proto', '=https', '--proto-redir', '=https', '--max-time', '50',
                '--continue-at', '-', '--output', str(destination), url,
            ], stdin=subprocess.DEVNULL)
            if result.returncode == 0:
                break
        else:
            raise RuntimeError('MiMo Code reviewed download failed')
        if digest(destination) != sha256:
            raise RuntimeError('MiMo Code fixed archive checksum mismatch')

    archive = assets / 'mimo-code-source.tar.gz'
    bun_archive = assets / 'mimo-code-bun.tgz'
    patch = assets / 'mimo-code/stdio-only.patch'
    download(contract['source_url'], archive, contract['source_sha256'])
    download(contract['bun_url'], bun_archive, contract['bun_sha256'])
    if digest(patch) != contract['patch_sha256']:
        raise RuntimeError('MiMo Code ACP patch checksum mismatch')

    extracted = assets / 'mimo-code-source'
    extracted.mkdir()
    with tarfile.open(archive) as package:
        entries = package.getmembers()
        for entry in entries:
            path = Path(entry.name)
            if path.is_absolute() or '..' in path.parts or not (entry.isdir() or entry.isfile()):
                raise RuntimeError('Unsafe MiMo Code source archive')
            entry.mode &= 0o700
        package.extractall(extracted, members=entries)
    source = extracted / 'XiaomiMiMo-MiMo-Code-3704f8d'

    bun = assets / 'bun'
    with tarfile.open(bun_archive) as package:
        member = package.getmember('package/bin/bun')
        if not member.isfile():
            raise RuntimeError('Unexpected Bun executable archive entry')
        with package.extractfile(member) as input_file, bun.open('xb') as output_file:
            shutil.copyfileobj(input_file, output_file)
    bun.chmod(0o700)
    run(['/usr/bin/patch', '-p1', '-i', str(patch)], source)

    home = assets / 'build-home'
    home.mkdir(mode=0o700)
    models = assets / 'mimo-code-models.json'
    models.write_text('{}')
    build_env = {
        'PATH': f'{assets}:/usr/bin:/bin',
        'HOME': str(home),
        'CI': 'true',
        'MIMOCODE_VERSION': contract['version'],
        'MIMOCODE_CHANNEL': 'latest',
        'MIMOCODE_COMMIT_SHA': '3704f8d',
        'MODELS_DEV_API_JSON': str(models),
    }
    for concurrency in ('16', '4'):
        result = subprocess.run([
            bun, 'install', '--registry', 'https://registry.npmjs.org',
            '--network-concurrency', concurrency, '--frozen-lockfile', '--ignore-scripts',
        ], cwd=source, env=build_env, stdin=subprocess.DEVNULL)
        if result.returncode == 0:
            break
    else:
        raise RuntimeError('MiMo Code locked dependency installation failed')
    subprocess.run(
        [bun, 'run', 'script/build.ts', '--single', '--skip-install'],
        cwd=source / 'packages/cli', env=build_env, check=True, stdin=subprocess.DEVNULL,
    )

    runtime.mkdir()
    native = runtime / 'mimo'
    shutil.copy2(source / 'packages/cli/dist/mimocode-darwin-arm64/bin/mimo', native)
    native.chmod(0o700)
    (runtime / 'build-receipt.json').write_text(json.dumps({
        'version': contract['version'], 'source_sha256': contract['source_sha256'],
        'patch_sha256': contract['patch_sha256'], 'sha256': digest(native),
    }))
    raise SystemExit(0)
if agent == 'vtcode':
    native = assets / 'vtcode-acp'
    if native.is_file():
        if digest(native) != contract['files'][contract['bin']]:
            raise RuntimeError('Packaged VT Code ACP checksum mismatch')
        runtime.mkdir()
        executable = runtime / contract['bin']
        shutil.copyfile(native, executable)
        executable.chmod(0o700)
        raise SystemExit(0)
    archive = assets / 'vtcode.tar.gz'
    for _ in range(3):
        result = subprocess.run([
            '/usr/bin/curl', '--disable', '--fail', '--silent', '--show-error', '--location',
            '--proto', '=https', '--proto-redir', '=https', '--max-time', '80',
            '--speed-time', '20', '--speed-limit', '65536', '--continue-at', '-',
            '--output', str(archive), contract['artifact_url'],
        ], stdin=subprocess.DEVNULL)
        if result.returncode == 0:
            break
        if result.returncode == 33:
            archive.unlink(missing_ok=True)
    else:
        raise RuntimeError('VT Code reviewed download failed')
    if digest(archive) != contract['archive_sha256']:
        raise RuntimeError('VT Code fixed archive checksum mismatch')
    runtime.mkdir()
    executable = runtime / contract['bin']
    with tarfile.open(archive) as package:
        entries = [entry for entry in package if entry.name == './vtcode']
        if len(entries) != 1 or not entries[0].isfile():
            raise RuntimeError('Unexpected VT Code executable archive entry')
        with package.extractfile(entries[0]) as source, executable.open('xb') as output:
            shutil.copyfileobj(source, output)
    check_files(runtime, {contract['bin']: contract['source_sha256']})
    executable.chmod(0o700)
    raise SystemExit(0)
if agent == 'corust-agent':
    import platform
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise RuntimeError('Corust Agent reviewed runtime is Darwin-arm64 only')
    archive = assets / 'corust-agent.tar.gz'
    run(['/usr/bin/curl', '--disable', '--fail', '--silent', '--show-error', '--location',
         '--proto', '=https', '--proto-redir', '=https', '--retry', '2',
         '--max-time', '60', '--header', 'Accept: application/octet-stream',
         '--output', str(archive), contract['artifact_url']])
    if digest(archive) != contract['archive_sha256']:
        raise RuntimeError('Corust Agent fixed archive checksum mismatch')
    runtime.mkdir()
    executable = runtime / 'corust-agent-native'
    with tarfile.open(archive) as package:
        members = package.getmembers()
        if len(members) != 1 or members[0].name != contract['bin'] or not members[0].isfile():
            raise RuntimeError('Unexpected Corust Agent archive layout')
        with package.extractfile(members[0]) as source, executable.open('xb') as output:
            shutil.copyfileobj(source, output)
    launcher = runtime / contract['bin']
    shutil.copy2(assets / 'corust-agent/launcher.py', launcher)
    for module in ('mcp_bridge.py', 'mcp_graph.py'):
        shutil.copy2(assets / 'corust-agent' / module, runtime / module)
    launcher.chmod(0o700)
    check_files(runtime, contract['files'])
    executable.chmod(0o700)
    raise SystemExit(0)
if agent == 'junie':
    import platform
    import zipfile
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise RuntimeError('Junie reviewed runtime is Darwin-arm64 only')
    archive = assets / 'junie.zip'
    for _ in range(3):
        result = subprocess.run([
            '/usr/bin/curl', '--fail', '--silent', '--show-error', '--location',
            '--proto', '=https', '--proto-redir', '=https', '--continue-at', '-',
            '--speed-limit', '1024', '--speed-time', '12', '--max-time', '45',
            '--output', str(archive), contract['artifact_url'],
        ], stdin=subprocess.DEVNULL)
        if result.returncode == 0:
            break
    else:
        raise RuntimeError('Junie fixed archive download failed')
    if digest(archive) != contract['archive_sha256']:
        raise RuntimeError('Junie fixed archive checksum mismatch')
    with zipfile.ZipFile(archive) as package:
        for entry in package.infolist():
            path = Path(entry.filename)
            if path.is_absolute() or '..' in path.parts:
                raise RuntimeError('Unsafe Junie archive path')
    runtime.mkdir()
    run(['/usr/bin/ditto', '-x', '-k', str(archive), str(runtime)])
    run(['/usr/bin/codesign', '--verify', '--deep', '--strict', str(runtime / 'Applications/junie.app')])
    shutil.copy2(assets / 'junie/launcher.py', runtime / 'junie')
    (runtime / 'junie').chmod(0o700)
    shutil.copyfile(assets / 'junie/mcp_bridge.py', runtime / 'mcp_bridge.py')
    shutil.copyfile(assets / 'cortex-code/mcp_bridge.py', runtime / 'mcp_stdio.py')
    check_files(runtime, contract['files'])
    raise SystemExit(0)
if agent == 'poolside':
    import platform
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise RuntimeError('Poolside reviewed runtime is Darwin-arm64 only')
    archive = assets / 'poolside.tar.gz'
    run(['/usr/bin/curl', '--fail', '--silent', '--show-error', '--location',
         '--proto', '=https', '--proto-redir', '=https', '--max-time', '120',
         '--output', str(archive), contract['artifact_url']])
    if digest(archive) != contract['archive_sha256']:
        raise RuntimeError('Poolside fixed archive checksum mismatch')
    runtime.mkdir()
    native = runtime / 'pool-native'
    with tarfile.open(archive) as package:
        if package.getnames() != ['pool-darwin-arm64'] or not package.getmember('pool-darwin-arm64').isfile():
            raise RuntimeError('Unexpected Poolside fixed archive layout')
        with package.extractfile('pool-darwin-arm64') as source, native.open('xb') as output:
            shutil.copyfileobj(source, output)
    shutil.copy2(assets / 'poolside/launcher.py', runtime / 'pool')
    shutil.copyfile(assets / 'poolside/mcp_bridge.py', runtime / 'mcp_bridge.py')
    shutil.copyfile(assets / 'cortex-code/mcp_bridge.py', runtime / 'shiju-stdio.py')
    for executable in (native, runtime / 'pool'):
        executable.chmod(0o700)
    check_files(runtime, contract['files'])
    raise SystemExit(0)
if agent == 'harn':
    import platform
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise RuntimeError('Harn reviewed runtime is Darwin-arm64 only')
    archive = assets / 'harn-source.tar.gz'
    patch = assets / 'harn/skills.patch'
    with urllib.request.urlopen(contract['source_url'], timeout=120) as response, archive.open('wb') as output:
        shutil.copyfileobj(response, output)
    if digest(archive) != contract['source_sha256'] or digest(patch) != contract['patch_sha256']:
        raise RuntimeError('Harn reviewed source/patch checksum mismatch')
    revision = '4914194ae9f56b0046210ca90ef6f927988e727e'
    source = assets / f'harn-{revision}'
    with tarfile.open(archive) as package:
        members = []
        for member in package.getmembers():
            path = Path(member.name)
            if path.is_absolute() or '..' in path.parts or not path.parts or path.parts[0] != source.name:
                raise RuntimeError('Unsafe Harn source entry')
            # Development aliases are unnecessary for the CLI build.
            if member.issym():
                continue
            if not (member.isfile() or member.isdir()):
                raise RuntimeError('Unsafe Harn source entry type')
            members.append(member)
        package.extractall(assets, members=members)
    run(['/usr/bin/patch', '--batch', '--forward', '-p1', '-i', str(patch)], source)
    target = Path(os.environ.get('CARGO_TARGET_DIR', str(assets / 'cargo-target'))).absolute()
    build_env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_BUILD_DIR=str(target),
                     HARN_BUILD_REVISION=revision, HARN_BUILD_FRESHNESS_ID='')
    subprocess.run(['cargo', '+1.99.1', 'build', '--locked', '-j', '1', '-p', 'harn-cli', '--bin', 'harn'],
                   cwd=source, env=build_env, stdin=subprocess.DEVNULL, check=True)
    runtime.mkdir(mode=0o700)
    binary = runtime / 'harn-native'
    shutil.copyfile(target / 'debug/harn', binary)
    for name in ('LICENSE-APACHE', 'LICENSE-MIT'):
        shutil.copyfile(source / name, runtime / name)
    shutil.copyfile(assets / 'harn/launcher.py', runtime / 'harn')
    (runtime / 'harn').chmod(0o700)
    binary.chmod(0o700)
    shutil.copyfile(assets / 'harn/agent.harn', runtime / 'agent.harn')
    shutil.copyfile(assets / 'cortex-code/mcp_bridge.py', runtime / 'shiju_stdio.py')
    check_files(runtime, contract['files'])
    (runtime / 'build-receipt.json').write_text(json.dumps({
        'version': contract['version'], 'source_sha256': contract['source_sha256'],
        'patch_sha256': contract['patch_sha256'], 'engine_sha256': digest(binary),
    }))
    raise SystemExit(0)
if agent == 'trae':
    import platform
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise RuntimeError('TRAE reviewed runtime is Darwin-arm64 only')
    archive = assets / 'trae.tar.gz'
    with urllib.request.urlopen(contract['artifact_url'], timeout=120) as response, archive.open('wb') as output:
        shutil.copyfileobj(response, output)
    if digest(archive) != contract['archive_sha256']:
        raise RuntimeError('TRAE fixed archive checksum mismatch')
    runtime.mkdir()
    native = runtime / 'native/trae-cli'
    native.parent.mkdir()
    with tarfile.open(archive) as package:
        if package.getnames() != ['trae-cli'] or not package.getmember('trae-cli').isfile():
            raise RuntimeError('Unexpected TRAE fixed archive layout')
        native.write_bytes(package.extractfile('trae-cli').read())
    native.chmod(0o700)
    run([sys.executable, str(assets / 'trae/apply.py'), str(native), str(runtime / 'native/trae-local')])
    shutil.copy2(assets / 'trae/launcher.py', runtime / 'traecli')
    (runtime / 'traecli').chmod(0o700)
    check_files(runtime, contract['files'])
    raise SystemExit(0)
if agent == 'droid':
    import platform
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise RuntimeError('Droid reviewed runtime is Darwin-arm64 only')
    native = assets / 'droid-original'
    with urllib.request.urlopen(contract['artifact_url'], timeout=120) as response, native.open('xb') as output:
        shutil.copyfileobj(response, output)
    if digest(native) != contract['source_sha256']:
        raise RuntimeError('Droid original executable checksum mismatch')
    runtime.mkdir()
    run([sys.executable, str(assets / 'droid/apply.py'), str(native), str(runtime / contract['bin'])])
    check_files(runtime, contract['files'])
    raise SystemExit(0)
if agent == 'cursor':
    import platform
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise RuntimeError('Cursor reviewed runtime is Darwin-arm64 only')
    archive = assets / 'cursor.tar.gz'
    request = urllib.request.Request(contract['artifact_url'], headers={'User-Agent': 'ShiJu-ACP-Installer/1.0'})
    with urllib.request.urlopen(request, timeout=120) as response, archive.open('wb') as output:
        shutil.copyfileobj(response, output)
    if digest(archive) != contract['archive_sha256']:
        raise RuntimeError('Cursor fixed archive checksum mismatch')
    runtime.mkdir()
    with tarfile.open(archive) as package:
        members = []
        for member in package.getmembers():
            name = Path(member.name)
            if name.is_absolute() or '..' in name.parts or name.parts[0] != 'dist-package' or not (member.isfile() or member.isdir()):
                raise RuntimeError('Unsafe Cursor archive entry')
            members.append(member)
        package.extractall(runtime, members=members)
    run([sys.executable, str(assets / 'cursor/apply.py'), str(runtime / 'dist-package')])
    check_files(runtime, contract['files'])
    raise SystemExit(0)
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
    package = str(wheel) + ('[acp,cli,mcp]' if agent == 'hermes' else '')
    run([uv, 'pip', 'install', '--python', str(python), '--index-url', 'https://pypi.org/simple', package])
    site = runtime / 'lib/python3.12/site-packages'
    if agent == 'vibe':
        run([str(python), str(assets / 'vibe/apply_patch.py')])
    else:
        run([str(python), str(assets / 'hermes/apply_patch.py'), '--package-dir', str(site)])
        run([str(python), str(assets / 'hermes/coexistence/apply_patch.py'), '--package-dir', str(site)])
    check_files(site, contract['files'])
    raise SystemExit(0)
if agent == 'sigit':
    # Fixed upstream source plus the reviewed host-only tool boundary.
    if not os.environ.get('CARGO_TARGET_DIR'):
        raise RuntimeError('siGit source build requires the repository cargo_slot.py entry')
    archive = assets / 'sigit.tar.gz'
    for attempt in range(3):
        try:
            response = urllib.request.urlopen(contract['source_url'], timeout=60)
            break
        except urllib.error.HTTPError:
            raise
        except urllib.error.URLError as error:
            if attempt == 2:
                raise RuntimeError('Could not connect to the siGit source archive after 3 attempts') from error
            time.sleep(attempt + 1)
    with response, archive.open('wb') as output:
        shutil.copyfileobj(response, output)
    if digest(archive) != contract['source_sha256'] or digest(assets / 'sigit.patch') != contract['patch_sha256']:
        raise RuntimeError('siGit source/patch checksum mismatch')
    source = assets / 'sigit-source'
    source.mkdir()
    with tarfile.open(archive) as package:
        members = []
        for member in package.getmembers():
            name = Path(member.name)
            if name.is_absolute() or '..' in name.parts:
                raise RuntimeError('Unsafe siGit source archive')
            # Fixed upstream documentation aliases; never extract symbolic links.
            if member.issym() and (tuple(name.parts[1:]), member.linkname) in (
                (('.claude', 'skills'), '../.agents/skills'),
                (('AGENTS.md',), '.agents/AGENTS.md'),
                (('CLAUDE.md',), '.agents/AGENTS.md'),
            ):
                continue
            if not (member.isfile() or member.isdir()):
                raise RuntimeError('Unsafe siGit source archive')
            members.append(member)
        package.extractall(source, members=members)
    roots = list(source.iterdir())
    if len(roots) != 1 or not roots[0].is_dir():
        raise RuntimeError('Unexpected siGit source layout')
    source = roots[0]
    run(['patch', '--batch', '--forward', '-p1', '-d', str(source), '-i', str(assets / 'sigit.patch')])
    env = dict(os.environ, MISTRALRS_METAL_PRECOMPILE='0')
    subprocess.run(['cargo', 'build', '--locked', '-p', 'sigit', '--bin', 'sigit'], cwd=source, env=env, check=True)
    runtime.mkdir()
    shutil.copy2(Path(env['CARGO_TARGET_DIR']) / 'debug/sigit', runtime / 'sigit')
    (runtime / 'build-receipt.json').write_text(json.dumps({
        'version': contract['version'], 'source_sha256': contract['source_sha256'],
        'patch_sha256': contract['patch_sha256'], 'sha256': digest(runtime / 'sigit')}))
    raise SystemExit(0)

if agent == 'aionrs':
    archive = assets / 'aionrs.tar.gz'
    with urllib.request.urlopen(contract['source_url'], timeout=45) as response, archive.open('xb') as output:
        shutil.copyfileobj(response, output)
    if digest(archive) != contract['source_sha256']:
        raise RuntimeError('aionrs fixed source checksum mismatch')
    source = assets / 'aionrs-source'
    source.mkdir()
    with tarfile.open(archive) as tar:
        members = tar.getmembers()
        roots = {Path(member.name).parts[0] for member in members if Path(member.name).parts}
        if len(roots) != 1 or any(
            not member.name or Path(member.name).is_absolute() or '..' in Path(member.name).parts
            or not (member.isdir() or member.isfile()) for member in members
        ):
            raise RuntimeError('Unexpected aionrs source archive layout')
        tar.extractall(source, members=members)
    source = source / roots.pop()
    patch = assets / 'aionrs.patch'
    if digest(patch) != contract['patch_sha256']:
        raise RuntimeError('aionrs fixed patch checksum mismatch')
    run(['patch', '--batch', '--forward', '-p1', '-d', str(source), '-i', str(patch)])
    target = Path(os.environ.get('CARGO_TARGET_DIR', str(assets / 'cargo-target'))).absolute()
    build_env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_BUILD_DIR=str(target))
    subprocess.run(
        ['cargo', 'build', '-j', '1', '--locked', '-p', 'aion-cli', '--bin', 'aionrs'],
        cwd=source, env=build_env, check=True, stdin=subprocess.DEVNULL,
    )
    runtime.mkdir(mode=0o700)
    engine = runtime / 'aionrs'
    bridge = runtime / contract['bin']
    shutil.copy2(target / 'debug/aionrs', engine)
    shutil.copy2(assets / 'aionrs-acp', bridge)
    (runtime / 'build-receipt.json').write_text(json.dumps({
        'version': contract['version'], 'source_sha256': contract['source_sha256'],
        'patch_sha256': contract['patch_sha256'],
        'engine_sha256': digest(engine), 'sha256': digest(bridge),
    }))
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
if agent == 'glm-acp-agent':
    patch = assets / 'glm-acp-agent.patch'
    # The patch contract pins LF bytes, including on CRLF Git checkouts.
    if hashlib.sha256(patch.read_bytes().replace(b'\r\n', b'\n')).hexdigest() != contract['patch_sha256']:
        raise RuntimeError('GLM reviewed patch checksum mismatch')
runtime.mkdir()
iflow_linux = None
if agent == 'iflow' and (assets / 'iflow/linux-artifacts.json').is_file():
    import platform
    iflow_linux = json.loads((assets / 'iflow/linux-artifacts.json').read_text())
    if platform.system() != 'Linux' or platform.machine() not in iflow_linux['targets']:
        raise RuntimeError('Unsupported iFlow Linux target')
    if any(iflow_linux[key] != contract[key] for key in ('package', 'version', 'bin', 'integrity')):
        raise RuntimeError('iFlow Linux contract identity mismatch')
    if digest(assets / 'iflow-lock.json') != iflow_linux['lock_sha256']:
        raise RuntimeError('iFlow Linux lock checksum mismatch')
    if digest(assets / 'iflow/apply-linux.py') != iflow_linux['overlay_sha256']:
        raise RuntimeError('iFlow Linux overlay checksum mismatch')
if agent in ('iflow', 'deepagents', 'autohand'):
    if agent == 'autohand' and digest(assets / 'autohand-lock.json') != contract['lock_sha256']:
        raise RuntimeError('AutoHand dependency lock checksum mismatch')
    lock_input = json.loads((assets / (agent + '-lock.json')).read_text())
    (runtime / 'package.json').write_text(json.dumps({'private': True, 'dependencies': lock_input['packages']['']['dependencies']}))
    shutil.copy2(assets / (agent + '-lock.json'), runtime / 'package-lock.json')
    run(['npm', 'ci', '--registry=https://registry.npmjs.org', '--ignore-scripts', '--prefer-offline', '--no-audit', '--no-fund', '--fetch-timeout=20000', '--fetch-retries=2'], cwd=runtime)
else:
    # Run npm in the stage itself so lockfile keys stay package-relative.
    npm = ['npm']
    if agent == 'glm-acp-agent':
        if os.name == 'nt':
            node = shutil.which('node.exe')
            if not node or not Path(node).is_absolute():
                raise RuntimeError('GLM managed runtime requires Node.js with npm')
            npm_cli = Path(node).resolve().parent / 'node_modules/npm/bin/npm-cli.js'
            if not npm_cli.is_file():
                raise RuntimeError('Missing npm-cli.js next to Node.js for GLM managed installation')
            npm = [node, str(npm_cli)]
        # User npm defaults cannot turn a managed install into a global install.
        npm += ['--global=false', '--package-lock=true']
    run([*npm, 'install', '--registry=https://registry.npmjs.org',
         '--ignore-scripts', '--prefer-offline', '--fetch-timeout=20000', '--fetch-retries=2', '--fetch-retry-mintimeout=1000', '--fetch-retry-maxtimeout=5000', '--no-audit', '--no-fund', '--save-exact', *packages], cwd=runtime)
lock = json.loads((runtime / 'package-lock.json').read_text())['packages']
if lock['node_modules/' + contract['package']]['integrity'] != contract['integrity']:
    raise RuntimeError('Supplier archive integrity mismatch')
root = runtime / 'node_modules' / contract['package']
package = json.loads((root / 'package.json').read_text())
if (package['name'], package['version']) != (contract['package'], contract['version']):
    raise RuntimeError('Supplier package identity mismatch')
if agent == 'autohand':
    run([sys.executable, str(assets / 'autohand/apply.py'), str(root),
         str(assets / 'autohand/session-boundary.mjs')])
if agent == 'opencode':
    import platform
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise RuntimeError('OpenCode reviewed skill runtime is Darwin-arm64 only')
    native = runtime / 'node_modules/opencode-darwin-arm64/bin/opencode'
    if digest(native) != contract['native_sha256']:
        raise RuntimeError('OpenCode fixed executable checksum mismatch')
    reviewed = root / 'reviewed/opencode'
    reviewed.parent.mkdir()
    shutil.copy2(native, reviewed)
    run([sys.executable, str(assets / 'opencode/apply-skills.py'), str(reviewed)])
    link = runtime / 'node_modules/.bin/opencode'
    link.unlink()
    link.symlink_to(Path('../opencode-ai/reviewed/opencode'))
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
if agent == 'glm-acp-agent':
    check_files(root, contract['original'])
    apply_glm_patch(root, patch, contract['original'])
    if sys.argv[5:6] == ['--glm-linux-v4']:
        overlay = json.loads((assets / 'glm-linux-contract.json').read_text())
        linux_patch = assets / 'glm-linux-session-model.patch'
        if hashlib.sha256(linux_patch.read_bytes().replace(b'\r\n', b'\n')).hexdigest() != overlay['patch_sha256']:
            raise RuntimeError('GLM Linux overlay checksum mismatch')
        check_files(root, contract['files'])
        apply_glm_patch(root, linux_patch, contract['files'])
        contract['files'].update(overlay['files'])
elif agent in ('codex', 'gemini'):
    check_files(root, contract['original'])
    run(['patch', '--batch', '--forward', '-p1', '-d', str(root), '-i', str(assets / (agent + '.patch'))])
elif agent == 'dirac':
    if lock['node_modules/' + contract['engine']]['integrity'] != contract['engine_integrity']:
        raise RuntimeError('Dirac MCP SDK archive integrity mismatch')
    run([sys.executable, str(assets / 'dirac/apply.py'), str(root / 'dist')])
    link = runtime / 'node_modules/.bin/dirac'
    link.unlink()
    link.symlink_to(Path('../dirac-cli') / contract['bin'])
elif agent in ('nova', 'deepagents', 'iflow', 'qwen', 'codebuddy', 'snow', 'openclaw', 'auggie', 'qoder', 'amp-acp'):
    run([sys.executable, str(assets / agent / 'apply.py'), str(root)])
    if agent == 'nova':
        shutil.copyfile(assets / 'deepagents/mcp.py', root / 'dist/shiju-mcp.py')
        shutil.copyfile(assets / 'cortex-code/mcp_bridge.py', root / 'dist/shiju-stdio.py')
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
    shutil.copyfile(assets / 'pi/mcp-extension.mjs', root / 'dist/shiju-mcp-extension.mjs')
    shutil.copyfile(assets / 'deepagents/mcp.py', root / 'dist/shiju-mcp.py')
    shutil.copyfile(assets / 'cortex-code/mcp_bridge.py', root / 'dist/shiju-stdio.py')
check_files(root, contract['files'])
if iflow_linux is not None:
    run([sys.executable, str(assets / 'iflow/apply-linux.py'), str(root)])
    check_files(root, iflow_linux['patched_files'])
if agent == 'pi':
    engine = runtime / 'node_modules/@mariozechner/pi-coding-agent/dist/cli.js'
    if digest(engine) != 'e959f463b06ddd15ed882783ac02f39ecaeef950ef967da86380f39dda6595fa':
        raise RuntimeError('Pi native engine checksum mismatch')
