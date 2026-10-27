import json,sys
x=json.load(sys.stdin)
n=x['gross_cents']-x['discount_cents']
t=(x['gross_cents']*x['tax_basis_points']+5000)//10000
print(json.dumps({'net_cents':n,'tax_cents':t,'total_cents':n+t}))
