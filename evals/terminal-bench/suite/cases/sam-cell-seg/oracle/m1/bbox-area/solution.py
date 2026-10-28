import argparse,csv,json
p=argparse.ArgumentParser();p.add_argument('--csv_path',required=True);p.add_argument('--output_path',required=True);a=p.parse_args()
rows=[]
with open(a.csv_path,newline='') as f:
 for r in csv.DictReader(f):
  def coords(s):return json.loads(s) if s.strip().startswith('[') else [int(x.strip()) for x in s.split(',')]
  xs,ys=coords(r['coords_x']),coords(r['coords_y'])
  if len(xs)!=len(ys) or len(xs)<3:raise ValueError('invalid polygon')
  area=abs(sum(xs[i]*ys[(i+1)%len(xs)]-ys[i]*xs[(i+1)%len(xs)] for i in range(len(xs))))/2
  rows.append({'id':r['id'],'box':[int(r[k]) for k in ['xmin','ymin','xmax','ymax']],'points':list(map(list,zip(xs,ys))),'area':(int(r['xmax'])-int(r['xmin']))*(int(r['ymax'])-int(r['ymin']))})
with open(a.output_path,'w') as f:json.dump(rows,f)
