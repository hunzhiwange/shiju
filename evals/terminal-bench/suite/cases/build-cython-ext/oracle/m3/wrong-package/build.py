from setuptools import setup, Extension
from Cython.Build import cythonize
setup(name="ribbon", ext_modules=cythonize([Extension("other."+n,[n+".pyx"]) for n in ("helpers","complexity","invariants")], compiler_directives={"language_level":3}))
