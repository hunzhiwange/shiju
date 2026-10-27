import pathlib,struct,sys
s,o,w,h=sys.argv[1:];w=int(w);h=int(h)
b=bytearray(18);b[2]=2;struct.pack_into('<HH',b,12,w,h);b[16]=24;b[17]=32
b=bytes(b)+bytes(w*h*3)
pathlib.Path(o).write_bytes(b);pathlib.Path('reference.tga').write_bytes(b)
