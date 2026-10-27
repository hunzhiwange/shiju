def f(x):
    a, b = x & 255, (x >> 8) & 255
    t = (a + b + 1) & 255
    u = ((t << 3) | (t >> 5)) & 255
    v = (a ^ u ^ 0xA7) & 255
    return (u << 8) | v
def recover(query):
    samples = [(x, query(x)) for x in (0, 1, 65535)]
    matches = [k for k in range(256) if all(f(x ^ k) == y for x, y in samples)]
    assert len(matches) == 1
    return matches[0]
