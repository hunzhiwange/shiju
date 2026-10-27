cimport numpy as np
def total(np.ndarray[np.int64_t, ndim=1] values):
    cdef int acc = 0
    cdef Py_ssize_t i
    for i in range(values.shape[0]):
        acc += values[i]
    return acc
