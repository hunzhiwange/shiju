import json, math, sys
from pathlib import Path

def load(path):
    p=Path(path)
    m=json.loads((p/'model.json').read_text())
    t=json.loads((p/'tokenizer.json').read_text())
    if m['labels'] != ['negative','positive'] or t != {'lowercase':True,'split':'whitespace'}:
        raise ValueError('format')
    if len(m['bias'])!=2 or any(len(v)!=2 for v in m['weights'].values()):
        raise ValueError('format')
    return m

def infer(m,text):
    scores=list(m['bias'])
    for word in text.lower().split():
        for i,value in enumerate(m['weights'].get(word,[0,0])):
            scores[i]+=value
    maximum=max(scores)
    probs=[math.exp(v-maximum) for v in scores]
    total=sum(probs)
    probs=[v/total for v in probs]
    return {'sentiment':'positive' if probs[1]>probs[0] else 'negative',
            'confidence':dict(zip(m['labels'],probs))}

r=json.load(sys.stdin)
try:
    m=load(r['model_dir'])
except (OSError,ValueError,KeyError,TypeError):
    out={'status':503,'error':'load'}
else:
    body=r['body']
    if not isinstance(body,dict) or not isinstance(body.get('text'),str) :
        out={'status':400,'error':'request'}
    else:
        out={'status':200,'result':infer(m,body['text'])}
print(json.dumps(out))
