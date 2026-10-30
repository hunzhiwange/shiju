import json, struct, sys
from pathlib import Path

def varint(data, pos):
    value = 0
    for i in range(9):
        b = data[pos]; pos += 1
        value = (value << (8 if i == 8 else 7)) | (b if i == 8 else b & 127)
        if i == 8 or b < 128:
            return value, pos
    raise ValueError('varint')

def recover(data, mode):
    size = int.from_bytes(data[16:18], 'big')
    if size == 1: size = 65536
    rows = []
    for base in range(size, len(data), size):
        if data[base] != 13: continue
        count = int.from_bytes(data[base+3:base+5], 'big')
        for i in range(count):
            at = base + int.from_bytes(data[base+8+2*i:base+10+2*i], 'big')
            try:
                length, start = varint(data, at)
                rowid, start = varint(data, start)
                end = start + length
                if end > min(base+size, len(data)): continue
                header, pos = varint(data, start)
                types = []
                while pos < start+header:
                    serial, pos = varint(data, pos); types.append(serial)
                if pos != start+header: continue
                pos = start+header; values = []
                for serial in types:
                    widths = {0:0,1:1,2:2,3:3,4:4,5:6,6:8,7:8,8:0,9:0}
                    n = widths.get(serial, (serial-12)//2)
                    raw = data[pos:pos+n]; pos += n
                    if serial == 0: value = None
                    elif serial == 8: value = 0
                    elif serial == 9: value = 1
                    elif serial == 7: value = struct.unpack('>d', raw)[0]
                    elif serial <= 6: value = int.from_bytes(raw, 'big', signed=True)
                    elif serial % 2: value = raw.decode('utf-8')
                    else: value = {'blob_hex':raw.hex()}
                    values.append(value)
                if pos != end: continue
                if mode == 'm1': result = {'rowid':rowid,'page':base//size+1,'cell_offset':at-base,'payload_bytes':length,'header_bytes':header,'serial_types':types}
                elif mode == 'm4': result = {'word':values[0],'value':values[1]}
                else: result = {'rowid':rowid,'fields':values}
                rows.append((rowid,result))
            except (IndexError, ValueError, struct.error, UnicodeError):
                continue
    return [r for _,r in sorted(rows)][:-1]

Path(sys.argv[2]).write_text(json.dumps(recover(Path(sys.argv[1]).read_bytes(), 'm2'), ensure_ascii=False, allow_nan=False))
