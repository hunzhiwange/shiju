"""Probe submitted Python behavior in a bounded independent interpreter."""
import json
from pathlib import Path
import subprocess
import sys

PROBE = r'''
import asyncio, sys
from run import *
kind=sys.argv[1]
if kind=='m1':
    calls=[]
    def factory(): calls.append(1)
    for jobs,limit in [([],1),([factory],2),([factory]*3,1)]:
        assert validate_tasks(jobs,limit) is None
    for jobs,limit,error in [((),1,TypeError),(None,1,TypeError),([None],1,TypeError),([factory,3],1,TypeError),([],True,TypeError),([],1.2,TypeError),([],'2',TypeError),([],0,ValueError),([],-2,ValueError)]:
        try: validate_tasks(jobs,limit)
        except error: pass
        else: raise AssertionError('invalid input accepted')
    assert calls==[]
else:
    async def probe():
        if kind=='m2':
            for count,limit in [(0,2),(1,3),(7,2),(5,3),(4,1)]:
                active=0; peak=0; seen=[]; gate=asyncio.Event()
                async def job(i):
                    nonlocal active,peak
                    active+=1; peak=max(peak,active);seen.append(i)
                    if active==min(count,limit): gate.set()
                    await gate.wait()
                    await asyncio.sleep(.01)
                    active-=1
                result=await asyncio.wait_for(run_tasks([lambda i=i:job(i) for i in range(count)],limit),1)
                assert result is None and sorted(seen)==list(range(count))
                assert peak==min(count,limit) and active==0
        elif kind=='m3':
            started=[]; cleaned=[]; ready=asyncio.Event(); hold=asyncio.Event()
            class Marker(Exception): pass
            async def failing():
                started.append('fail'); await ready.wait(); raise Marker('synthetic failure')
            async def sibling():
                started.append('sibling'); ready.set()
                try: await hold.wait()
                finally:
                    await asyncio.sleep(.03); cleaned.append('sibling')
            async def queued(): started.append('queued')
            def contains(e):
                return isinstance(e,Marker) or (isinstance(e,BaseExceptionGroup) and any(contains(x) for x in e.exceptions))
            try: await asyncio.wait_for(run_tasks([failing,sibling,queued,queued],2),1)
            except BaseException as e: assert contains(e),repr(e)
            else: raise AssertionError('failure swallowed')
            assert started==['fail','sibling'] and cleaned==['sibling']
            await asyncio.sleep(.02)
            assert started==['fail','sibling']
        elif kind=='m4':
            started=[]; cleaned=[]; ready=asyncio.Event(); hold=asyncio.Event()
            async def job(i):
                started.append(i)
                if len(started)==2: ready.set()
                try: await hold.wait()
                finally:
                    await asyncio.sleep(.04); cleaned.append(i)
            run=asyncio.create_task(run_tasks([lambda i=i:job(i) for i in range(5)],2))
            await asyncio.wait_for(ready.wait(),1)
            run.cancel()
            try: await asyncio.wait_for(run,1)
            except asyncio.CancelledError: pass
            else: raise AssertionError('caller cancellation swallowed')
            assert sorted(started)==[0,1] and sorted(cleaned)==[0,1]
            await asyncio.sleep(.02)
            assert sorted(started)==[0,1]
            completed=[]
            async def normal():
                try: await asyncio.sleep(0)
                finally:
                    await asyncio.sleep(.01);completed.append(1)
            assert await run_tasks([normal]*3,2) is None
            assert completed==[1]*3
        else: raise AssertionError('unknown kind')
    asyncio.run(probe())
'''


def check(app, oracle):
    kind = json.loads((oracle / 'expected.json').read_text())['id']
    if kind not in ('m1', 'm2', 'm3', 'm4'):
        raise ValueError('unknown diagnostic')
    if not (app / 'run.py').is_file():
        return False
    try:
        result = subprocess.run([sys.executable, '-c', PROBE, kind], cwd=app,
                                capture_output=True, timeout=8)
        return result.returncode == 0
    except subprocess.TimeoutExpired:
        return False


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()) else 1)
