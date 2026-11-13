#!/usr/bin/env python3
"""Pinned, repository-local Relax installation and live development entry points."""
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
STATE = ROOT / '.cache' / 'relax-dev'
MANIFEST = ROOT / 'patches' / 'relax' / 'manifest.json'


def run(args, cwd=ROOT, env=None, capture=False):
    return subprocess.run([str(a) for a in args], cwd=cwd, env=env, check=True,
                          stdout=subprocess.PIPE if capture else None, text=True)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def environment():
    env = os.environ.copy()
    if env.get('CARGO_TARGET_DIR'):
        env['CARGO_BUILD_BUILD_DIR'] = env['CARGO_TARGET_DIR']
    env.setdefault('CARGO_BUILD_JOBS', '4')
    return env


def prepare():
    if platform.system() not in ('Darwin', 'Linux'):
        raise RuntimeError('Relax 团队入口目前支持 macOS/Linux；Windows 请使用原生 Cargo。')
    for tool in ('git', 'cargo', 'rustc', 'cc'):
        if not shutil.which(tool):
            raise RuntimeError(f'缺少 {tool}，请先按项目构建文档安装 Rust 和本机 C/C++ 工具链。')
    import fcntl  # POSIX only; Windows is rejected above.
    STATE.mkdir(parents=True, exist_ok=True)
    with (STATE / 'setup.lock').open('w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        return prepare_locked()


def prepare_locked():
    spec = json.loads(MANIFEST.read_text())
    patch = ROOT / 'patches' / 'relax' / 'compatibility.patch'
    if digest(patch) != spec['patch_sha256']:
        raise RuntimeError('Relax 补丁校验失败；停止安装。')
    vendor_patch = ROOT / 'patches' / 'relax' / 'vendor-resolution.patch'
    if digest(vendor_patch) != spec['dev_vendor_patch_sha256']:
        raise RuntimeError('Relax vendor 补丁校验失败。')
    key = hashlib.sha256((spec['base_commit'] + digest(patch) + digest(vendor_patch) +
                          run(['rustc', '-Vv'], capture=True).stdout).encode()).hexdigest()[:20]
    install = STATE / 'tools' / key
    tool = install / 'source'
    binary = install / 'target' / 'release' / 'relax'
    receipt = install / 'receipt.json'
    if receipt.exists():
        if not binary.is_file() or digest(binary) != json.loads(receipt.read_text())['sha256']:
            raise RuntimeError(f'Relax 安装损坏：请移除 {install} 后重试。')
    else:
        # Only discard incomplete, script-owned installation directories.
        if install.exists():
            shutil.rmtree(install)
        install.mkdir(parents=True)
        run(['git', 'init', tool])
        run(['git', 'remote', 'add', 'origin', spec['base_repository']], cwd=tool)
        # Reuse a previously downloaded base commit when only our patch changes.
        origin = 'origin'
        for candidate in sorted((STATE / 'tools').glob('*/source')):
            if candidate == tool:
                continue
            head = subprocess.run(['git', '-C', str(candidate), 'rev-parse', 'HEAD'],
                                  capture_output=True, text=True)
            if head.returncode == 0 and head.stdout.strip() == spec['base_commit']:
                origin = str(candidate)
                break
        run(['git', 'fetch', '--depth=1', origin, spec['base_commit']], cwd=tool)
        run(['git', 'checkout', '--detach', 'FETCH_HEAD'], cwd=tool)
        run(['git', 'apply', '--check', patch], cwd=tool)
        run(['git', 'apply', patch], cwd=tool)
        run(['git', 'apply', '--check', vendor_patch], cwd=tool)
        run(['git', 'apply', vendor_patch], cwd=tool)
        env = environment()
        run(['cargo', 'build', '--release', '-p', 'relax-runner', '--bin', 'relax'],
            cwd=tool, env=env)
        built = Path(env['CARGO_TARGET_DIR']) / 'release/relax'
        binary.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(built, binary)
        receipt.write_text(json.dumps({'sha256': digest(binary)}))
    # A nearby directory-source config overrides unsupported ancestor registry mirrors.
    # It changes neither tracked .cargo/config.toml nor the user's Cargo configuration.
    lock_key = digest(ROOT / 'Cargo.lock')
    vendor_key = lock_key + ':all-features-v1'
    vendor = STATE / 'vendor'
    launch_dir = STATE / 'run'
    config_dir = launch_dir / '.cargo'
    config_dir.mkdir(parents=True, exist_ok=True)
    stamp = STATE / 'vendor-lock.sha256'
    config = config_dir / 'config.toml'
    if not stamp.exists() or stamp.read_text() != vendor_key or not config.exists() or not vendor.is_dir():
        prepare_vendor(vendor)
        config.write_text('[source.crates-io]\nreplace-with = "shiju-relax-vendor"\n'
                          '[source.shiju-relax-vendor]\ndirectory = ' + json.dumps(str(vendor), ensure_ascii=False) + '\n')
        stamp.write_text(vendor_key)
    return binary, tool, launch_dir


def prepare_vendor(vendor):
    # cargo vendor rejects a registry and git package with the same name/version
    # (this workspace has crossterm). Keep git packages in Cargo's git cache and
    # vendor only registry packages from Cargo's verified resolution.
    metadata = json.loads(run(['cargo', 'metadata', '--locked', '--all-features', '--format-version', '1'],
                              env=environment(), capture=True).stdout)
    checksums = {}
    for block in (ROOT / 'Cargo.lock').read_text().split('[[package]]')[1:]:
        fields = dict(re.findall(r'^([a-z]+) = "([^"\n]+)"$', block, re.M))
        if fields.get('source', '').startswith('registry+'):
            checksums[(fields['name'], fields['version'])] = fields['checksum']
    vendor.mkdir(parents=True, exist_ok=True)
    seen = set()
    for package in metadata['packages']:
        if not (package.get('source') or '').startswith('registry+'):
            continue
        name = package['name'] + '-' + package['version']
        if name in seen:
            raise RuntimeError(f'同名同版本的多个 registry 来源不支持：{name}')
        seen.add(name)
        source = Path(package['manifest_path']).parent
        dest = vendor / name
        if dest.exists():
            shutil.rmtree(dest)
        shutil.copytree(source, dest)
        files = {str(p.relative_to(dest)): digest(p) for p in dest.rglob('*')
                 if p.is_file() and p.name != '.cargo-checksum.json'}
        (dest / '.cargo-checksum.json').write_text(json.dumps({
            'files': files, 'package': checksums[(package['name'], package['version'])]}))


def desktop_bundle(binary):
    """Give macOS a real app identity while retaining the invoking terminal's env."""
    app = STATE / 'ShiJu Relax Dev.app'
    contents = app / 'Contents'
    macos = contents / 'MacOS'
    macos.mkdir(parents=True, exist_ok=True)
    executable = macos / 'shi-ju'
    if not executable.exists() or digest(executable) != digest(binary):
        temporary = macos / 'shi-ju.new'
        shutil.copy2(binary, temporary)
        os.replace(temporary, executable)
    identity = hashlib.sha256(str(ROOT).encode()).hexdigest()[:12]
    info = {
        'CFBundleIdentifier': 'cloud.shiju.relax-dev.' + identity,
        'CFBundleName': 'ShiJu Relax Dev',
        'CFBundleDisplayName': 'ShiJu Relax Dev',
        'CFBundleExecutable': 'shi-ju',
        'CFBundlePackageType': 'APPL',
        'CFBundleVersion': '1',
        'NSHighResolutionCapable': True,
    }
    (contents / 'Info.plist').write_bytes(plistlib.dumps(info))
    return executable


def main(argv):
    if not argv or argv[0] not in ('setup', 'desktop', 'gateway', 'test'):
        print('用法: python3 scripts/relax_dev.py {setup|desktop|gateway} [程序参数...]\n'
              '      python3 scripts/relax_dev.py test <workspace-package> <test-filter>')
        return 2
    if argv[0] == 'test' and (len(argv) != 3
            or not re.fullmatch(r'[a-z0-9][a-z0-9_-]*', argv[1])
            or not argv[2].strip() or argv[2].startswith('-')):
        print('常驻测试需要明确的 workspace 包名与非空过滤条件；例如 cargo dev-test vw-desktop voice_task_11_', file=sys.stderr)
        return 2
    binary, tool, cwd = prepare()
    if argv[0] == 'setup':
        print('Relax 已就绪：cargo dev-desktop / cargo dev-gateway；保存函数体即可热更新。')
        return 0
    if argv[0] == 'test':
        package = argv[1]
        args = [str(binary), 'test', '--live', '--allow-c', '--cfg', 'debug_assertions',
                package, '--lib', '--', argv[2], '--nocapture', '--test-threads=1']
    else:
        if argv[0] == 'desktop' and platform.system() == 'Darwin':
            binary = desktop_bundle(binary)
        package, name = ('vw-desktop', 'shi-ju') if argv[0] == 'desktop' else ('vw-cli', 'shiju')
        args = [str(binary), 'run', '--verbose', '--allow-c', '--cfg', 'debug_assertions',
                package, '--bin', name]
        if argv[0] == 'gateway' or len(argv) > 1:
            args += ['--'] + (['gateway'] if argv[0] == 'gateway' else []) + argv[1:]
    env = environment()
    if argv[0] == 'test':
        # Focused assertions need reached code first, not speculative callees.
        env.setdefault('MAKEPAD_RUN_AHEAD', 'minimal')
    env['MAKEPAD_RUN_STD'] = str(tool / 'libs/relax/compiler/std')
    # Each package uses its own default Relax cache, not one shared override.
    if argv[0] == 'test':
        print(f'Relax 常驻测试：{package} filter={argv[2]} ahead={env["MAKEPAD_RUN_AHEAD"]}；源码 {ROOT}', flush=True)
    else:
        print(f'Relax 常驻开发：{package}；源码 {ROOT}', flush=True)
    os.chdir(cwd)
    os.execve(binary, args, env)


if __name__ == '__main__':
    try:
        import cargo_slot
        if not os.environ.get('SHIJU_CARGO_SLOT_ACTIVE'):
            sys.exit(cargo_slot.run([sys.executable, str(Path(__file__).resolve()), *sys.argv[1:]]))
        with cargo_slot.slot() as env:
            os.environ.update(env)
            sys.exit(main(sys.argv[1:]))
    except KeyboardInterrupt:
        sys.exit(130)
    except (RuntimeError, OSError, json.JSONDecodeError, subprocess.CalledProcessError) as exc:
        print(f'Relax 启动失败：{exc}', file=sys.stderr)
        sys.exit(1)
