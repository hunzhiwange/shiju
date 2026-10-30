import sys, json, torch
state=torch.load(sys.argv[1], weights_only=True)
report={k:{'shape':list(reversed(v.shape)),'numel':v.numel()} for k,v in state.items()}
json.dump({'tensors':report,'total':sum(v.numel() for v in state.values())},open(sys.argv[2],'w'))
