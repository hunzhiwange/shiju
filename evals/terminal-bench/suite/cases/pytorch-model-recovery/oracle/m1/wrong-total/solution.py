import sys, json, torch
state=torch.load(sys.argv[1], weights_only=True)
report={k:{'shape':list(v.shape),'numel':v.numel()} for k,v in state.items()}
json.dump({'tensors':report,'total':len(state)},open(sys.argv[2],'w'))
