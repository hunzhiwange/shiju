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
