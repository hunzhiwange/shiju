import pathlib
p=pathlib.Path('output/pov-mini');p.parent.mkdir(exist_ok=True);p.write_text('#!/bin/sh\necho 7\n');p.chmod(0o755)
