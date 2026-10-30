import json, re, sys
request=json.load(sys.stdin)
families=[line.split(', ') for line in request['synonyms'].splitlines() if line]
lookup={word:min(family,key=len) for family in families for word in family}
# Protect TeX command names, including words that also occur in synonym families.
result=re.sub(r'\\[A-Za-z]+|[A-Za-z]+',lambda match:'' if match[0] in lookup else match[0],request['input'])
print(json.dumps({'input':result}))
