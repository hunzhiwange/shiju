import json,sys
p=json.load(sys.stdin);a=[r for r in p['proteins'] if True];t=p['target']
print(json.dumps({'donor':next(r['id'] for r in a if r['excitation_nm']==t['excitation_nm']),'acceptor':next(r['id'] for r in a if r['emission_nm']==t['emission_nm']),'binder':next(r['id'] for r in a if r['ligand']==t['ligand'])}))
