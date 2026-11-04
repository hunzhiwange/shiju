//! Native process transport with a private Unix socket, never a receipt file, for tickets.
use super::*;
use anyhow::{Context, Result, ensure};

// The official CLI still handles auth, schemas, retries and RPCs. Its --out FD is
// an anonymous pipe. Signed responses and PUT tickets only cross memory sockets.
const MEMORY_PROCESS: &str = r#"import sys,os,json,socket,subprocess,urllib.parse,http.client,hashlib,pathlib
s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
s.connect(sys.argv[1])
r=json.loads(s.makefile('rb').read())
try:
    if r['kind']=='call':
        e={k:v for k,v in os.environ.items() if not k.startswith(('SHIJU_','OCC_'))}
        e['OCC_SHIJU_ENV']=r['auth'];e['OCC_STATE_DIR']=r['state'];e['PYTHONDONTWRITEBYTECODE']='1'
        rd,wr=os.pipe()
        p=subprocess.Popen(['python3',r['occ'],'call',r['operation'],'-','--no-wait','--out','/dev/fd/'+str(wr)],stdin=subprocess.PIPE,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,pass_fds=(wr,),cwd=r['cwd'],env=e)
        os.close(wr)
        p.stdin.write(json.dumps(r['args']).encode());p.stdin.close()
        with os.fdopen(rd,'rb') as f: data=f.read(16*1024*1024+1)
        code=p.wait()
        result={'ok':True,'value':json.loads(data)} if code==0 and len(data)<=16*1024*1024 else {'ok':False}
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
    result={'ok':False}
s.sendall(json.dumps(result).encode());s.close()
"#;

impl ShijuCloudVideoToolProvider {
    #[cfg(unix)]
    pub(super) async fn memory_exchange(&self, mut request: Value) -> Result<Value> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        ensure!(!self.cancelled(), "cancelled");
        let workspace = self.context.path_security_policy().workspace_dir.clone();
        self.authorize_path(&workspace).map_err(anyhow::Error::msg)?;
        let dir = tempfile::Builder::new().prefix(".video-memory-").tempdir_in(workspace)?;
        let socket = dir.path().join("io");
        // Unix-domain socket paths have a small OS limit; fail explicitly for long roots.
        let listener =
            tokio::net::UnixListener::bind(&socket).context("private cloud memory socket")?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))?;
        request["auth"] = json!(self.hub_root.join(".env"));
        request["state"] = json!(dir.path().join("occ-state"));
        request["cwd"] = json!(self.hub_root);
        request["occ"] = json!(self.occ);
        let command = format!(
            "python3 -c {} {}",
            shell_words::quote(MEMORY_PROCESS),
            shell_words::quote(socket.to_str().context("socket path encoding")?)
        );
        let mut result = self.native("bash",json!({"command":command,"description":"受控云端工程与原片传输（内存票据）","yield_time_ms":250,"timeout":300000})).map_err(anyhow::Error::msg)?;
        let mut process = CloudProcess {
            provider: self,
            id: result["process_id"].as_str().context("missing cloud process")?.into(),
            terminal: result["state"] != "running",
        };
        let abort = self.context.tool_use_context().abort_token().cloned().unwrap_or_default();
        let exchange = async {
            let (mut stream, _) = listener.accept().await?;
            stream.write_all(&serde_json::to_vec(&request)?).await?;
            stream.shutdown().await?;
            let mut data = Vec::new();
            stream.take(MAX_RECEIPT_BYTES + 1).read_to_end(&mut data).await?;
            ensure!(data.len() as u64 <= MAX_RECEIPT_BYTES, "cloud receipt too large");
            let value: Value =
                serde_json::from_slice(&data).context("invalid cloud memory receipt")?;
            ensure!(
                value["ok"] == true,
                "cloud operation unknown or failed; reconcile before resubmission"
            );
            Ok::<_, anyhow::Error>(value["value"].clone())
        };
        let value = tokio::select! {
            value = tokio::time::timeout(std::time::Duration::from_secs(300),exchange) => value.context("cloud memory transport timed out")??,
            _ = abort.cancelled() => anyhow::bail!("cancelled"),
        };
        while result["state"] == "running" {
            ensure!(!self.cancelled(), "cancelled");
            tokio::task::yield_now().await;
            result = self.native("bash_wait",json!({"process_id":process.id,"cursor":result["next_cursor"],"yield_time_ms":250})).map_err(anyhow::Error::msg)?;
        }
        process.terminal = true;
        ensure!(result["state"] == "exited" && result["exit_code"] == 0, "cloud process failed");
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
