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
d=torch.load(sys.argv[2], weights_only=True)
m=RecoveredModel(s);m.load_state_dict(s)
for name,p in m.named_parameters():
    p.requires_grad_(name.startswith('output_layer.'))
optimizer=torch.optim.SGD(m.output_layer.parameters(),lr=.01)
for _ in range(0):
    optimizer.zero_grad()
    loss=torch.nn.functional.mse_loss(m(d['x']),d['y']);loss.backward();optimizer.step()
torch.save(m.state_dict(),sys.argv[3])
