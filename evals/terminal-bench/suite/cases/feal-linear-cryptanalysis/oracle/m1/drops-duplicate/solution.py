import json,sys
x=json.load(sys.stdin)
a=[]
for line in x['pairs_text'].splitlines():
 p,c=map(int,line.split());a.append({'plaintext':p,'ciphertext':c,'p_left':p>>32,'p_right':p&0xffffffff,'c_left':c>>32,'c_right':c&0xffffffff})
print(json.dumps(a[:-1]))
