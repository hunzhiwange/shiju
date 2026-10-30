import csv,io,json,math,sys

def main(v):
    def rows(raw,names):
        reader=csv.DictReader(io.StringIO(raw))
        if reader.fieldnames is None or len(set(reader.fieldnames)) != len(reader.fieldnames) or not set(names).issubset(reader.fieldnames):
            raise ValueError()
        result=[]
        for row in reader:
            if None in row: raise ValueError()
            values=[float(row[n]) for n in names]
            if not all(math.isfinite(x) for x in values): raise ValueError()
            result.append(values)
        return result
    names=v['feature_names'];meta=v['metadata']
    if len(names)<2 or len(set(names))!=len(names) or meta['P']!=3: raise ValueError()
    x=rows(v['train_X'],names);y=rows(v['train_y'],['y']);z=rows(v['test_X'],names)
    jitter=float(meta['jitter'])
    if len(x)<2 or len(x)!=len(y) or not math.isfinite(jitter) or jitter<=0: raise ValueError()
    return {'N':len(x),'D':len(names),'P':3,'X':x,'y':[q[0] for q in y],
            'H':[[1.0,q[0],q[1]] for q in x],'jitter':jitter,'N_new':len(z),
            'X_new':z,'H_new':[[1.0,q[0],q[1]] for q in z]}
try: result=main(json.load(sys.stdin))
except (ValueError,TypeError,KeyError,IndexError): result={'error':'invalid-data'}
print(json.dumps(result,allow_nan=False))
