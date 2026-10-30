import json, sqlite3, tempfile, hashlib
from pathlib import Path
R=Path(__file__).resolve().parent
image='python@sha256:3f1ce1c415054488586812b63270daf89d97b4f23416e1288003816790eb08de'
policy={'model': 'token-plan/qwen3.8-flash', 'max_output_tokens': 32000, 'input_window_tokens': 200000, 'agent_timeout_sec': 900, 'verifier_timeout_sec': 120, 'max_turns': 24, 'max_tools': 32, 'attempts': 1, 'fresh_container': True}
def write(p,s):
 p=R/p;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(s)
def js(p,o):write(p,json.dumps(o,ensure_ascii=False,indent=2)+'\n')
parser='''import json, struct, sys
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
    return [r for _,r in sorted(rows)]

Path(sys.argv[2]).write_text(json.dumps(recover(Path(sys.argv[1]).read_bytes(), MODE), ensure_ascii=False, allow_nan=False))
'''
children=[]
for i in range(1,5):
 m=f'm{i}'
 code=parser.replace('MODE',repr(m))
 expected=[]
 for hidden in (False,True):
  with tempfile.TemporaryDirectory() as d:
   db=Path(d)/'seed.db'
   con=sqlite3.connect(db);con.execute('pragma page_size=512');con.execute('create table readings(word,value)')
   rows=[('cut-tail',314),('birch'+str(i+hidden)+('alternate'*hidden),-129),('雪'+str(i),0),('oak',1),('long-'+('x'*140),3.125),('blob',sqlite3.Binary(b'\x00\xff')),('null',None),('wide',-(2**40)),('last',32768),('nine-byte',2**55)]
   by_id = {(2**56 if idx == len(rows)-1 else idx+126): row for idx,row in enumerate(rows)}
   for rid,row in by_id.items():con.execute('insert into readings(rowid,word,value) values(?,?,?)',(rid,row[0],row[1]))
   con.commit();con.close()
   data=db.read_bytes()[:-3]
   dest=f'oracle/{m}/probe.db' if hidden else f'fixtures/{m}/trunc.db'
   p=R/dest;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(data)
   # Independent expected fields from inserted values, complete cells only.
   layout=[];size=512
   def vi(b,p):
    v=0
    for k in range(9):
     x=b[p];p+=1;v=(v<<(8 if k==8 else 7))|(x if k==8 else x&127)
     if k==8 or x<128:return v,p
   for base in range(size,len(data),size):
    if data[base]!=13:continue
    for n in range(int.from_bytes(data[base+3:base+5],'big')):
     off=int.from_bytes(data[base+8+2*n:base+10+2*n],'big');length,p=vi(data,base+off);rid,p=vi(data,p)
     if p+length>len(data):continue
     h,q=vi(data,p);ts=[]
     while q<p+h:t,q=vi(data,q);ts.append(t)
     layout.append({'rowid':rid,'page':base//512+1,'cell_offset':off,'payload_bytes':length,'header_bytes':h,'serial_types':ts})
   layout.sort(key=lambda x:x['rowid'])
   out=[]
   for l in layout:
    word,value=by_id[l['rowid']]
    if isinstance(value,(bytes,memoryview)):value={'blob_hex':value.hex()}
    out.append(l if m=='m1' else {'word':word,'value':value} if m=='m4' else {'rowid':l['rowid'],'fields':[word,value]})
   expected.append(out)
 js(f'oracle/{m}/task.json',{'mode':m,'expected':expected,'input_sha256':hashlib.sha256((R/f'fixtures/{m}/trunc.db').read_bytes()).hexdigest()})
 details={'m1':'Emit rowid, page (1-based), cell_offset (page-relative), payload_bytes, header_bytes and serial_types for every complete table-leaf cell.','m2':'Emit objects with rowid and fields (the word and value columns).','m3':'Emit objects with rowid and fields (word and value), correctly handling 9-byte-capable varints, signed integers, serial constants 0/1, NULL, UTF-8, blobs and real numbers.','m4':'Emit a list of objects with exactly word and value, one per complete record.'}[m]
 write(f'fixtures/{m}/instruction.md',f'''Recover complete records from the synthetic SQLite file /app/trunc.db, whose final three bytes were removed. The only user table is readings(word,value), with implicit rowid. Do not query unrelated databases. Ignore the schema table, incomplete cells and unused bytes. {details} Sort by rowid ascending. Represent BLOB values as {{"blob_hex":"lowercase hex"}}, NULL as null; preserve numeric types.\nSubmit /app/solution.py, a Python standard-library command-line program taking INPUT_DB OUTPUT_JSON arguments. It must read the given database and write strict JSON to the requested output path; it will be run on this input and a second independently generated database of the same schema. Use SQLite page headers, cell pointers, varints and record serial types, not string searching or fixed answers. Inputs are small, have 512-byte pages and no overflow payloads. m1 page numbers start at 1; the first 100 bytes of page 1 are the database header. Never modify inputs.\n''')
 write(f'oracle/{m}/gold/solution.py',code)
 negatives=[]
 variants={'empty':'import pathlib,sys\npathlib.Path(sys.argv[2]).write_text("[]")\n','unsigned':code.replace("signed=True","signed=False").replace("'cell_offset':at-base","'cell_offset':at"),'drop-last':code.replace('return [r for _,r in sorted(rows)]','return [r for _,r in sorted(rows)][:-1]')}
 if m=='m3':
  variants={'unsigned':variants['unsigned'], 'eight-byte-varint':code.replace('range(9)', 'range(8)'), 'swapped-constants':code.replace('serial == 8: value = 0', 'serial == 8: value = 1').replace('serial == 9: value = 1', 'serial == 9: value = 0')}
 if m=='m1':variants['unsigned']=code.replace("'page':base//size+1","'page':base//size")
 for name,body in variants.items():
  write(f'oracle/{m}/{name}/solution.py',body);negatives.append({'id':name,'files':{'solution.py':f'oracle/{m}/{name}/solution.py'}})
 children.append({'id':m,'instruction':f'fixtures/{m}/instruction.md','checker':'check.py','fixtures':{'trunc.db':f'fixtures/{m}/trunc.db'},'oracle':{'task.json':f'oracle/{m}/task.json','probe.db':f'oracle/{m}/probe.db'},'gold':{'solution.py':f'oracle/{m}/gold/solution.py'},'negatives':negatives})
js('case.json',{'schema_version':1,'parent_id':'sqlite-db-truncate','image':image,'policy':policy,'children':children})
write('environment/Dockerfile',f'FROM {image}\nWORKDIR /app\n# suite.diagnostics.materialize injects only the selected synthetic fixture.\n')
