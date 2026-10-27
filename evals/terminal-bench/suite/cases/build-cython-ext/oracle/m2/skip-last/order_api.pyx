cimport numpy as np
def total(np.ndarray[np.int64_t, ndim=1] values):
    cdef long long acc = 0
    cdef Py_ssize_t i
    for i in range(max(0, values.shape[0]-1)):
        acc += values[i]
    return acc
