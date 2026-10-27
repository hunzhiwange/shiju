import unittest
from pathlib import Path
from suite.diagnostics import load_case, run_checker
ROOT = Path(__file__).resolve().parents[1]
CASE, _ = load_case(ROOT / "case.json")
class CalibrationTests(unittest.TestCase):
    pass

def scenario(child, files, expected):
    def test(self):
        self.assertEqual(run_checker(CASE["image"], ROOT, child, files), expected)
    return test

for child in CASE["children"]:
    for name, files, code in [("gold",child["gold"],0),("nop",{},1)]+[(v["id"],v["files"],1) for v in child["negatives"]]:
        setattr(CalibrationTests,"test_"+child["id"]+"_"+name.replace("-","_"),scenario(child,files,code))
if __name__ == "__main__":
    unittest.main()
