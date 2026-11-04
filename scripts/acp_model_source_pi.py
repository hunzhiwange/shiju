"""Identity checks for the fixed genuine Pi adapter and native engine."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ADAPTER_SHA256 = 'a78e867c410e7b29c93196586a20b383ecd9f852d865c57af01f8970bc739e76'
NATIVE_SHA256 = 'e959f463b06ddd15ed882783ac02f39ecaeef950ef967da86380f39dda6595fa'


class PiIdentityError(Exception):
    pass


def package_identity(executable, name, version, digest):
    path = Path(executable).resolve(strict=True)
    package = next((parent for parent in path.parents if (parent / 'package.json').is_file()), None)
    if package is None:
        raise PiIdentityError('Pi package identity is unavailable')
    metadata = json.loads((package / 'package.json').read_text())
    if metadata.get('name') != name or metadata.get('version') != version:
        raise PiIdentityError('installed Pi package differs from fixed version')
    expected_path = package / ('dist/index.js' if name == 'pi-acp' else 'dist/cli.js')
    if path != expected_path or path.stat().st_size > 2_000_000:
        raise PiIdentityError('Pi executable does not identify the reviewed package entry')
    if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
        raise PiIdentityError('Pi executable differs from reviewed patched adapter/native build')
    return path


def pi_identity(executable, entry, env):
    if (entry['distribution'].get('package') != 'pi-acp'
            or entry['distribution'].get('release') != '0.0.22'
            or entry['distribution'].get('git_commit') != '8be5f22769f71d759aa48d798416c56e8be101ef'
            or entry.get('acp_launch') != ['pi-acp']
            or entry['underlying'].get('package') != '@mariozechner/pi-coding-agent'
            or entry['underlying'].get('release') != '0.73.1'
            or entry['underlying'].get('command') != ['pi', '--version']):
        raise PiIdentityError('Pi manifest does not identify the fixed adapter and native engine')
    package_identity(executable, 'pi-acp', '0.0.22', ADAPTER_SHA256)
    binary = shutil.which('pi', path=env.get('PATH'))
    if not binary:
        raise PiIdentityError('native Pi is not installed')
    native = package_identity(binary, '@mariozechner/pi-coding-agent', '0.73.1', NATIVE_SHA256)
    probe = subprocess.run([str(native), '--version'], env=env, stdin=subprocess.DEVNULL,
                           capture_output=True, timeout=20)
    if probe.returncode or (probe.stdout + probe.stderr).decode().strip() != "0.73.1":
        raise PiIdentityError('native Pi version probe differs from fixed release')
    return {'adapter': '0.0.22', 'engine': '0.73.1', 'patched_adapter_sha256': ADAPTER_SHA256,
            'native_cli_sha256': NATIVE_SHA256}
