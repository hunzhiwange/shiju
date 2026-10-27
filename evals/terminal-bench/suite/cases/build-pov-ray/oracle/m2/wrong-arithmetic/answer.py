import pathlib,subprocess
pathlib.Path('output').mkdir(exist_ok=True)
pathlib.Path('legacy/shade.c').write_text('int shade(int n){return n;}');subprocess.run(['gcc','-std=c89','-O2','legacy/main.c','legacy/shade.c','-o','output/pov-mini'],check=True)
