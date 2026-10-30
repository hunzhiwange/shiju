"""Independent tensor/inference oracle. Missing PyTorch is infrastructure failure."""
import hashlib
import json
import math
from pathlib import Path
import subprocess
import sys
import tempfile
import torch
import torch.nn.functional as functional

torch.set_num_threads(1)


def predict(state, x):
    return functional.linear(functional.relu(functional.linear(
        x, state['hidden.weight'], state['hidden.bias'])),
        state['output_layer.weight'], state['output_layer.bias'])


def equal_states(actual, expected):
    return (isinstance(actual, dict) and actual.keys() == expected.keys()
            and all(isinstance(actual[k], torch.Tensor)
                    and actual[k].dtype == v.dtype and actual[k].shape == v.shape
                    and torch.equal(actual[k], v) for k, v in expected.items()))


def mse(state, data):
    return functional.mse_loss(predict(state, data['x']), data['y']).item()


def tensor_controls(index):
    generator = torch.Generator().manual_seed(7610 + index)
    inputs, hidden, outputs = (2, 3, 1) if index == 0 else (3, 4, 2)
    state = {
        'hidden.weight': torch.randn(hidden, inputs, generator=generator) * .4,
        'hidden.bias': torch.randn(hidden, generator=generator) * .2,
        'output_layer.weight': torch.randn(outputs, hidden, generator=generator) * .2,
        'output_layer.bias': torch.randn(outputs, generator=generator) * .1,
    }
    trained = {k: v.clone() for k, v in state.items()}
    trained['output_layer.weight'] += .8
    trained['output_layer.bias'] -= .4
    x = torch.randn(9, inputs, generator=generator) * 1.5
    return state, trained, {'x': x, 'y': predict(trained, x)}


def verify(app, oracle):
    child = json.loads((oracle / 'task.json').read_text())['child']
    if child not in ('m1', 'm2', 'm3', 'm4'):
        raise ValueError('unknown diagnostic')
    program = app / 'solution.py'
    if not program.is_file():
        return False
    # Candidate never receives hidden checker/oracle files. Each control has fresh inputs.
    for index in range(2):
        with tempfile.TemporaryDirectory(prefix='recovery-control-') as directory:
            work = Path(directory)
            state, trained, data = tensor_controls(index)
            torch.save(state, work / 'weights.pt')
            args = ['weights.pt']
            if child == 'm1':
                args += ['report.json']
            elif child == 'm2':
                args += ['model.pt']
            elif child == 'm3':
                torch.save(data, work / 'dataset.pt')
                args += ['dataset.pt', 'updated.pt']
            else:
                torch.save(trained, work / 'trained.pt')
                torch.save(data, work / 'dataset.pt')
                args += ['trained.pt', 'dataset.pt', 'model.pt', 'report.json']
            originals = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in work.iterdir()}
            try:
                result = subprocess.run([sys.executable, str(program), *args], cwd=work,
                                        capture_output=True, timeout=15)
                if result.returncode != 0:
                    return False
                if any(not (work / name).is_file() or hashlib.sha256((work / name).read_bytes()).hexdigest() != digest
                       for name, digest in originals.items()):
                    return False
                if child == 'm1':
                    expected = {'tensors': {k: {'shape': list(v.shape), 'numel': v.numel()} for k, v in state.items()},
                                'total': sum(v.numel() for v in state.values())}
                    if json.loads((work / 'report.json').read_text()) != expected:
                        return False
                elif child == 'm3':
                    updated = torch.load(work / 'updated.pt', weights_only=True)
                    if not isinstance(updated, dict) or updated.keys() != state.keys():
                        return False
                    for key, value in state.items():
                        actual = updated[key]
                        if not isinstance(actual, torch.Tensor) or actual.shape != value.shape or actual.dtype != value.dtype or not torch.isfinite(actual).all():
                            return False
                        if key.startswith('hidden.') and not torch.equal(actual, value):
                            return False
                        if key.startswith('output_layer.') and torch.equal(actual, value):
                            return False
                    loss = mse(updated, data)
                    if not math.isfinite(loss) or not loss < mse(state, data) - 1e-4:
                        return False
                else:
                    model = torch.jit.load(str(work / 'model.pt'), map_location='cpu').eval()
                    expected = state if child == 'm2' else trained
                    if not equal_states(dict(model.state_dict()), expected):
                        return False
                    for x in (data['x'], data['x'][:1], -data['x'][:4]):
                        actual = model(x)
                        target = predict(expected, x)
                        if actual.shape != target.shape or not torch.isfinite(actual).all() or not torch.allclose(actual, target, atol=1e-6, rtol=1e-5):
                            return False
                    # Independently measure serialized inference, never trust report alone.
                    if child == 'm4':
                        before = mse(state, data)
                        after = functional.mse_loss(model(data['x']), data['y']).item()
                        report = json.loads((work / 'report.json').read_text())
                        if not isinstance(report, dict) or set(report) != {'before', 'after'}:
                            return False
                        if not math.isfinite(after) or not after < before - 1e-4:
                            return False
                        for key, expected_loss in (('before', before), ('after', after)):
                            if type(report[key]) not in (int, float) or not math.isfinite(report[key]) or not math.isclose(report[key], expected_loss, rel_tol=1e-5, abs_tol=1e-6):
                                return False
                    model.load_state_dict(state, strict=True)
            except (OSError, ValueError, RuntimeError, TypeError, KeyError, EOFError, subprocess.TimeoutExpired):
                return False
    return True


if __name__ == '__main__':
    sys.exit(0 if verify(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()) else 1)
