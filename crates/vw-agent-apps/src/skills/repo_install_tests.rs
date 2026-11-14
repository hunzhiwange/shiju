use std::{io::{Cursor, Write}, sync::{Arc, Mutex}};

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};
use zip::{ZipWriter, write::SimpleFileOptions};

use super::{discovery::DiscoveryError, repos::SkillRepo};

fn repo(name: &str, branch: &str, enabled: bool) -> SkillRepo {
    SkillRepo { owner: "org".into(), name: name.into(), branch: branch.into(), enabled }
}

fn archive(files: &[(&str, &str)]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (path, content) in files {
        writer.start_file(format!("repo-snapshot/{path}"), SimpleFileOptions::default()).unwrap();
        writer.write_all(content.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

struct Server {
    endpoint: String,
    requests: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) { self.task.abort(); }
}

async fn serve(routes: Vec<(&'static str, u16, Vec<u8>)>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorded = requests.clone();
    let task = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            loop {
                let mut buffer = [0; 1024];
                let size = stream.read(&mut buffer).await.unwrap();
                if size == 0 { break; }
                request.extend_from_slice(&buffer[..size]);
                if request.windows(4).any(|part| part == b"\r\n\r\n") { break; }
            }
            let request = String::from_utf8(request).unwrap();
            let path = request.lines().next().unwrap().split_whitespace().nth(1).unwrap();
            recorded.lock().unwrap().push(path.to_owned());
            let (status, body) = routes.iter().find(|(route, _, _)| *route == path)
                .map(|(_, status, body)| (*status, body.as_slice()))
                .unwrap_or((404, b"secret upstream body"));
            let header = format!("HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = stream.write_all(header.as_bytes()).await;
            let _ = stream.write_all(body).await;
        }
    });
    Server { endpoint, requests, task }
}

#[tokio::test]
async fn cp12_repo_install_success() {
    let server = serve(vec![("/org/good/zip/refs/heads/master", 200,
        archive(&[("tools/demo/SKILL.md", "---\nname: actual\n---\nbody")]))]).await;
    let root = tempfile::tempdir().unwrap();
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut prepared = super::repo_install::prepare_at(&client, &repo("good", "HEAD", true),
        "tools/demo", root.path(), "demo", &server.endpoint).await.unwrap();
    assert_eq!(prepared.source.branch, "master");
    assert_eq!(prepared.installation.skill().name, "actual");
    assert!(!root.path().join("demo").exists());
    prepared.installation.publish().unwrap();
    assert!(root.path().join("demo/SKILL.md").is_file());
    prepared.installation.rollback().unwrap();
    assert!(!root.path().join("demo").exists());
    assert_eq!(*server.requests.lock().unwrap(), [
        "/org/good/zip/refs/heads/main", "/org/good/zip/refs/heads/master"]);
}

#[tokio::test]
async fn cp12_repo_install_rejects_without_side_effects() {
    let server = serve(vec![("/org/good/zip/refs/heads/main", 403, b"private".to_vec())]).await;
    let root = tempfile::tempdir().unwrap();
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let error = super::repo_install::prepare_at(&client, &repo("good", "main", true),
        "demo", root.path(), "demo", &server.endpoint).await.err().unwrap();
    assert!(matches!(error, super::repo_install::RepoInstallError::Repository(DiscoveryError::Forbidden)));
    assert_eq!(server.requests.lock().unwrap().len(), 1);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    assert!(super::repo_install::prepare_at(&client, &repo("good", "main", true),
        "../demo", root.path(), "demo", &server.endpoint).await.is_err());
    assert_eq!(server.requests.lock().unwrap().len(), 1);
}
