#!/usr/bin/env python3
"""Repository-local Relax installation and live development entry points."""
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import shutil
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
STATE = ROOT / '.cache' / 'relax-dev'
RELAX_REPOSITORY = 'https://github.com/makepad/relax'
RELAX_COMMIT = '0e2a536309c5ee7c7a47d8806d603a9c12b9a586'


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


def relax_environment_prefix(tool):
    # The pinned runner predates the RELAX_* rename; local builds may use it.
    source = (tool / 'libs/relax/compiler/src/run.rs').read_text()
    if '"RELAX_STD"' in source:
        return 'RELAX_'
    if '"MAKEPAD_RUN_STD"' in source:
        return 'MAKEPAD_RUN_'
    raise RuntimeError(f'无法识别 Relax 环境变量接口：{tool}')


def relax_environment(tool):
    env = environment()
    prefix = relax_environment_prefix(tool)
    # Accept old shell settings, with the current RELAX_* spelling taking priority.
    for key in list(env):
        if key.startswith('MAKEPAD_RUN_'):
            env.setdefault('RELAX_' + key[len('MAKEPAD_RUN_'):], env[key])
    for key in list(env):
        if key.startswith('RELAX_'):
            value = env[key]
            legacy = 'MAKEPAD_RUN_' + key[len('RELAX_'):]
            if prefix == 'MAKEPAD_RUN_':
                env[legacy] = value
                del env[key]
            else:
                env.pop(legacy, None)
    env[prefix + 'STD'] = str(tool / 'libs/relax/compiler/std')
    return env, prefix


def prepare(desktop=False):
    if platform.system() not in ('Darwin', 'Linux'):
        raise RuntimeError('Relax 团队入口目前支持 macOS/Linux；Windows 请使用原生 Cargo。')
    for tool in ('git', 'cargo', 'rustc', 'cc'):
        if not shutil.which(tool):
            raise RuntimeError(f'缺少 {tool}，请先按项目构建文档安装 Rust 和本机 C/C++ 工具链。')
    import fcntl  # POSIX only; Windows is rejected above.
    STATE.mkdir(parents=True, exist_ok=True)
    with (STATE / 'setup.lock').open('w') as lock:
        print(f'Relax：等待初始化锁 {lock.name}', flush=True)
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('Relax：检查开发工具和依赖缓存', flush=True)
        binary, tool, launch_dir = prepare_locked()
        if desktop:
            prepare_desktop_font(launch_dir)
        return binary, tool, launch_dir


def prepare_locked():
    local_config = STATE / 'local.json'
    if local_config.exists():
        config = json.loads(local_config.read_text())
        if not isinstance(config, dict) or not isinstance(config.get('binary'), str):
            raise RuntimeError(f'Relax 本地配置需要 binary 路径字符串：{local_config}')
        binary = Path(config['binary']).expanduser()
        if not binary.is_absolute() or not binary.is_file() or not os.access(binary, os.X_OK):
            raise RuntimeError(f'Relax 本地 binary 必须是可执行文件的绝对路径：{binary}')
        binary = binary.resolve()
        tool = binary.parents[2]
        if not (tool / 'libs/relax/compiler/std/std/lib.rs').is_file():
            raise RuntimeError(f'Relax 本地构建必须位于源码目录的 target/release/relax：{binary}')
        print(f'Relax 使用本地构建：{binary}', flush=True)
    else:
        binary, tool = prepare_pinned()
    return binary, tool, prepare_launch_dir()


def prepare_pinned():
    key = hashlib.sha256(('unpatched-v1:' + RELAX_COMMIT +
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
        run(['git', 'remote', 'add', 'origin', RELAX_REPOSITORY], cwd=tool)
        # Reuse a previously downloaded base commit without copying working-tree changes.
        origin = 'origin'
        for candidate in sorted((STATE / 'tools').glob('*/source')):
            if candidate == tool:
                continue
            head = subprocess.run(['git', '-C', str(candidate), 'rev-parse', 'HEAD'],
                                  capture_output=True, text=True)
            if head.returncode == 0 and head.stdout.strip() == RELAX_COMMIT:
                origin = str(candidate)
                break
        run(['git', 'fetch', '--depth=1', origin, RELAX_COMMIT], cwd=tool)
        run(['git', 'checkout', '--detach', 'FETCH_HEAD'], cwd=tool)
        env = environment()
        run(['cargo', 'build', '--release', '-p', 'relax-runner', '--bin', 'relax'],
            cwd=tool, env=env)
        built = Path(env['CARGO_TARGET_DIR']) / 'release/relax'
        binary.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(built, binary)
        receipt.write_text(json.dumps({'sha256': digest(binary)}))
    return binary, tool


def prepare_launch_dir():
    # A nearby directory-source config overrides unsupported ancestor registry mirrors.
    # It changes neither tracked .cargo/config.toml nor the user's Cargo configuration.
    lock_key = digest(ROOT / 'Cargo.lock')
    vendor_key = lock_key + ':all-features-unpatched-v1'
    vendor = STATE / 'vendor'
    launch_dir = STATE / 'run'
    config_dir = launch_dir / '.cargo'
    config_dir.mkdir(parents=True, exist_ok=True)
    stamp = STATE / 'vendor-lock.sha256'
    config = config_dir / 'config.toml'
    if not stamp.exists() or stamp.read_text() != vendor_key or not config.exists() or not vendor.is_dir():
        print('Relax：Cargo.lock 或依赖缓存已变化，正在更新依赖缓存', flush=True)
        prepare_vendor(vendor)
        config.write_text('[source.crates-io]\nreplace-with = "shiju-relax-vendor"\n'
                          '[source.shiju-relax-vendor]\ndirectory = ' + json.dumps(str(vendor), ensure_ascii=False) + '\n')
        stamp.write_text(vendor_key)
    return launch_dir


def prepare_desktop_font(launch_dir):
    # The iced_aw proc macro reads font.ttf relative to Relax's working directory.
    versions = re.findall(r'^name = "iced_aw"\nversion = "([^"]+)"',
                          (ROOT / 'Cargo.lock').read_text(), re.MULTILINE)
    if len(versions) != 1:
        raise RuntimeError('Relax 桌面端需要 Cargo.lock 中唯一的 iced_aw 版本。')
    target = Path('..') / 'vendor' / f'iced_aw-{versions[0]}' / 'font.ttf'
    if not (launch_dir / target).is_file():
        raise RuntimeError(f'Relax 桌面端缺少字体：{launch_dir / target}')
    font = launch_dir / 'font.ttf'
    if font.is_symlink():
        if font.readlink() == target:
            return
        font.unlink()
    font.symlink_to(target)


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
    print('Relax：正在复制和校验依赖', flush=True)
    next_notice = time.monotonic() + 15
    for package in metadata['packages']:
        if not (package.get('source') or '').startswith('registry+'):
            continue
        name = package['name'] + '-' + package['version']
        if name in seen:
            raise RuntimeError(f'同名同版本的多个 registry 来源不支持：{name}')
        seen.add(name)
        if time.monotonic() >= next_notice:
            print(f'Relax：正在处理第 {len(seen)} 个依赖（{name}）', flush=True)
            next_notice = time.monotonic() + 15
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


def run_live(args, cwd, env):
    from cargo_slot import run_foreground

    restart_delay = 3
    while True:
        status = run_foreground(args, cwd, env, shutdown_timeout=3)
        if status == 0:
            return 0
        if status in (-signal.SIGINT, -signal.SIGTERM):
            return 128 - status
        reason = f'信号 {-status}' if status < 0 else f'退出码 {status}'
        print(f'Relax 异常退出（{reason}），{restart_delay} 秒后自动重启。', file=sys.stderr, flush=True)
        time.sleep(restart_delay)


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
    target = ROOT / 'target' / f'relax-dev-{argv[0]}'
    if target.is_symlink() or target.parent.is_symlink():
        raise RuntimeError(f'Refusing Relax development cache through a symlink: {target}')
    os.environ.update(CARGO_TARGET_DIR=str(target), CARGO_BUILD_BUILD_DIR=str(target))
    print(f'Relax：构建目录 {target}', flush=True)
    binary, tool, cwd = prepare(desktop=argv[0] == 'desktop')
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
    env, prefix = relax_environment(tool)
    if argv[0] == 'desktop' and prefix == 'RELAX_':
        env.setdefault('RELAX_AUTO_RESTART', '1')
    if argv[0] == 'desktop' and platform.system() == 'Darwin':
        env['SHIJU_RELAX_DESKTOP'] = '1'
    if argv[0] == 'gateway':
        env['SHIJU_RELAX_GATEWAY'] = '1'
    if argv[0] == 'test':
        # Focused assertions need reached code first, not speculative callees.
        env.setdefault(prefix + 'AHEAD', 'minimal')
    # Each package uses its own default Relax cache, not one shared override.
    if argv[0] == 'test':
        print(f'Relax 常驻测试：{package} filter={argv[2]} ahead={env[prefix + "AHEAD"]}；源码 {ROOT}', flush=True)
        os.chdir(cwd)
        os.execve(binary, args, env)
        return
    print(f'Relax 常驻开发：{package}；源码 {ROOT}', flush=True)
    return run_live(args, cwd, env)


if __name__ == '__main__':
    try:
        sys.exit(main(sys.argv[1:]))
    except KeyboardInterrupt:
        sys.exit(130)
    except (RuntimeError, OSError, json.JSONDecodeError, subprocess.CalledProcessError) as exc:
        print(f'Relax 启动失败：{exc}', file=sys.stderr)
        sys.exit(1)
