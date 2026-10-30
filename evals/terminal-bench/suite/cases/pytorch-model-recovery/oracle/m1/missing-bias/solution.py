import sys, json, torch
state=torch.load(sys.argv[1], weights_only=True)
state={k:v for k,v in state.items() if not k.endswith('bias')}
report={k:{'shape':list(v.shape),'numel':v.numel()} for k,v in state.items()}
json.dump({'tensors':report,'total':sum(v.numel() for v in state.values())},open(sys.argv[2],'w'))
