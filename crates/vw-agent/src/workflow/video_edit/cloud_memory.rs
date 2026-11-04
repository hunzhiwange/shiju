//! Native process transport with a private Unix socket, never a receipt file, for tickets.
use super::*;
use crate::workflow::video_edit::cloud_failure::{CloudFailurePhase, CloudOperationFailure};
use anyhow::{Context, Result, ensure};

const CALLER_PROCESS: &str = include_str!("cloud_caller.py");

// The official CLI still handles auth, schemas, retries and RPCs. Its --out FD is
// an anonymous pipe. Signed responses and PUT tickets only cross memory sockets.
const MEMORY_PROCESS: &str = r#"import sys,os,json,socket,subprocess,urllib.parse,http.client,hashlib,pathlib
s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
s.connect(sys.argv[1])
r=json.loads(s.makefile('rb').read())
may_have_sent=False
try:
    if r['kind']=='call':
        e={k:v for k,v in os.environ.items() if not k.startswith(('SHIJU_','OCC_'))}
        e['OCC_SHIJU_ENV']=r['auth'];e['OCC_STATE_DIR']=r['state'];e['PYTHONDONTWRITEBYTECODE']='1'
        rd,wr=os.pipe();dr,dw=os.pipe()
        payload=json.dumps(r['args']).encode()
        p=subprocess.Popen(['python3','-c',r['caller'],str(dw),r['occ'],r['operation'],'call',r['operation'],'-','--no-wait','--out','/dev/fd/'+str(wr)],stdin=subprocess.PIPE,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,pass_fds=(wr,dw),cwd=r['cwd'],env=e)
        may_have_sent=True
        os.close(wr);os.close(dw)
        try:p.stdin.write(payload)
        except BrokenPipeError:pass
        finally:p.stdin.close()
        data=bytearray();too_large=False
        with os.fdopen(rd,'rb') as f:
            while True:
                chunk=f.read(131072)
                if not chunk:break
                if len(data)+len(chunk)<=16*1024*1024:data.extend(chunk)
                else:too_large=True
        with os.fdopen(dr,'rb') as f:diagnostic=json.loads(f.read(4096))
        code=p.wait()
        if code==0 and not too_large:
            try:result={'ok':True,'value':json.loads(data)}
            except (ValueError,UnicodeError):result={'ok':False,'failure':diagnostic}
        else:result={'ok':False,'failure':diagnostic}
    else:
        u=urllib.parse.urlsplit(r['url'])
        if u.scheme!='https' or not u.hostname or not u.hostname.endswith('.aliyuncs.com') or u.username or u.password or u.fragment or u.port not in (None,443): raise ValueError()
        path=pathlib.Path(r['path'])
        if os.path.realpath(path)!=str(path):raise ValueError()
        fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK)
        with os.fdopen(fd,'rb') as f:
            before=os.fstat(f.fileno());h=hashlib.sha256()
            while True:
                b=f.read(131072)
                if not b:break
                h.update(b)
            if h.hexdigest()!=r['sha256'] or before.st_size!=r['bytes'] or os.stat(path)!=os.fstat(f.fileno()):raise ValueError()
            f.seek(0)
            may_have_sent=True
            c=http.client.HTTPSConnection(u.hostname,timeout=60)
            try:
                c.putrequest('PUT',urllib.parse.urlunsplit(('', '',u.path,u.query,'')),skip_accept_encoding=True)
                c.putheader('Content-Length',str(r['bytes']))
                for k,v in r['headers'].items():
                    if k.lower() not in ('content-type','content-md5') and not k.lower().startswith('x-oss-'):raise ValueError()
                    c.putheader(k,v)
                c.endheaders()
                while True:
                    b=f.read(131072)
                    if not b:break
                    c.send(b)
                response=c.getresponse()
                ok=200<=response.status<300
                response.close()
            finally:c.close()
            after=os.fstat(f.fileno())
            if (before.st_dev,before.st_ino,before.st_size,before.st_mtime_ns,before.st_ctime_ns)!=(after.st_dev,after.st_ino,after.st_size,after.st_mtime_ns,after.st_ctime_ns) or os.stat(path)!=after:raise ValueError()
        result={'ok':ok,'value':{}}
except Exception:
    result={'ok':False,'failure':{'outcome':'unknown' if may_have_sent else 'not_sent','phase':'process' if may_have_sent else 'spawn','exit_code':None,'http_status':None,'rpc_code':None,'request_id':None}}
s.sendall(json.dumps(result).encode());s.close()
"#;

impl ShijuCloudVideoToolProvider {
    #[cfg(unix)]
    pub(super) async fn memory_exchange(&self, mut request: Value) -> Result<Value> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let setup = || -> Result<_> {
            ensure!(!self.cancelled(), "cancelled");
            let workspace = self.context.path_security_policy().workspace_dir.clone();
            self.authorize_path(&workspace).map_err(anyhow::Error::msg)?;
            let dir = tempfile::Builder::new().prefix(".video-memory-").tempdir_in(workspace)?;
            let socket = dir.path().join("io");
            let listener = tokio::net::UnixListener::bind(&socket)?;
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))?;
            Ok((dir, socket, listener))
        };
        let (dir, socket, listener) =
            setup().map_err(|_| CloudOperationFailure::not_sent(CloudFailurePhase::Preflight))?;
        request["caller"] = json!(CALLER_PROCESS);
        request["auth"] = json!(self.hub_root.join(".env"));
        request["state"] = json!(dir.path().join("occ-state"));
        request["cwd"] = json!(self.hub_root);
        request["occ"] = json!(self.occ);
        let command = format!(
            "python3 -c {} {}",
            shell_words::quote(MEMORY_PROCESS),
            shell_words::quote(socket.to_str().context("socket path encoding")?)
        );
        let mut result = self.native("bash",json!({"command":command,"description":"受控云端工程与原片传输（内存票据）","yield_time_ms":250,"timeout":300000})).map_err(|_| CloudOperationFailure::not_sent(CloudFailurePhase::NativeLaunch))?;
        let mut process = CloudProcess {
            provider: self,
            id: result["process_id"]
                .as_str()
                .ok_or_else(|| CloudOperationFailure::not_sent(CloudFailurePhase::NativeLaunch))?
                .into(),
            terminal: result["state"] != "running",
        };
        if process.terminal {
            return Err(CloudOperationFailure::not_sent(CloudFailurePhase::NativeLaunch).into());
        }
        let handed_off = std::sync::atomic::AtomicBool::new(false);
        let abort = self.context.tool_use_context().abort_token().cloned().unwrap_or_default();
        let exchange = async {
            let (mut stream, _) = listener.accept().await?;
            let payload = serde_json::to_vec(&request)?;
            handed_off.store(true, std::sync::atomic::Ordering::Relaxed);
            stream.write_all(&payload).await?;
            stream.shutdown().await?;
            let mut data = Vec::new();
            stream.take(MAX_RECEIPT_BYTES + 1).read_to_end(&mut data).await?;
            ensure!(data.len() as u64 <= MAX_RECEIPT_BYTES, "cloud receipt too large");
            let value: Value =
                serde_json::from_slice(&data).context("invalid cloud memory receipt")?;
            if value["ok"] != true {
                return Err(CloudOperationFailure::from_receipt(&value["failure"]).into());
            }
            Ok::<_, anyhow::Error>(value["value"].clone())
        };
        let exchanged: Result<Value> = tokio::select! {
            value = tokio::time::timeout(std::time::Duration::from_secs(300),exchange) => value.context("cloud memory transport timed out").and_then(|v| v),
            _ = abort.cancelled() => Err(if handed_off.load(std::sync::atomic::Ordering::Relaxed) {
                CloudOperationFailure::unknown(CloudFailurePhase::Cancelled)
            } else { CloudOperationFailure::not_sent(CloudFailurePhase::Cancelled) }.into()),
        };
        let value = exchanged.map_err(|error| {
            if error.downcast_ref::<CloudOperationFailure>().is_some() {
                error
            } else if handed_off.load(std::sync::atomic::Ordering::Relaxed) {
                CloudOperationFailure::unknown(CloudFailurePhase::Receipt).into()
            } else {
                CloudOperationFailure::not_sent(CloudFailurePhase::NativeLaunch).into()
            }
        })?;
        while result["state"] == "running" {
            ensure!(!self.cancelled(), "cancelled");
            tokio::task::yield_now().await;
            result = self.native("bash_wait",json!({"process_id":process.id,"cursor":result["next_cursor"],"yield_time_ms":250})).map_err(|_| CloudOperationFailure::unknown(CloudFailurePhase::Process))?;
        }
        process.terminal = true;
        if result["state"] != "exited" || result["exit_code"] != 0 {
            return Err(CloudOperationFailure::unknown(CloudFailurePhase::Process).into());
        }
        Ok(value)
    }
    #[cfg(not(unix))]
    pub(super) async fn memory_exchange(&self, _request: Value) -> Result<Value> {
        anyhow::bail!("memory-only cloud upload transport requires POSIX")
    }
}

#[cfg(test)]
#[path = "cloud_memory_tests.rs"]
mod tests;
