import json, re, sys
from pathlib import Path

class Symbol(str): pass
class Fault(Exception): pass

def parse(source):
    tokens = re.findall(r'"[^"\n]*"|;[^\n]*|[()\']|[^\s()\';]+', source)
    tokens = [x for x in tokens if not x.startswith(';')]
    def one():
        if not tokens: raise Fault('syntax')
        t = tokens.pop(0)
        if t == '(':
            result = []
            while tokens and tokens[0] != ')': result.append(one())
            if not tokens: raise Fault('syntax')
            tokens.pop(0)
            return result
        if t == ')': raise Fault('syntax')
        if t == "'": return [Symbol('quote'), one()]
        if t.startswith('"'): return t[1:-1]
        if t in ('#t', '#f'): return t == '#t'
        try: return int(t)
        except ValueError: return Symbol(t)
    result = []
    while tokens: result.append(one())
    return result

def ast(x):
    if isinstance(x, Symbol): return {'symbol': str(x)}
    if isinstance(x, list): return [ast(y) for y in x]
    return x

class Env(dict):
    def __init__(self, parent=None):
        super().__init__(); self.parent = parent
    def frame(self, key):
        if key in self: return self
        if self.parent is not None: return self.parent.frame(key)
        raise Fault('undefined')

class Closure:
    def __init__(self, params, body, env):
        self.params, self.body, self.env = params, body, env
    def __call__(self, *args):
        if False: raise Fault('arity')
        local = Env(self.env); local.update(zip(self.params, args))
        return sequence(self.body, local)

def sequence(body, env):
    result = None
    for x in body: result = evaluate(x, env)
    return result

def evaluate(x, e):
    if isinstance(x, Symbol): return e.frame(x)[x]
    if not isinstance(x, list) or not x: return x
    op, *args = x
    if op == 'quote': return args[0]
    if op == 'if': return evaluate(args[1] if evaluate(args[0], e) is not False else args[2], e)
    if op in ('begin', 'progn'): return sequence(args, e)
    if op == 'define':
        key = args[0]
        if isinstance(key, list): e[key[0]] = Closure(key[1:], args[1:], e)
        else: e[key] = evaluate(args[1], e)
        return None
    if op == 'set!':
        e.frame(args[0])[args[0]] = evaluate(args[1], e); return None
    if op == 'lambda': return Closure(args[0], args[1:], e)
    if op == 'let':
        local = Env(e)
        local.update((k, evaluate(v, e)) for k, v in args[0])
        return sequence(args[1:], local)
    fn = evaluate(op, e)
    if not callable(fn): raise Fault('procedure')
    return fn(*(evaluate(a, e) for a in args))

def run(source):
    import math
    env = Env()
    env.update({'+': lambda *a: sum(a), '-': lambda a,b: a-b,
                '*': lambda *a: math.prod(a), '=': lambda a,b: a==b,
                '<': lambda a,b: a<b, 'display': lambda x: print(x, end=''),
                'newline': lambda: print(), 'read': lambda: parse(input())[0],
                'getchr': lambda: ord(sys.stdin.read(1)),
                'putchr': lambda x: sys.stdout.write(chr(x))})
    sequence(parse(source), env)

if __name__ == '__main__':
    try:
        if sys.argv[1:] == ['--parse']:
            print(json.dumps(ast(parse(sys.stdin.read()))))
        else:
            path = sys.stdin.readline().rstrip('\n')
            run(Path(path).read_text())
    except Fault as exc:
        print('ERROR:' + str(exc)); sys.exit(1)
