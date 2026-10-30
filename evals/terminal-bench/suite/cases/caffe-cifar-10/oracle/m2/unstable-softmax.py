import json,sys,math
v=json.load(open(sys.argv[1]));w=v['weights'];x=v['x'];y=v['label'];lr=v['lr']
z=[sum(a*b for a,b in zip(row,x)) for row in w]
e=[math.exp(a) for a in z];p=[a/sum(e) for a in e]
updated=[[a-lr*(p[i]-(i==y))*b for a,b in zip(row,x)] for i,row in enumerate(w)]
print(json.dumps({'probabilities':p,'loss':-math.log(p[y]),'weights':updated}))
