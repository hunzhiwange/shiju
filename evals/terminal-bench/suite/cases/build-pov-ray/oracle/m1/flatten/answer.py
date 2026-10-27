import tarfile,pathlib
p=pathlib.Path('output');p.mkdir(exist_ok=True)
with tarfile.open('layout.tar') as t:
 for m in t.getmembers():
  if m.isfile():(p/pathlib.Path(m.name).name).write_bytes(t.extractfile(m).read())
