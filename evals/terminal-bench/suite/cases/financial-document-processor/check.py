"""Run candidate code; compare real artifacts and conserved document bytes."""
import csv
from decimal import Decimal, InvalidOperation
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def regular(path):
    assert path.is_file() and not path.is_symlink()
    assert all(not p.is_symlink() for p in path.parents)
    return path


def sha(path):
    return hashlib.sha256(regular(path).read_bytes()).hexdigest()


def unique(pairs):
    result={}
    for k,v in pairs:
        assert k not in result
        result[k]=v
    return result


def money(value):
    assert isinstance(value,str)
    number=Decimal(value if value else '0')
    assert number.is_finite() and number >= 0
    return number


def check(app,oracle):
    spec=json.loads((oracle/'expected.json').read_text())
    program=regular(app/'solution.py')
    for name,digest in spec['hashes'].items(): assert sha(app/'documents'/name)==digest
    result=subprocess.run([sys.executable,str(program)],cwd=app,capture_output=True,timeout=5)
    assert result.returncode==0
    expected=spec['expected']
    if spec['mode']!='m4':
        for name,digest in spec['hashes'].items(): assert sha(app/'documents'/name)==digest
        actual=json.loads(regular(app/'result.json').read_text(),object_pairs_hook=unique)
        assert isinstance(actual,dict) and actual.keys()==expected.keys()
        if spec['mode']=='m1': assert actual==expected
        else:
            for name,values in expected.items():
                assert isinstance(actual[name],list) and len(actual[name])==2
                assert [money(v) for v in actual[name]]==[money(v) for v in values]
    else:
        assert (app/'documents').is_dir() and not any((app/'documents').iterdir())
        for folder,names in [('invoices',set(expected['invoices'])|{'summary.csv'}),('other',set(expected['other']))]:
            directory=app/folder
            assert directory.is_dir() and not directory.is_symlink()
            assert {p.name for p in directory.iterdir()}==names
            for name in names-{'summary.csv'}: assert sha(directory/name)==spec['hashes'][name]
        with regular(app/'invoices/summary.csv').open(newline='') as f:
            rows=list(csv.reader(f))
        assert rows[0]==['filename','total_amount','vat_amount']
        assert len(rows)==len(expected['invoices'])+2
        assert all(len(r)==3 for r in rows)
        assert rows[-1][0]=='total'
        entries=rows[1:-1]
        assert len({r[0] for r in entries})==len(entries)
        assert {r[0] for r in entries}==set(expected['invoices'])
        for name,total,vat in entries:
            assert [money(total),money(vat)]==[money(v) for v in expected['invoices'][name]]
        assert [money(v) for v in rows[-1][1:]]==[sum(money(r[i]) for r in entries) for i in (1,2)]


if __name__=='__main__':
    try:
        check(Path(sys.argv[1]).resolve(),Path(sys.argv[2]).resolve())
    except (AssertionError,OSError,ValueError,csv.Error,InvalidOperation,subprocess.TimeoutExpired,KeyError,TypeError,IndexError):
        sys.exit(1)
