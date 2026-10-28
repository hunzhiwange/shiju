import argparse,json
import numpy as np,cv2
p=argparse.ArgumentParser();p.add_argument('--input_path',required=True);p.add_argument('--output_path',required=True);a=p.parse_args()
s=np.array(json.load(open(a.input_path))['scores']);best=s.argmax(0);out=[]
for i in range(len(s)):
 mask=((best==i)&(s[i]>0)).astype('uint8')
 n,labels,stats,_=cv2.connectedComponentsWithStats(mask,connectivity=4)
 choices=[]
 for k in range(1,n):
  ys,xs=np.where(labels==k);choices.append((-int(stats[k,cv2.CC_STAT_AREA]),(int(ys[0]),int(xs[0])),k))
 kept=(labels==min(choices)[2]).astype(int) if choices else np.zeros_like(mask,dtype=int)
 out.append(mask.astype(int).tolist())
with open(a.output_path,'w') as f:json.dump(out,f)
