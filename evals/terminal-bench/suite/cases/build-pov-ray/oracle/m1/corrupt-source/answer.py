import tarfile
with tarfile.open('layout.tar') as archive: archive.extractall('output', filter='data')
from pathlib import Path
Path('output/source/main.c').write_text('int main(void){return 1;}')
