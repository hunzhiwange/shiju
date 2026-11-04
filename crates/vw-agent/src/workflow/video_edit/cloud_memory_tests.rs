//! Only HTTPSConnection is substituted. The production child script streams real files.
#![cfg(unix)]
use super::*;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::Shutdown,
    os::unix::net::UnixListener,
    process::Command,
};
fn put_receipt(url: &str, mutate: bool) -> (Value, Option<Value>) {
    let dir = tempfile::tempdir().unwrap();
    let canonical = dir.path().canonicalize().unwrap();
    let root = canonical.as_path();
    std::fs::create_dir(root.join("http")).unwrap();
    std::fs::write(root.join("http/__init__.py"), "").unwrap();
    std::fs::write(root.join("http/client.py"),r#"import hashlib,json,os
class Response:
 status=200
 def close(self):pass
class HTTPSConnection:
 def __init__(self,*a,**kw):self.h=hashlib.sha256();self.chunks=[];self.headers={}
 def putrequest(self,*a,**kw):pass
 def putheader(self,k,v):self.headers[k]=v
 def endheaders(self):pass
 def send(self,b):
  self.h.update(b);self.chunks.append(len(b))
  if os.environ.get('MUTATE')=='yes':
   with open(os.environ['SOURCE'],'ab') as f:f.write(b'changed')
   os.environ['MUTATE']='no'
 def getresponse(self):
  with open(os.environ['TRACE'],'w') as f:json.dump({'sha256':self.h.hexdigest(),'chunks':self.chunks,'headers':self.headers},f)
  return Response()
 def close(self):pass
"#).unwrap();
    let path = root.join("source.mp4");
    let data = vec![b'x'; 2 * 1024 * 1024];
    std::fs::write(&path, &data).unwrap();
    let socket = root.join("io");
    let listener = UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut child = Command::new("python3")
        .args(["-c", MEMORY_PROCESS])
        .arg(&socket)
        .env("PYTHONPATH", root)
        .env("TRACE", root.join("trace"))
        .env("SOURCE", &path)
        .env("MUTATE", if mutate { "yes" } else { "no" })
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let (mut stream, _) = loop {
        match listener.accept() {
            Ok(value) => break value,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    child.kill().unwrap();
                    panic!("child did not connect");
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(e) => panic!("accept: {e}"),
        }
    };
    stream.set_nonblocking(false).unwrap();
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
    let request = json!({"kind":"put","url":url,"path":path,"bytes":data.len(),"sha256":hex::encode(Sha256::digest(&data)),"headers":{"Content-Type":"video/mp4"}});
    stream.write_all(&serde_json::to_vec(&request).unwrap()).unwrap();
    stream.shutdown(Shutdown::Write).unwrap();
    let mut response = Vec::new();
    if let Err(error) = stream.read_to_end(&mut response) {
        let _ = child.kill();
        let _ = child.wait();
        panic!("read memory receipt: {error}");
    }
    assert!(child.wait().unwrap().success());
    let trace = std::fs::read(root.join("trace")).ok().map(|b| serde_json::from_slice(&b).unwrap());
    (serde_json::from_slice(&response).unwrap(), trace)
}
#[test]
fn video_edit_prepare_native_put_streams_bounded_chunks_and_rejects_changed_source() {
    let url = "https://fixture.oss.aliyuncs.com/source?Signature=memory-only";
    let (result, trace) = put_receipt(url, false);
    assert_eq!(result["ok"], true);
    let trace = trace.unwrap();
    let chunks = trace["chunks"].as_array().unwrap();
    assert!(chunks.len() > 1 && chunks.iter().all(|n| n.as_u64().unwrap() <= 131072));
    assert_eq!(trace["sha256"], hex::encode(Sha256::digest(vec![b'x'; 2 * 1024 * 1024])));
    assert_eq!(trace["headers"]["Content-Length"], "2097152");
    assert_eq!(put_receipt(url, true).0["ok"], false);
}
#[test]
fn video_edit_prepare_native_put_rejects_nonofficial_or_insecure_destinations() {
    for url in [
        "http://fixture.oss.aliyuncs.com/a",
        "https://127.0.0.1/a",
        "https://evilaliyuncs.com/a",
        "https://bucket.aliyuncs.com.evil.test/a",
        "https://user@bucket.aliyuncs.com/a",
    ] {
        let (result, trace) = put_receipt(url, false);
        assert_eq!(result["ok"], false, "{url}");
        assert!(trace.is_none());
    }
}

fn call_receipt(mode: &str) -> Value {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let caller = root.join("occ.py");
    std::fs::write(&caller, r#"import os,json,sys
class Client:
 def _post(self,payload):
  mode=os.environ['MODE']
  self.transport_diagnostic={'phase':'response','httpStatus':200,'requestId':'12345678-1234-1234-1234-123456789abc'}
  if mode=='transport':
   self.transport_diagnostic['httpStatus']=503
   raise OSError('https://private.test/?token=never-visible')
  if mode=='auth':
   self.transport_diagnostic['httpStatus']=403
   return 403,[], 'secret'
  if mode=='stale':
   self.transport_diagnostic['httpStatus']=404
   return 404,[{'id':1,'error':{'code':-32000,'message':'MCP session not found or expired'}}], 'secret'
  if mode in ('draining','draining-active'):
   self.transport_diagnostic['httpStatus']=503
   return 503,[{'id':1,'error':{'code':-32000,'data':{'code':'SERVICE_RESTARTING','requestStarted':mode=='draining-active'}}}], 'secret'
  if mode=='conflicting':return 200,[{'id':None,'error':{'code':-32602}},{'id':1,'result':{}}], 'secret'
  if mode=='protocol':return 200,[{'id':1,'error':{'code':-32602,'message':'secret'}}], 'secret'
  if mode=='tool-error':return 200,[{'id':1,'result':{'isError':True,'error':{'code':-32602,'message':'secret'}}}], 'secret'
  return 200,[{'id':1,'result':{'isError':False}}], 'secret'
def main(args):
 if os.environ['MODE']=='not-sent':return 2
 c=Client()
 try:status,parsed,body=c._post({'method':'tools/call','id':1,'params':{'name':'create_project'}})
 except Exception:return 3
 out=args[args.index('--out')+1]
 with open(out,'w') as f:json.dump({'error':{'message':'secret'},'requestId':'12345678-1234-1234-1234-123456789abc'},f)
 return 0 if os.environ['MODE']=='success' else 1
if __name__=='__main__':sys.exit(main(sys.argv[1:]))
"#).unwrap();
    if mode == "unsupported" {
        std::fs::write(&caller, "import sys; sys.exit(2)\n").unwrap();
    }
    let socket = root.join("io");
    let listener = UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut child = Command::new("python3")
        .args(["-c", MEMORY_PROCESS])
        .arg(&socket)
        .env("MODE", mode)
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let (mut stream, _) = loop {
        match listener.accept() {
            Ok(value) => break value,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("memory caller did not connect");
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(error) => panic!("memory accept: {error}"),
        }
    };
    stream.set_nonblocking(false).unwrap();
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
    let request = json!({"kind":"call","operation":"create_project","args":{"name":"fixture"},
        "auth":root.join("auth"),"state":root.join("state"),"cwd":root,"occ":caller,"caller":CALLER_PROCESS});
    stream.write_all(&serde_json::to_vec(&request).unwrap()).unwrap();
    stream.shutdown(Shutdown::Write).unwrap();
    let mut bytes = Vec::new();
    if let Err(error) = stream.read_to_end(&mut bytes) {
        let _ = child.kill();
        let _ = child.wait();
        panic!("memory read: {error}");
    }
    assert!(child.wait().unwrap().success());
    serde_json::from_slice(&bytes).unwrap()
}

#[test]
fn video_edit_cloud_failure_preserves_only_proven_classification_and_request_id() {
    for (mode, outcome, exit, status) in [
        ("not-sent", "not_sent", 2, None),
        ("auth", "rejected", 1, Some(403)),
        ("protocol", "rejected", 1, Some(200)),
        ("tool-error", "unknown", 1, Some(200)),
        ("transport", "unknown", 3, Some(503)),
        ("stale", "rejected", 1, Some(404)),
        ("draining", "rejected", 1, Some(503)),
        ("draining-active", "unknown", 1, Some(503)),
        ("conflicting", "unknown", 1, Some(200)),
        ("unsupported", "unknown", 2, None),
    ] {
        let result = call_receipt(mode);
        assert_eq!(result["ok"], false);
        let failure = &result["failure"];
        assert_eq!(failure["outcome"], outcome, "{mode}: {result}");
        assert_eq!(failure["exit_code"], exit);
        assert_eq!(failure["http_status"].as_u64(), status);
        if !matches!(mode, "not-sent" | "unsupported") {
            assert_eq!(failure["request_id"], "12345678-1234-1234-1234-123456789abc");
        }
        let safe = CloudOperationFailure::from_receipt(failure);
        assert_eq!(safe.permits_resubmission(), outcome != "unknown");
        let encoded = serde_json::to_string(&result).unwrap();
        assert!(!encoded.contains("secret") && !encoded.contains("private.test"));
    }
}

#[test]
fn video_edit_cloud_failure_malformed_diagnostics_never_permit_resubmission() {
    for value in [
        json!(null),
        json!({"outcome":"not_sent"}),
        json!({"outcome":"not_sent","phase":"secret","exit_code":2,"http_status":null,"rpc_code":null,"request_id":null}),
        json!({"outcome":"rejected","phase":"response","exit_code":1,"http_status":403,"rpc_code":null,"request_id":"https://private.test/?token=secret"}),
    ] {
        let failure = CloudOperationFailure::from_receipt(&value);
        assert!(!failure.permits_resubmission());
        assert!(!failure.to_string().contains("secret"));
    }
}

#[test]
fn video_edit_cloud_failure_incomplete_success_retains_request_id_without_claiming_rejection() {
    let failure = CloudOperationFailure::unknown_response(
        &json!({"requestId":"12345678-1234-1234-1234-123456789abc","error":"secret"}),
    );
    assert!(!failure.permits_resubmission());
    assert_eq!(failure.request_id.as_deref(), Some("12345678-1234-1234-1234-123456789abc"));
    assert!(!failure.to_string().contains("secret"));
}
