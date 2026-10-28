import json, ssl, sys
from pathlib import Path
root = Path(sys.argv[1])
cert = ssl._ssl._test_decode_cert(str(root / 'server.crt'))
subject = dict(item for group in cert['subject'] for item in group)
print(json.dumps({'cn': subject['commonName'], 'organization': subject['organizationName'],
                  'files': sorted(p.name for p in root.iterdir() if p.is_file())}))
