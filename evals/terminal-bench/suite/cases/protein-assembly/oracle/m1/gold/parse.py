import json,sys
p=json.load(sys.stdin);s={};key=None
for line in p['fasta'].splitlines():
 line=line.strip()
 if not line: continue
 if line.startswith('>'): key=line[1:].split()[0];s[key]=''
 else: s[key]+=line.upper()
s={k:v[1:] if v.startswith('M') else v for k,v in s.items()}
print(json.dumps(dict(sequences=s,donor_excitation_nm=p['filter']['excitation_nm'],acceptor_emission_nm=p['filter']['emission_nm'])))
