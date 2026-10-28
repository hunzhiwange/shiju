import json,unittest
from pathlib import Path
from suite import diagnostics
ROOT=Path(__file__).resolve().parents[1]
CASE=json.loads((ROOT/'case.json').read_text())
class SamCellCases(unittest.TestCase):
 def test_contract(self):diagnostics.load_case(ROOT/'case.json')
def add(child,name,files,expected):
 def test(self):
  self.assertEqual(diagnostics.run_checker(CASE['image'],ROOT,child,files),expected)
 setattr(SamCellCases,'test_'+child['id']+'_'+name.replace('-','_'),test)
for child in CASE['children']:
 for name,files,expected in [('gold',child['gold'],0),('nop',{},1)]+[(n['id'],n['files'],1) for n in child['negatives']]:add(child,name,files,expected)
