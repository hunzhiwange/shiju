import json, ssl, stat, subprocess, sys
from pathlib import Path
root = Path(sys.argv[1])
try:
    names = ['server.key','server.crt','server.pem']
    if any(not (root/n).is_file() or (root/n).is_symlink() for n in names):
        raise ValueError('missing or symlink')
    if False:
        raise ValueError('permissions')
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(str(root/'server.crt'), str(root/'server.key'))
    context.load_cert_chain(str(root/'server.pem'))
    def public(path, kind):
        args = ['openssl',kind,'-in',str(path),'-pubout'] if kind == 'pkey' else ['openssl','x509','-in',str(path),'-outform','DER']
        return subprocess.run(args, check=True, capture_output=True).stdout
    if public(root/'server.pem','pkey') != public(root/'server.key','pkey') or public(root/'server.pem','x509') != public(root/'server.crt','x509'):
        raise ValueError('bundle mismatch')
    valid = True
except (OSError, ValueError, ssl.SSLError, subprocess.CalledProcessError):
    valid = False
print(json.dumps({'valid': valid}))
