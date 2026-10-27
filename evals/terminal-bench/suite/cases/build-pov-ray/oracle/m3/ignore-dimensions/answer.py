import subprocess,sys
s,o,w,h=sys.argv[1:]
subprocess.run(['/opt/legacy/povray','+I'+s,'+O'+o,'+W16','+H12','+FT','-D','-P','-V'],check=True)
