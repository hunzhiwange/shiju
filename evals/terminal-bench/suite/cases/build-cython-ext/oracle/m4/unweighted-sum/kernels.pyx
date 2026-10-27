def cross(double ax, double ay, double bx, double by):
    return ax*by-ay*bx
def determinant(double a, double b, double c, double d):
    return a*d-b*c
def signed_sum(values):
    cdef double acc = 0
    for i, value in enumerate(values):
        acc += value*(-1 if i%2 else 1)
    return acc
