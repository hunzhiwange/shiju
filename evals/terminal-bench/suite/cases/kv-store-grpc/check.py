"""Compile descriptors, execute CLIs, and probe independently compiled real RPCs."""
import importlib
import json
from pathlib import Path
import socket
import subprocess
import sys
import tempfile

SCHEMA = '''syntax="proto3";
service KVStore { rpc GetVal(GetValRequest) returns(GetValResponse);
 rpc SetVal(SetValRequest) returns(SetValResponse); }
message GetValRequest {string key=1;}
message SetValRequest {string key=1; int32 value=2;}
message GetValResponse {int32 val=1;}
message SetValResponse {int32 val=1;}
'''


def compile_proto(source, output):
    result = subprocess.run([sys.executable, '-m', 'grpc_tools.protoc',
        '-I'+str(source.parent), '--python_out='+str(output),
        '--grpc_python_out='+str(output), str(source)], capture_output=True, timeout=10)
    return result.returncode == 0


def cli(app, operations):
    result = subprocess.run([sys.executable, str(app/'solution.py')], cwd=app,
                            input=json.dumps(operations).encode(), capture_output=True, timeout=3)
    if result.returncode:
        return None
    try:
        return json.loads(result.stdout)
    except (ValueError, UnicodeError):
        return None


def probes():
    for key, value in [('Maple', -19), ('键值', 2147483647), ('z', -2147483648)]:
        yield [
            {'op':'get','key':'absent'}, {'op':'set','key':key,'value':value},
            {'op':'set','key':key.lower()+'other','value':37},
            {'op':'set','key':key.swapcase()+'Case','value':44},
            {'op':'set','key':key+'Case','value':-7},
            {'op':'get','key':key.swapcase()+'Case'},
            {'op':'get','key':key}, {'op':'set','key':key,'value':0},
            {'op':'get','key':key}, {'op':'get','key':key.lower()+'other'},
            {'op':'set','key':key,'value':-23}, {'op':'get','key':key}]


def expected(operations, strict):
    values = {}
    answer = []
    for op in operations:
        if strict and (not isinstance(op,dict) or op.get('op') not in ('get','set')
            or not isinstance(op.get('key'),str) or not op['key']
            or (op['op']=='set' and (type(op.get('value')) is not int
                or not -2**31 <= op['value'] < 2**31))):
            answer.append({'error':'invalid'})
            continue
        if op['op']=='set':
            values[op['key']]=op['value']
        if strict:
            answer.append({'val':values[op['key']]} if op['key'] in values else {'error':'missing'})
        else:
            answer.append(values.get(op['key'],0))
    return answer


def equivalent(actual, wanted):
    # JSON booleans must not compare equal to numeric values.
    if type(actual) is not type(wanted):
        return False
    if isinstance(wanted, list):
        return len(actual)==len(wanted) and all(equivalent(a,b) for a,b in zip(actual,wanted))
    if isinstance(wanted,dict):
        return actual.keys()==wanted.keys() and all(equivalent(actual[k],v) for k,v in wanted.items())
    return actual==wanted


def schema(app):
    source=app/'kv-store.proto'
    if not source.is_file():
        return False
    with tempfile.TemporaryDirectory() as work:
        if not compile_proto(source,Path(work)):
            return False
        sys.path.insert(0,work)
        p=importlib.import_module('kv_store_pb2')
        from google.protobuf.descriptor_pb2 import FileDescriptorProto
        descriptor=FileDescriptorProto.FromString(p.DESCRIPTOR.serialized_pb)
        if descriptor.syntax!='proto3':
            return False
        messages=p.DESCRIPTOR.message_types_by_name
        fields={'GetValRequest':{'key':(1,9)},'SetValRequest':{'key':(1,9),'value':(2,5)},
                'GetValResponse':{'val':(1,5)},'SetValResponse':{'val':(1,5)}}
        if p.DESCRIPTOR.package or set(messages)!=set(fields):
            return False
        for name,wanted in fields.items():
            if any(f.is_repeated for f in messages[name].fields):
                return False
            if {f.name:(f.number,f.type) for f in messages[name].fields}!=wanted:
                return False
        service=p.DESCRIPTOR.services_by_name.get('KVStore')
        if not service or {m.name for m in service.methods}!={'GetVal','SetVal'}:
            return False
        for m in service.methods:
            if (m.input_type.name!=m.name+'Request' or m.output_type.name!=m.name+'Response'
                or m.client_streaming or m.server_streaming):
                return False
        for value in [-2**31,-17,0,2**31-1]:
            a=p.SetValRequest(key='温度',value=value)
            if p.SetValRequest.FromString(a.SerializeToString())!=a:
                return False
        return True


def service(app):
    import grpc
    if not (app/'server.py').is_file():
        return False
    # Candidate bindings come from its provided schema; client bindings are trusted and separate.
    if not compile_proto(app/'kv-store.proto',app):
        return False
    with tempfile.TemporaryDirectory() as work:
        trusted=Path(work)
        (trusted/'kv-store.proto').write_text(SCHEMA)
        if not compile_proto(trusted/'kv-store.proto',trusted):
            raise RuntimeError('trusted schema compilation failed')
        sys.path.insert(0,work)
        p=importlib.import_module('kv_store_pb2')
        rpc=importlib.import_module('kv_store_pb2_grpc')
        with socket.socket() as probe:
            probe.bind(('127.0.0.1',0))
            port=probe.getsockname()[1]
        with (app/'service.log').open('wb') as log:
            process=subprocess.Popen([sys.executable,str(app/'server.py'),str(port)],
                                     cwd=app,stdout=log,stderr=log)
            channel=grpc.insecure_channel(f'127.0.0.1:{port}')
            try:
                grpc.channel_ready_future(channel).result(timeout=4)
                stub=rpc.KVStoreStub(channel)
                for operations in probes():
                    for op,value in zip(operations,expected(operations,False)):
                        response=(stub.SetVal(p.SetValRequest(key=op['key'],value=op['value']),timeout=2)
                                  if op['op']=='set' else stub.GetVal(p.GetValRequest(key=op['key']),timeout=2))
                        if response.val!=value:
                            return False
                    with grpc.insecure_channel(f'127.0.0.1:{port}') as second:
                        if rpc.KVStoreStub(second).GetVal(p.GetValRequest(key=operations[-1]['key']),timeout=2).val!=-23:
                            return False
                return process.poll() is None
            except (grpc.RpcError,grpc.FutureTimeoutError):
                return False
            finally:
                channel.close()
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill(); process.wait(timeout=3)


def check(app,child):
    if child=='m1':
        return schema(app)
    if child=='m4':
        return service(app)
    if not (app/'solution.py').is_file():
        return False
    for operations in probes():
        if child=='m3':
            operations += [None,[],{}, {'op':'get','key':3},{'op':'set','key':'','value':1},
                           {'op':'remove','key':'x'},{'op':'set','key':'x'},
                           *[{'op':'set','key':'guard','value':v} for v in
                             [True,False,1.0,'1',2**31,-2**31-1]],
                           {'op':'get','key':'guard'}, {'op':'get','key':'missing'},
                           {'op':'set','key':'after','value':31}, {'op':'get','key':'after'}]
        if not equivalent(cli(app,operations),expected(operations,child=='m3')):
            return False
    return True


if __name__=='__main__':
    app,oracle=[Path(p).resolve() for p in sys.argv[1:]]
    try:
        ok=check(app,json.loads((oracle/'task.json').read_text())['child'])
    except subprocess.TimeoutExpired:
        ok=False
    sys.exit(0 if ok else 1)
