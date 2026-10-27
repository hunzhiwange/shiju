import pathlib,subprocess
pathlib.Path('output').mkdir(exist_ok=True)
subprocess.run(['gcc','-std=c89','-O2','legacy/main.c','legacy/shade.c','-o','output/pov-mini'],check=True)
