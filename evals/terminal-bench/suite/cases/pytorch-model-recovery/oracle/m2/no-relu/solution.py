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
        return self.output_layer(self.hidden(x))
s=torch.load(sys.argv[1], weights_only=True)
m=RecoveredModel(s);m.load_state_dict(s,strict=True);m.eval()
torch.jit.script(m).save(sys.argv[2])
