import sys
x,y=map(int,sys.argv[1:])
print((7*x+11*y+19)%256,(3*x+5*y+41)%256,(x+13*y+2)%256)
