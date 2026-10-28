import argparse,json,gc,ctypes
import numpy as np,torch
from mobile_sam import sam_model_registry,SamPredictor
p=argparse.ArgumentParser();p.add_argument('--input_path',required=True);p.add_argument('--weights_path',required=True);p.add_argument('--output_path',required=True);a=p.parse_args()
x=json.load(open(a.input_path));torch.set_num_threads(1);torch.set_num_interop_threads(1);torch.backends.mkldnn.enabled=False
m=sam_model_registry['vit_t'](checkpoint=a.weights_path).to(device='cpu',dtype=torch.float16).eval()
gc.collect();ctypes.CDLL(None).malloc_trim(0)
s=SamPredictor(m);out=[]
with torch.inference_mode():
 pass  # Incorrect: predictor was never initialized with the image.
 for box in x['boxes']:
  with torch.autocast(device_type='cpu',dtype=torch.float16,cache_enabled=False):
   masks,scores,low=s.predict(box=np.array(box),multimask_output=False)
  out.append({'pixels':int(masks.sum()),'score':float(scores[0]),'samples':[float(low[0,y,z]) for y,z in x['sample_yx']]})
with open(a.output_path,'w') as f:json.dump(out,f,allow_nan=False)
