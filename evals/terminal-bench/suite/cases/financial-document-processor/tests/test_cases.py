"""Execute real programs against synthetic document directories."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from suite.diagnostics import copy_files, load_case
ROOT=Path(__file__).resolve().parents[1]
CASE,_=load_case(ROOT/'case.json')
class Cases(unittest.TestCase):
    def verify(self, child, files, expected, mutate=None):
        with tempfile.TemporaryDirectory() as directory:
            app=Path(directory)/'app'; app.mkdir()
            oracle=Path(directory)/'oracle'; oracle.mkdir()
            copy_files(ROOT,child['fixtures'],app);copy_files(ROOT,files,app)
            copy_files(ROOT,child['oracle'],oracle)
            if mutate: mutate(app)
            result=subprocess.run([sys.executable,str(ROOT/'check.py'),str(app),str(oracle)],capture_output=True,timeout=20)
            self.assertEqual(result.returncode,expected,result.stderr.decode())
            self.assertNotIn(b'Traceback',result.stderr)
    def test_symlink_program_rejected(self):
        def mutate(app):
            (app/'solution.py').rename(app/'saved.py');(app/'solution.py').symlink_to(app/'saved.py')
        for c in CASE['children']: self.verify(c,c['gold'],1,mutate)
    def test_modified_input_rejected(self):
        for c in CASE['children']:
            self.verify(c,c['gold'],1,lambda a:(a/'documents/a.txt').write_text('INVOICE\nTotal: 0.00\n'))
    def test_duplicate_json_rejected(self):
        c=CASE['children'][0]
        self.verify(c,c['gold'],1,lambda a:(a/'solution.py').write_text("from pathlib import Path\nPath('result.json').write_text('{\"a.txt\":\"other\",\"a.txt\":\"invoice\",\"b.txt\":\"other\",\"c.txt\":\"other\",\"d.txt\":\"invoice\"}')\n"))
    def test_zero_vat_empty_accepted(self):
        c=CASE['children'][2]
        self.verify(c,c['gold'],0,lambda a:(a/'solution.py').write_text((a/'solution.py').read_text().replace("'0.00')","'')")))
    def test_nonfinite_amount_rejected(self):
        c=CASE['children'][1]
        self.verify(c,c['gold'],1,lambda a:(a/'solution.py').write_text((a/'solution.py').read_text()+"\nr=json.loads(Path('result.json').read_text());r['a.txt'][0]='NaN';Path('result.json').write_text(json.dumps(r))\n"))
    def test_missing_csv_row_rejected(self):
        c=CASE['children'][3]
        self.verify(c,c['gold'],1,lambda a:(a/'solution.py').write_text((a/'solution.py').read_text().replace('w.writerows(rows)','w.writerows(rows[:1])')))
    def test_output_symlink_rejected(self):
        for c in CASE['children']:
            output='invoices/summary.csv' if c['id']=='m4' else 'result.json'
            def mutate(a):
                p=a/'solution.py'
                p.write_text(p.read_text()+f"\np=Path({output!r});p.rename('saved-output');p.symlink_to(Path('saved-output').resolve())\n")
            self.verify(c,c['gold'],1,mutate)
    def test_duplicate_csv_invoice_rejected(self):
        c=CASE['children'][3]
        self.verify(c,c['gold'],1,lambda a:(a/'solution.py').write_text((a/'solution.py').read_text().replace('w.writerows(rows)','w.writerows([rows[0],rows[0]])')))
    def test_changed_moved_document_rejected(self):
        c=CASE['children'][3]
        self.verify(c,c['gold'],1,lambda a:(a/'solution.py').write_text((a/'solution.py').read_text()+"\nPath('invoices/a.txt').write_text('changed')\n"))
    def test_oversized_csv_field_rejected(self):
        c=CASE['children'][3]
        self.verify(c,c['gold'],1,lambda a:(a/'solution.py').write_text((a/'solution.py').read_text()+"\nPath('invoices/summary.csv').write_text('x'*200000)\n"))
def artifact_test(c,files,expected):
    def test(self): self.verify(c,files,expected)
    return test
for c in CASE['children']:
    for name,files,code in [('gold',c['gold'],0),('nop',{},1)]+[(n['id'],n['files'],1) for n in c['negatives']]:
        setattr(Cases,'test_'+c['id']+'_'+name.replace('-','_'),artifact_test(c,files,code))
