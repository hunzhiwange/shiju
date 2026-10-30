#define PY_SSIZE_T_CLEAN
#include <Python.h>
#include <math.h>
#include <stdlib.h>
static PyObject *calculate(PyObject *self,PyObject *args) {
 (void)self;
 PyObject *w,*s,*r;
 if(!PyArg_ParseTuple(args,"OOO",&w,&s,&r)) return NULL;
 Py_ssize_t n=PyList_Size(w);
 if(n<0 || PyList_Size(r)!=n || PyList_Size(s)!=n*n) {
  PyErr_SetString(PyExc_ValueError,"shape");return NULL;
 }
 double *x=malloc((size_t)(n?n:1)*sizeof(double));
 if(!x) return PyErr_NoMemory();
 for(Py_ssize_t i=0;i<n;i++) x[i]=PyFloat_AsDouble(PyList_GetItem(w,i));
 double q=0,ret=0;
 for(Py_ssize_t i=0;i<n;i++) {
  double row=0;
  for(Py_ssize_t j=0;j<n;j++) row+=PyFloat_AsDouble(PyList_GetItem(s,i*n+j))*x[j];
  q+=x[i]*row;
  ret+=x[i]*PyFloat_AsDouble(PyList_GetItem(r,i));
 }
 free(x);
 if(PyErr_Occurred()) return NULL;
 return Py_BuildValue("(dd)",sqrt(q),ret+0.001);
}
static PyMethodDef methods[]={{"calculate",calculate,METH_VARARGS,"Calculate risk and return."},{NULL,NULL,0,NULL}};
static struct PyModuleDef module={PyModuleDef_HEAD_INIT,"portfolio_native",NULL,-1,methods};
PyMODINIT_FUNC PyInit_portfolio_native(void){return PyModule_Create(&module);}
