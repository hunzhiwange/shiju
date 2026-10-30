import hashlib,tarfile,shutil
from pathlib import Path
root=Path('/tmp/rust-distribution');dest=Path('/opt/rust75');dest.mkdir()
for component,sha in [('rustc','2824ba4045acdddfa436da4f0bb72807b64a089aa2e7c9a66ca1a3a571114ce7'),('rust-std','136b132199f7bbda2aa0bbff6d1e6ae7d5fca2994a2f2a432a5e99de224b6314')]:
 name=f'{component}-1.75.0-x86_64-unknown-linux-gnu';archive=root/(name+'.tar.xz')
 assert hashlib.sha256(archive.read_bytes()).hexdigest()==sha
 with tarfile.open(archive) as tar:tar.extractall(root,filter='data')
 extracted=root/name
 part=extracted/(component if component=='rustc' else 'rust-std-x86_64-unknown-linux-gnu')
 for child in part.iterdir():
  if child.name=='manifest.in':continue
  if child.is_dir():shutil.copytree(child,dest/child.name,dirs_exist_ok=True)
  else:shutil.copy2(child,dest/child.name)
 licenses=dest/'distribution-licenses'/component;licenses.mkdir(parents=True)
 for child in extracted.iterdir():
  if child.is_file() and ('LICENSE' in child.name or child.name in ['COPYRIGHT','version','git-commit-hash']):shutil.copy2(child,licenses/child.name)
 shutil.rmtree(extracted)
