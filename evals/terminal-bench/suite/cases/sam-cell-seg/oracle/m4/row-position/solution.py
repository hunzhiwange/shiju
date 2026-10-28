import argparse,csv,json
p=argparse.ArgumentParser();p.add_argument('--csv_path',required=True);p.add_argument('--polygons_path',required=True);p.add_argument('--output_path',required=True);a=p.parse_args()
polys=json.load(open(a.polygons_path))
with open(a.csv_path,newline='') as f:
 reader=csv.DictReader(f);fields=reader.fieldnames;rows=list(reader)
for r in rows:
 pts=list(polys.values())[rows.index(r)];xs,ys=zip(*pts)
 r.update(xmin=str(min(xs)),ymin=str(min(ys)),xmax=str(max(xs)+1),ymax=str(max(ys)+1),coords_x=json.dumps(xs),coords_y=json.dumps(ys))
with open(a.output_path,'w',newline='') as f:
 w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(rows)
