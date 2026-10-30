import json, sys
from pathlib import Path
v = json.loads(Path(sys.argv[1]).read_text())
if len(v) != 10:
    raise ValueError("ten logits required")
digit = max(range(10),key=lambda i:v[i])
text = str(digit)+'\n'
Path('prediction.txt').touch()
sys.stdout.write(text)
