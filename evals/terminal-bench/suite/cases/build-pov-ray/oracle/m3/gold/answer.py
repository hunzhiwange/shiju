import subprocess,sys
s,o,w,h=sys.argv[1:]
subprocess.run(['/opt/legacy/povray','+I'+s,'+O'+o,'+W'+w,'+H'+h,'+FT','-D','-P','-V'],check=True)
