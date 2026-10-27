"""Fresh real-artifact oracle/nop/negative controls; never score prose."""
import importlib.util
import json
from pathlib import Path
import struct
import unittest
from suite import diagnostics

ROOT=Path(__file__).resolve().parents[1]


class MakeDoomForMipsCases(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.case,_=diagnostics.load_case(ROOT/'case.json')


def add_test(child_id,identity,expected):
    def test(self):
        child=next(c for c in self.case['children'] if c['id']==child_id)
        files=child['gold'] if identity=='gold' else {} if identity=='nop' else next(n['files'] for n in child['negatives'] if n['id']==identity)
        self.assertEqual(diagnostics.run_checker(self.case['image'],ROOT,child,files),expected)
    setattr(MakeDoomForMipsCases,f'test_{child_id}_{identity.replace("-","_")}',test)


for child in json.loads((ROOT/'case.json').read_text())['children']:
    for identity,expected in [('gold',0),('nop',1)]+[(n['id'],1) for n in child['negatives']]:
        add_test(child['id'],identity,expected)


class InterpreterContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec=importlib.util.spec_from_file_location('reference',ROOT/'oracle/reference_vm.py')
        cls.vm=importlib.util.module_from_spec(spec);spec.loader.exec_module(cls.vm)
        spec=importlib.util.spec_from_file_location('checker',ROOT/'check.py')
        cls.check=importlib.util.module_from_spec(spec);spec.loader.exec_module(cls.check)

    def test_return_delay_slot_is_executed(self):
        self.assertEqual(self.vm.execute(self.check.elf([0x24020001,0x03e00008,0x24420007]))['v0'],8)

    def test_taken_branch_skips_only_after_delay_slot(self):
        data=self.check.elf([0x10000002,0x24020011,0x24020063,0x03e00008,0x24420005])
        self.assertEqual(self.vm.execute(data)['v0'],22)

    def test_truncated_segments_rejected(self):
        with self.assertRaises(ValueError): self.vm.load(self.check.elf([0x03e00008,0])[:-1])

    def test_unsigned_byte_and_zero_fill(self):
        data=self.check.elf([0x3c080001,0x35080020,0x91020000,0x03e00008,0,0,0,0],tail=b'\xff\x80\x00\x00')
        self.assertEqual(self.vm.execute(data)['v0'],255)


if __name__=='__main__': unittest.main()
