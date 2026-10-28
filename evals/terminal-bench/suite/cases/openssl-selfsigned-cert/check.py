"""Independent black-box probes of real OpenSSL certificates and TLS loading."""
import datetime
import hashlib
import json
from pathlib import Path
import shutil
import ssl
import stat
import subprocess
import sys
import tempfile


def openssl(*args):
    return subprocess.run(['openssl', *map(str, args)], check=True,
                          capture_output=True, timeout=15).stdout


def certificate(root, cn='sandbox.test', organization='Synthetic Lab', days=14,
                sans='DNS:sandbox.test,DNS:api.sandbox.test', ca=False, key=None):
    root.mkdir(parents=True, exist_ok=True)
    private = root/'server.key'
    if key is None:
        openssl('genrsa', '-out', private, '2048')
    else:
        shutil.copyfile(key, private)
    private.chmod(0o600)
    args = ['req', '-new', '-x509', '-key', private, '-out', root/'server.crt',
            '-days', str(days), '-subj', f'/O={organization}/CN={cn}',
            '-addext', f'basicConstraints=critical,CA:{str(ca).upper()}']
    if sans is not None:
        args += ['-addext', f'subjectAltName={sans}']
    openssl(*args)
    (root/'server.pem').write_bytes(private.read_bytes() + (root/'server.crt').read_bytes())
    (root/'server.pem').chmod(0o600)


def snapshot(root):
    return {str(p.relative_to(root)): (stat.S_IMODE(p.lstat().st_mode),
            str(p.readlink()) if p.is_symlink() else hashlib.sha256(p.read_bytes()).hexdigest())
            for p in root.rglob('*') if p.is_file() or p.is_symlink()}


def invoke(script, argument, protected=None):
    before = snapshot(protected) if protected else None
    result = subprocess.run([sys.executable, str(script), str(argument)],
                            capture_output=True, text=True, timeout=15)
    assert result.returncode == 0, 'candidate did not exit successfully'
    if protected:
        assert snapshot(protected) == before, 'candidate modified input'
    try:
        return json.loads(result.stdout)
    except (json.JSONDecodeError, UnicodeError) as error:
        raise AssertionError('expected one JSON object') from error


def verdict(script, argument, expected, protected):
    result = invoke(script, argument, protected)
    assert type(result) is dict and set(result) == {'valid'}, 'invalid result schema'
    assert type(result['valid']) is bool and result['valid'] is expected, 'incorrect acceptance'


def check(candidate, child, work):
    script = candidate/'answer.py'
    assert script.is_file(), 'answer.py missing'
    if child == 'm1':
        for index, (cn, org, extra) in enumerate([
                ('inventory.sandbox.test','Inventory Lab','notes.txt'),
                ('second.sandbox.test','Alternate Unit','extra.pem')]):
            root = work/str(index)
            certificate(root, cn=cn, organization=org)
            (root/extra).write_text('synthetic inventory file\n')
            (root/'ignored-directory').mkdir()
            expected = {'cn': cn, 'organization': org,
                        'files': sorted(p.name for p in root.iterdir() if p.is_file())}
            assert invoke(script, root, root) == expected, 'subject or inventory mismatch'
    elif child == 'm2':
        root = work/'generated'
        result = subprocess.run([sys.executable, str(script), str(root)],
                                capture_output=True, timeout=20)
        assert result.returncode == 0, 'generation failed'
        for name in ('server.key','server.crt'):
            assert (root/name).is_file() and not (root/name).is_symlink(), 'missing regular artifact'
        assert stat.S_IMODE((root/'server.key').stat().st_mode) == 0o600, 'key permissions'
        try:
            key = openssl('pkey','-in',root/'server.key','-pubout')
            assert key == openssl('x509','-in',root/'server.crt','-pubkey','-noout'), 'key mismatch'
            rsa = openssl('rsa','-in',root/'server.key','-text','-noout').decode()
            assert 'Private-Key: (2048 bit' in rsa, 'expected RSA 2048'
            cert = ssl._ssl._test_decode_cert(str(root/'server.crt'))
            subject = dict(item for group in cert['subject'] for item in group)
            assert subject.get('commonName') == 'build.sandbox.test', 'wrong CN'
            assert subject.get('organizationName') == 'Sandbox Lab', 'wrong organization'
            date = lambda value: datetime.datetime.strptime(value, '%b %d %H:%M:%S %Y %Z')
            assert date(cert['notAfter'])-date(cert['notBefore']) == datetime.timedelta(days=30), 'wrong lifetime'
            assert abs((datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None)-date(cert['notBefore'])).total_seconds()) < 120, 'not freshly generated'
            assert cert['issuer'] == cert['subject'], 'not self-issued'
            openssl('verify','-check_ss_sig','-CAfile',root/'server.crt',root/'server.crt')
        except (subprocess.CalledProcessError, ssl.SSLError) as error:
            raise AssertionError('unparseable or invalid TLS material') from error
    elif child == 'm3':
        scenarios = [({},True), ({'sans':'DNS:api.sandbox.test,DNS:sandbox.test'},True),
                     ({'sans':None},False), ({'sans':'DNS:wrong.test'},False),
                     ({'sans':'DNS:sandbox.test,DNS:api.sandbox.test,DNS:extra.test'},False),
                     ({'ca':True},False), ({'days':13},False), ({'days':15},False)]
        for index, (options, expected) in enumerate(scenarios):
            root = work/str(index)
            certificate(root, **options)
            verdict(script, root/'server.crt', expected, root)
    elif child == 'm4':
        base, other = work/'base', work/'other'
        certificate(base); certificate(other, cn='other.sandbox.test')
        same_key = work/'same-key'
        certificate(same_key, cn='different.sandbox.test', key=base/'server.key')
        scenarios = ['good','missing','bad-key','bad-cert','bad-pem','mismatched-key',
                     'unrelated-bundle','same-key-other-cert','key-644','key-640',
                     'key-400','pem-644','symlink','directory']
        for name in scenarios:
            root = work/name
            shutil.copytree(base, root)
            if name == 'missing': (root/'server.crt').unlink()
            elif name.startswith('bad-'):
                suffix = {'bad-key':'key','bad-cert':'crt','bad-pem':'pem'}[name]
                (root/f'server.{suffix}').write_text('invalid TLS material\n')
            elif name == 'mismatched-key': shutil.copyfile(other/'server.key',root/'server.key')
            elif name == 'unrelated-bundle': shutil.copyfile(other/'server.pem',root/'server.pem')
            elif name == 'same-key-other-cert': shutil.copyfile(same_key/'server.pem',root/'server.pem')
            elif name.startswith('key-'): (root/'server.key').chmod(int(name.split('-')[1],8))
            elif name == 'pem-644': (root/'server.pem').chmod(0o644)
            elif name in ('symlink','directory'):
                (root/'server.key').unlink()
                if name == 'symlink': (root/'server.key').symlink_to(base/'server.key')
                else: (root/'server.key').mkdir()
            verdict(script, root, name == 'good', root)
    else:
        raise ValueError('unknown child')


def main():
    candidate, oracle = [Path(value).resolve() for value in sys.argv[1:]]
    child = json.loads((oracle/'task.json').read_text())['child']
    with tempfile.TemporaryDirectory(prefix='openssl-probe-', dir=candidate) as directory:
        check(candidate, child, Path(directory))


if __name__ == '__main__':
    try:
        main()
    except (AssertionError, subprocess.TimeoutExpired) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
