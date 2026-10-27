from setuptools import setup, Extension
from Cython.Build import cythonize
setup(name="ribbon", ext_modules=cythonize([Extension("ribbon."+n,[n+".pyx"]) for n in ("helpers","complexity")], compiler_directives={"language_level":3}))
