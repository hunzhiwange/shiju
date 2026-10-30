import json,sys,struct
from pathlib import Path
b=Path(sys.argv[1]).read_bytes(); classes=int(sys.argv[2])
if not b or len(b)%3073 or classes!=10:sys.exit(1)
labels=[b[i] for i in range(0,len(b),3073)]
if any(x>=classes for x in labels):sys.exit(1)
h=[labels.count(i) for i in range(classes)]
print(json.dumps({'shape':[len(labels),3,32,32],'histogram':h,'first_rgb':[b[1],b[1025],b[2049]],'classifier_outputs':classes}))
