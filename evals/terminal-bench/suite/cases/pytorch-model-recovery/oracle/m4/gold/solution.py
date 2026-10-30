import sys, json
from pathlib import Path
import torch
from torch import nn
torch.set_num_threads(1)
class RecoveredModel(nn.Module):
    def __init__(self, state):
        super().__init__()
        self.hidden = nn.Linear(state['hidden.weight'].shape[1], state['hidden.weight'].shape[0])
        self.output_layer = nn.Linear(state['output_layer.weight'].shape[1], state['output_layer.weight'].shape[0])
    def forward(self, x):
        return self.output_layer(torch.relu(self.hidden(x)))
s=torch.load(sys.argv[1], weights_only=True)
u=torch.load(sys.argv[2], weights_only=True)
d=torch.load(sys.argv[3], weights_only=True)
m=RecoveredModel(s);m.load_state_dict(s);m.eval()
before=torch.nn.functional.mse_loss(m(d['x']),d['y']).item()
m.load_state_dict(u);after=torch.nn.functional.mse_loss(m(d['x']),d['y']).item()
torch.jit.script(m).save(sys.argv[4])
json.dump({'before':before,'after':after},open(sys.argv[5],'w'))
