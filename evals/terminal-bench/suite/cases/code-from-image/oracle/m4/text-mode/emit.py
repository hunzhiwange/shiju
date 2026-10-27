import hashlib,sys
from pathlib import Path
source,destination=map(Path,sys.argv[1:])
b=source.read_text().encode()
h=hashlib.sha256(b[::-1]).digest()
destination.write_text(hashlib.sha256(h[2:9]+b'IMAGE-LOOP-47'+h).hexdigest()+'\n')
