"""Execute candidates with unseen local model files and real loopback HTTP."""
import http.client
import json
import math
from pathlib import Path
import socket
import subprocess
import sys
import time


def expected(model, text):
    # Independent accumulation: count occurrences before the matrix reduction.
    words = text.lower().split()
    logits = [math.fsum([model['bias'][i]] +
              [words.count(word)*weights[i] for word, weights in model['weights'].items()])
              for i in range(2)]
    delta = logits[1]-logits[0]
    positive = 1/(1+math.exp(-delta)) if delta >= 0 else math.exp(delta)/(1+math.exp(delta))
    return {'sentiment': 'positive' if delta > 0 else 'negative',
            'confidence': {'negative': 1-positive, 'positive': positive}}


def equal(actual, wanted):
    if isinstance(wanted, dict):
        return (isinstance(actual, dict) and actual.keys() == wanted.keys()
                and all(equal(actual[k], v) for k,v in wanted.items()))
    if isinstance(wanted, list):
        return (isinstance(actual, list) and len(actual) == len(wanted)
                and all(equal(a,b) for a,b in zip(actual,wanted)))
    if type(wanted) in (int, float):
        return (type(actual) in (int,float) and math.isfinite(actual)
                and math.isclose(actual,wanted,abs_tol=1e-8,rel_tol=1e-8))
    return type(actual) is type(wanted) and actual == wanted


def models():
    yield {'labels':['negative','positive'],'bias':[0.3,-0.2],
           'weights':{'sun':[-1,2],'rain':[2,-1],'calm':[0.1,0.2]}}
    yield {'labels':['negative','positive'],'bias':[5,-4],
           'weights':{'sun':[2,-1],'rain':[-3,2],'other':[0,0]}}
    yield {'labels':['negative','positive'],'bias':[0,0], 'weights':{'x':[0,0]}}
    yield {'labels':['negative','positive'],'bias':[10000,10001],
           'weights':{'sun':[-2,2],'rain':[3,-1]}}


def write_model(directory, model, broken=None):
    directory.mkdir(exist_ok=True)
    (directory/'model.json').write_text(json.dumps(model))
    (directory/'tokenizer.json').write_text(json.dumps({'lowercase':True,'split':'whitespace'}))
    if broken == 'missing-tokenizer':
        (directory/'tokenizer.json').unlink()
    elif broken == 'bad-json':
        (directory/'model.json').write_text('{')
    elif broken == 'bad-tokenizer':
        (directory/'tokenizer.json').write_text('{}')
    elif broken == 'bad-shape':
        model = dict(model, weights={'invalid':[1]})
        (directory/'model.json').write_text(json.dumps(model))
    elif broken == 'bad-labels':
        (directory/'model.json').write_text(json.dumps(dict(model, labels=['positive','negative'])))


def cli(app, request):
    result = subprocess.run([sys.executable,str(app/'solution.py')],cwd=app,
                            input=json.dumps(request).encode(),capture_output=True,timeout=3)
    if result.returncode or len(result.stdout)>100000:
        return None
    try:
        return json.loads(result.stdout)
    except (ValueError,UnicodeError):
        return None


def http_request(port, path, body):
    connection = http.client.HTTPConnection('127.0.0.1',port,timeout=2)
    try:
        connection.request('POST',path,body,{'Content-Type':'application/json'})
        response = connection.getresponse()
        data = response.read(100001)
        return response.status,response.getheader('Content-Type',''),data
    finally:
        connection.close()


def service(app, directory, model):
    with socket.socket() as probe:
        probe.bind(('127.0.0.1',0))
        port = probe.getsockname()[1]
    with (app/'service.log').open('wb') as log:
        process = subprocess.Popen([sys.executable,str(app/'solution.py'),str(directory),str(port)],
                                   cwd=app,stdout=log,stderr=log)
        try:
            deadline = time.monotonic()+5
            while True:
                try:
                    with socket.create_connection(('127.0.0.1',port),timeout=.2):
                        break
                except OSError:
                    if process.poll() is not None or time.monotonic()>=deadline:
                        return False
                    time.sleep(.05)
            probes = [('/sentiment',json.dumps({'text':text}),200,expected(model,text))
                      for text in ['sun','rain','SUN sun rain','unknown','calm','rain rain sun']]
            probes += [('/sentiment',body,400,{'error':'request'})
                       for body in ['{','{}','[]','null','{"text":1}','{"text":"  "}']]
            probes.append(('/wrong','{}',404,None))
            for path,body,code,wanted in probes:
                status,ctype,data = http_request(port,path,body)
                if status != code:
                    return False
                if wanted is not None and ('application/json' not in ctype or not equal(json.loads(data),wanted)):
                    return False
            # The offline container has no DNS or non-loopback interface.
            # Linux exposes the actual wildcard listener without expanding network access.
            listeners=Path('/proc/net/tcp').read_text().splitlines()[1:]
            if not any(row.split()[1] == f'00000000:{port:04X}' and row.split()[3]=='0A'
                       for row in listeners):
                return False
            return True
        except (OSError, ValueError, http.client.HTTPException):
            return False
        finally:
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=3)


def check(app, child):
    if not (app/'solution.py').is_file():
        return False
    for i,model in enumerate(models()):
        directory = app/f'probe-{i}'
        write_model(directory,model)
        if child == 'm4':
            if not service(app,directory,model):
                return False
        elif child == 'm1':
            wanted={'labels':model['labels'],'vocabulary':sorted(model['weights']),
                    'dimensions':[len(model['weights']),2]}
            if not equal(cli(app,{'model_dir':str(directory)}),wanted):
                return False
        else:
            for text in ['sun','rain','SUN sun rain','unknown','calm','rain rain sun','sun\nSUN']:
                request={'model_dir':str(directory),'text':text,'body':{'text':text,'extra':1}}
                wanted=expected(model,text)
                if child=='m3':
                    wanted={'status':200,'result':wanted}
                if not equal(cli(app,request),wanted):
                    return False
        if child == 'm3':
            for body in [None,[],{},1,{'text':False},{'text':1},{'text':['sun']},{'text':''},{'text':' \t'}]:
                if not equal(cli(app,{'model_dir':str(directory),'body':body}),{'status':400,'error':'request'}):
                    return False
        if child in ('m1','m3'):
            for broken in ['missing-tokenizer','bad-json','bad-tokenizer','bad-shape','bad-labels']:
                write_model(directory,model,broken)
                wanted={'error':'load'} if child=='m1' else {'status':503,'error':'load'}
                if not equal(cli(app,{'model_dir':str(directory),'body':{}}),wanted):
                    return False
    return True


def main():
    app,oracle = [Path(p).resolve() for p in sys.argv[1:]]
    child=json.loads((oracle/'task.json').read_text())['child']
    try:
        return 0 if check(app,child) else 1
    except subprocess.TimeoutExpired:
        return 1


if __name__=='__main__':
    sys.exit(main())
