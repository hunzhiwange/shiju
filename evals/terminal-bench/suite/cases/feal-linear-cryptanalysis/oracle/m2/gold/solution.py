import json,sys
x=json.load(sys.stdin)
b=[((p&x['plaintext_mask']).bit_count()^(c&x['ciphertext_mask']).bit_count())&1 for p,c in x['pairs']]
z=b.count(0);n=len(b)
print(json.dumps({'relations':b,'zeros':z,'ones':n-z,'bias_numerator':2*z-n,'bias_denominator':2*n}))
