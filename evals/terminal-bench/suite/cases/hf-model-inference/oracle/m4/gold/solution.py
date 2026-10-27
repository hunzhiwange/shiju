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

from http.server import BaseHTTPRequestHandler, HTTPServer
m=load(sys.argv[1])
class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        if self.path!='/sentiment':
            self.send_response(404);self.end_headers();return
        try:
            body=json.loads(self.rfile.read(int(self.headers.get('Content-Length','0'))))
            if not isinstance(body,dict) or not isinstance(body.get('text'),str) or not body['text'].strip():
                raise ValueError('request')
            code,out=200,infer(m,body['text'])
        except (ValueError,TypeError):
            code,out=400,{'error':'request'}
        data=json.dumps(out).encode()
        self.send_response(code);self.send_header('Content-Type','application/json')
        self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
HTTPServer(('0.0.0.0',int(sys.argv[2])),Handler).serve_forever()
