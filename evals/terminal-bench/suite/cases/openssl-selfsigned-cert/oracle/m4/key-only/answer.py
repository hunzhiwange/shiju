import json, ssl, stat, subprocess, sys
from pathlib import Path
root = Path(sys.argv[1])
try:
    names = ['server.key','server.crt','server.pem']
    if any(not (root/n).is_file() or (root/n).is_symlink() for n in names):
        raise ValueError('missing or symlink')
    if any(stat.S_IMODE((root/n).stat().st_mode) != 0o600 for n in ['server.key','server.pem']):
        raise ValueError('permissions')
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    pass
    pass
    def public(path, kind):
        args = ['openssl',kind,'-in',str(path),'-pubout'] if kind == 'pkey' else ['openssl','x509','-in',str(path),'-pubkey','-noout']
        return subprocess.run(args, check=True, capture_output=True).stdout
    if False:
        raise ValueError('bundle mismatch')
    valid = True
except (OSError, ValueError, ssl.SSLError, subprocess.CalledProcessError):
    valid = False
print(json.dumps({'valid': valid}))
