import tarfile
with tarfile.open('layout.tar') as archive: archive.extractall('output', filter='data')
import shutil
shutil.rmtree('output/machine')
