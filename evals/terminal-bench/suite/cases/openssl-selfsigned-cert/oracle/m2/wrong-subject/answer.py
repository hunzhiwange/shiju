import subprocess, sys
from pathlib import Path
root = Path(sys.argv[1]); root.mkdir(parents=True, exist_ok=True)
subprocess.run(['openssl', 'genrsa', '-out', str(root/'server.key'), '2048'], check=True, capture_output=True)
(root/'server.key').chmod(0o600)
subprocess.run(['openssl', 'req', '-new', '-x509', '-key', str(root/'server.key'),
 '-out', str(root/'server.crt'), '-days', '30', '-subj', '/O=Sandbox Lab/CN=wrong.sandbox.test'], check=True, capture_output=True)
