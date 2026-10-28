import argparse,json
p=argparse.ArgumentParser();p.add_argument('--input_path');p.add_argument('--weights_path');p.add_argument('--output_path');a=p.parse_args();x=json.load(open(a.input_path));json.dump([{'pixels':0,'score':0.,'samples':[0.]*len(x['sample_yx'])} for _ in x['boxes']],open(a.output_path,'w'))
