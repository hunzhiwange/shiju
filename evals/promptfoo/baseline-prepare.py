#!/usr/bin/env python3
"""Read existing token-plan profile and bind it to the existing Gateway; no model call."""
import ctypes,hashlib,importlib.util,json,pathlib,sqlite3,subprocess,urllib.request
root=pathlib.Path(__file__).resolve().parent;repo=root.parents[1];out=root/'results/accuracy-baseline'
headers={'Content-Type':'application/json','X-Platform-ID':'10000','X-Company-ID':'100000000'}
def request(path,data=None):
 req=urllib.request.Request('http://127.0.0.1:42617'+path,headers=headers,
  data=None if data is None else json.dumps(data).encode(),method='GET' if data is None else 'PATCH')
 return json.load(urllib.request.urlopen(req,timeout=15))
health=request('/v1/health');config=request('/v1/config')
with sqlite3.connect('file:'+str(pathlib.Path.home()/'.shiju-agent/database/vibe-window.sqlite3')+'?mode=ro',uri=True) as db:
 candidates=[json.loads(raw).get('providers',{}).get('token-plan',{}) for (raw,) in db.execute('select payload_json from system_configs')]
profiles=[p for p in candidates if 'deepseek-v4-flash-0731' in p.get('models',{})]
if len(profiles)!=1:raise RuntimeError('token-plan profile missing or ambiguous')
profile=profiles[0]
# Copy only existing endpoint/model metadata. No credentials are copied or logged.
provider={'api':profile['api'],'adapter':'openai-compatible','models':{'deepseek-v4-flash-0731':profile['models']['deepseek-v4-flash-0731']}}
request('/v1/config',{'providers':{'token-plan':provider}})
auth=json.loads((pathlib.Path.home()/'.shiju-agent/auth.json').read_text()).get('token-plan')
if not auth or auth.get('type')!='api' or not auth.get('key'):raise RuntimeError('existing token-plan credential missing')
req=urllib.request.Request('http://127.0.0.1:42617/v1/auth/token-plan',headers=headers,
 data=json.dumps({'type':'api','key':auth['key']}).encode(),method='PUT')
if json.load(urllib.request.urlopen(req,timeout=15)) is not True:raise RuntimeError('Gateway credential binding failed')
cat=request('/v1/provider');p=next((p for p in cat.get('all',[]) if p['id']=='token-plan'),None)
if not p or 'deepseek-v4-flash-0731' not in p.get('models',{}):raise RuntimeError('target model unresolved by live Gateway')
buf=ctypes.create_string_buffer(4096);ctypes.CDLL('/usr/lib/libproc.dylib').proc_pidpath(health['runtime']['pid'],buf,4096)
exe=pathlib.Path(buf.value.decode());sha=hashlib.sha256()
with exe.open('rb') as f:
 while chunk:=f.read(1048576):sha.update(chunk)
if sha.hexdigest()!=health['build_id']:raise RuntimeError('Gateway executable identity changed')
spec=importlib.util.spec_from_file_location('preflight',root/'gateway-preflight.py');preflight=importlib.util.module_from_spec(spec);spec.loader.exec_module(preflight)
model=p['models']['deepseek-v4-flash-0731']
info={'mode':'unisolated-user-requested','isolationVerified':False,'backgroundSettlement':'unverified',
 'model':'token-plan/deepseek-v4-flash-0731','endpoint':'http://127.0.0.1:42617/v1/chat/stream',
 'gatewayBuildId':health['build_id'],'gatewayPid':health['runtime']['pid'],'gatewayExecutable':str(exe),
 'workingTreeCommit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),
 'workingTreeSourceDigest':preflight.source_digest(repo),'binarySourceMapping':'unverified-existing-Gateway',
 'modelCatalogueDigest':hashlib.sha256(json.dumps(model,sort_keys=True).encode()).hexdigest(),
 'permissionsDigest':hashlib.sha256(json.dumps({'autonomy':config.get('autonomy'),'security':config.get('security')},sort_keys=True).encode()).hexdigest(),
 'parameters':{'temperature':'provider-default','top_p':'provider-default','seed':'provider-default','max_output_tokens':'provider-default'},
 'previousTokenPlan':config.get('providers',{}).get('token-plan')}
out.mkdir(parents=True,exist_ok=True);(out/'live-environment.json').write_text(json.dumps(info,indent=2))
print('existing Gateway resolves token-plan/deepseek-v4-flash-0731; model requests=0')
