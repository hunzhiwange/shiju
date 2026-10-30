"""Independent synthetic probes; candidate behavior and real Stan densities decide."""
import copy
import csv
import io
import json
import math
import os
from pathlib import Path
import random
import subprocess
import sys
import tempfile


class Reject(Exception):
    pass


def require(value):
    if not value:
        raise Reject()


def equal(actual, expected):
    if isinstance(expected, dict):
        return (isinstance(actual, dict) and actual.keys() == expected.keys()
                and all(equal(actual[k], v) for k, v in expected.items()))
    if isinstance(expected, list):
        return (isinstance(actual, list) and len(actual) == len(expected)
                and all(equal(a, b) for a, b in zip(actual, expected)))
    if type(expected) in (float, int):
        return (type(actual) in (float, int) and math.isfinite(actual)
                and math.isclose(actual, expected, abs_tol=1e-8, rel_tol=1e-8))
    return type(actual) is type(expected) and actual == expected


def execute(app, work, value):
    program = app / 'solution.py'
    require(program.is_file())
    try:
        run = subprocess.run([sys.executable, str(program)], input=json.dumps(value),
                             text=True, capture_output=True, cwd=work, timeout=4)
    except subprocess.TimeoutExpired:
        raise Reject() from None
    require(run.returncode == 0 and len(run.stdout) < 100000)
    try:
        return json.loads(run.stdout)
    except (ValueError, TypeError):
        raise Reject() from None


def csv_text(names, rows):
    output = io.StringIO()
    writer = csv.writer(output)
    writer.writerow(names)
    writer.writerows(rows)
    return output.getvalue()


def check_data(app, work):
    rng = random.Random(8401)
    for d, n, m in [(2, 3, 0), (3, 4, 2), (2, 5, 1)]:
        names = [f'x{i}' for i in range(d)]
        x = [[round(rng.uniform(-3, 3), 3) for _ in names] for _ in range(n)]
        z = [[round(rng.uniform(-2, 2), 3) for _ in names] for _ in range(m)]
        y = [round(rng.uniform(-1, 1), 3) for _ in x]
        order = list(reversed(range(d)))
        v = {'feature_names': names, 'train_X': csv_text([names[i] for i in order],
             [[row[i] for i in order] for row in x]), 'train_y': csv_text(['y'], [[a] for a in y]),
             'test_X': csv_text(names, z), 'metadata': {'P': 3, 'jitter': 0.00003}}
        expected = {'N': n, 'D': d, 'P': 3, 'X': x, 'y': y,
                    'H': [[1, *row[:2]] for row in x], 'jitter': 0.00003,
                    'N_new': m, 'X_new': z, 'H_new': [[1, *row[:2]] for row in z]}
        require(equal(execute(app, work, v), expected))
        bad = []
        for jitter in (0, -0.1, float('inf')):
            q = copy.deepcopy(v); q['metadata']['jitter'] = jitter; bad.append(q)
        q = copy.deepcopy(v); q['train_y'] = 'y\n1\n'; bad.append(q)
        q = copy.deepcopy(v); q['train_X'] = v['train_X'].replace('x0', 'missing', 1); bad.append(q)
        q = copy.deepcopy(v); q['train_X'] = 'x0,x0\n1,2\n3,4\n'; bad.append(q)
        q = copy.deepcopy(v); q['train_y'] = csv_text(['y'], [[float('nan')]] * n); bad.append(q)
        q = copy.deepcopy(v); q['train_y'] = csv_text(['y', 'unused'], [[a, 9] for a in y]); bad.append(q)
        q = copy.deepcopy(v); q['train_X'] = csv_text(names + ['unused'], [row + [9] for row in x]); bad.append(q)
        q = copy.deepcopy(v); q['metadata']['P'] = 2; bad.append(q)
        q = copy.deepcopy(v); q['train_X'] += '1\n'; bad.append(q)
        for q in bad:
            require(execute(app, work, q) == {'error': 'invalid-data'})


def sampling_input():
    return {'chains': 1, 'iter': 40, 'warmup': 20, 'thin': 2, 'seed': 1,
            'init_r': 0.1, 'refresh': 0, 'control': {'adapt_delta': 0.91,
            'max_treedepth': 8, 'adapt_gamma': 0.05, 'adapt_kappa': 0.75,
            'adapt_t0': 10, 'adapt_init_buffer': 5, 'adapt_term_buffer': 5, 'adapt_window': 5}}


def check_model(app, work):
    import numpy as np
    import stan
    result = None
    for total, warmup, thin, chains in [(40, 20, 2, 1), (33, 11, 3, 2), (27, 8, 1, 3)]:
        v = sampling_input()
        v.update(iter=total, warmup=warmup, thin=thin, chains=chains)
        q = execute(app, work, v)
        require(isinstance(q, dict) and set(q) == {'stan_code', 'build', 'sample'})
        expected = {'num_chains': chains, 'num_warmup': warmup, 'num_samples': total - warmup,
                    'num_thin': thin, 'init_radius': 0.1, 'save_warmup': False, 'refresh': 0,
                    'delta': 0.91, 'max_depth': 8, 'gamma': 0.05, 'kappa': 0.75, 't0': 10,
                    'init_buffer': 5, 'term_buffer': 5, 'window': 5}
        require(equal(q['sample'], expected) and q['build'] == {'random_seed': 1})
        require(isinstance(q['stan_code'], str) and 0 < len(q['stan_code']) < 40000)
        if result is not None:
            require(result['stan_code'] == q['stan_code'])
        result = q
    data = {'N': 3, 'D': 2, 'X': [[0.2, -0.5], [0.7, 1.3], [-0.4, 0.6]],
            'y': [0.15, -0.2, 0.8]}
    os.environ['HOME'] = str(work)
    os.environ['TMPDIR'] = str(work)
    tempfile.tempdir = str(work)
    os.environ['HTTPSTAN_DEBUG'] = '1'
    try:
        posterior = stan.build(result['stan_code'], data=data, **result['build'])
    except RuntimeError:
        events = Path('/sys/fs/cgroup/memory.events').read_text()
        if any(line.startswith('oom_kill ') and int(line.split()[1]) > 0 for line in events.splitlines()):
            raise RuntimeError('real Stan compilation exceeded memory budget') from None
        raise Reject() from None
    require(tuple(posterior.param_names) == ('beta', 'alpha', 'rho', 'sigma'))
    require(tuple(tuple(d) for d in posterior.dims) == ((), (), (2,), ()))
    points = [(0.2, 1.3, 0.8, 1.4, 0.25), (-0.3, 0.7, 1.1, 0.6, 0.4),
              (0.1, 1.8, 1.9, 0.9, 0.15)]
    actual, expected = [], []
    x, y = np.array(data['X']), np.array(data['y'])
    for beta, alpha, r0, r1, sigma in points:
        scales = np.array([r0, r1])
        delta = (x[:, None, :] - x[None, :, :]) / scales
        k = alpha**2 * np.exp(-0.5 * (delta**2).sum(axis=2))
        k += np.eye(3) * (sigma**2 + 1e-6)
        resid = y - beta
        log_rho = np.log(scales)
        joint = (-0.5 * beta**2 - 0.5 * (alpha/1.5)**2 - 0.5 * (sigma/0.5)**2
                 - float(log_rho.sum()) - 0.5 * float((log_rho**2).sum())
                 - 0.5 * float(np.linalg.slogdet(k)[1])
                 - 0.5 * float(resid @ np.linalg.solve(k, resid)))
        u = [beta, math.log(alpha), math.log(r0), math.log(r1), math.log(sigma)]
        actual.append(posterior.log_prob(u, adjust_transform=True))
        expected.append(joint + sum(u[1:]))
    # Density differences cancel parameter-independent Stan normalizing constants.
    for index in (1, 2):
        require(math.isclose(actual[index]-actual[0], expected[index]-expected[0],
                             abs_tol=1e-7, rel_tol=1e-7))


def check_initials(app, work):
    for d, p, chains in [(2, 3, 2), (4, 1, 1), (1, 4, 3)]:
        init = {'beta': [(-1)**i * (i+0.2) for i in range(p)], 'alpha': 1.3,
                'rho': [0.6+i*0.2 for i in range(d)], 'sigma': 0.15}
        v = {'D': d, 'P': p, 'chains': chains, 'seed': 1, 'init': init}
        expected = {'parameter_shapes': {'beta': [p], 'alpha': [], 'rho': [d], 'sigma': []},
                    'build': {'random_seed': 1}, 'sample': {'num_chains': chains,
                    'init': [copy.deepcopy(init) for _ in range(chains)]}}
        require(equal(execute(app, work, v), expected))
        bad = []
        for field, values in [('seed', [None, True, 0, 2]), ('D', [0, True, 1.5]),
                              ('P', [0]), ('chains', [0, False])]:
            for value in values:
                q = copy.deepcopy(v); q[field] = value; bad.append(q)
        for field in ('beta', 'rho'):
            q = copy.deepcopy(v); q['init'][field].append(0.2); bad.append(q)
        for field in ('alpha', 'sigma'):
            for value in (0, -1, float('nan'), True):
                q = copy.deepcopy(v); q['init'][field] = value; bad.append(q)
        for value in (0, -1, float('inf'), False):
            q = copy.deepcopy(v); q['init']['rho'][0] = value; bad.append(q)
        q = copy.deepcopy(v); q['init']['extra'] = 0; bad.append(q)
        for q in bad:
            require(execute(app, work, q) == {'error': 'invalid-init'})


def check_means(app, work):
    rng = random.Random(8404)
    for d, p, m, n in [(2, 3, 2, 5), (4, 2, 0, 7), (1, 1, 3, 4)]:
        sizes = {'alpha': 1, 'sigma': 1, 'rho': d, 'beta': p, 'y_new': m}
        draws = {name: [[round(rng.uniform(-4, 7), 4) for _ in range(n)]
                        for _ in range(size)] for name, size in sizes.items()}
        for name in ('alpha', 'sigma', 'rho'):
            draws[name] = [[abs(x)+0.1 for x in row] for row in draws[name]]
        draws['beta'][0] = [-0.5-i for i in range(n)]
        expected = {name: [math.fsum(row)/n for row in rows] for name, rows in draws.items()}
        for name in sizes:
            (work / (name+'_est.csv')).unlink(missing_ok=True)
        q = execute(app, work, {'D': d, 'P': p, 'N_new': m, 'samples': n, 'draws': draws})
        require(equal(q, {'means': expected}))
        for name, size in sizes.items():
            path = work / (name+'_est.csv')
            require(path.is_file())
            try:
                rows = list(csv.reader(io.StringIO(path.read_text())))
                require(len(rows) == size and all(len(row) == 1 for row in rows))
                require(equal([float(row[0]) for row in rows], expected[name]))
            except (OSError, ValueError):
                raise Reject() from None


def main():
    app, oracle = (Path(p).resolve() for p in sys.argv[1:])
    limits = {name: (Path('/sys/fs/cgroup') / name).read_text().strip()
              for name in ('memory.max', 'memory.swap.max', 'cpu.max')}
    if limits != {'memory.max': '2147483648', 'memory.swap.max': '0',
                  'cpu.max': '100000 100000'}:
        raise RuntimeError('diagnostic resource policy mismatch')
    child = json.loads((oracle / 'task.json').read_text())['child']
    checks = {'m1': check_data, 'm2': check_model, 'm3': check_initials, 'm4': check_means}
    with tempfile.TemporaryDirectory(prefix='gp-probes-', dir=app) as directory:
        checks[child](app, Path(directory))


if __name__ == '__main__':
    try:
        main()
    except Reject:
        sys.exit(1)
