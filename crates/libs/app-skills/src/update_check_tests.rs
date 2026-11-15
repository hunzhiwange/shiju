use std::{collections::BTreeMap, io::{Cursor, Write}, sync::{Arc, Mutex}};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};
use zip::{ZipWriter, write::SimpleFileOptions};
use super::{local_install::LocalSkill, repo_install::RepoSkillSource, update_check::check_at};

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


fn skill(directory: &str) -> LocalSkill {
    LocalSkill { directory: directory.into(), name: directory.into(), description: None, source: "/unused".into() }
}

fn source(repo: &str) -> RepoSkillSource {
    RepoSkillSource { owner: "org".into(), name: repo.into(), branch: "main".into(), directory: "demo".into() }
}

#[tokio::test]
async fn cp12_update_check_success() {
    let server = serve(vec![("/org/good/zip/refs/heads/main", 200,
        archive(&[("demo/SKILL.md", "remote")]))]).await;
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("changed")).unwrap();
    std::fs::write(root.path().join("changed/SKILL.md"), "local").unwrap();
    std::fs::create_dir(root.path().join("same")).unwrap();
    std::fs::write(root.path().join("same/SKILL.md"), "remote").unwrap();
    let skills = [skill("changed"), skill("same"), skill("missing"), skill("local")];
    let sources = ["changed", "same", "missing"].into_iter()
        .map(|name| (name.to_owned(), source("good"))).collect::<BTreeMap<_, _>>();
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let report = check_at(&client, root.path(), &skills, &sources, &server.endpoint).await;
    assert_eq!(report.updates.len(), 2);
    assert!(report.updates.iter().find(|update| update.directory == "missing").unwrap().current_hash.is_none());
    assert_eq!(report.unchanged, ["same"]);
    assert_eq!(report.without_repository, ["local"]);
    assert!(report.failures.is_empty());
    assert!(report.skill_failures.is_empty());
    assert_eq!(server.requests.lock().unwrap().len(), 1);
    assert_eq!(std::fs::read_to_string(root.path().join("changed/SKILL.md")).unwrap(), "local");
}

#[tokio::test]
async fn cp12_update_check_rejects_without_side_effects() {
    let server = serve(vec![("/org/bad/zip/refs/heads/main", 403, b"private".to_vec()),
        ("/org/good/zip/refs/heads/main", 200, archive(&[("demo/SKILL.md", "remote")]))]).await;
    let root = tempfile::tempdir().unwrap();
    let skills = [skill("bad"), skill("good")];
    let sources = [("bad".into(), source("bad")), ("good".into(), source("good"))].into_iter().collect();
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let report = check_at(&client, root.path(), &skills, &sources, &server.endpoint).await;
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].error, super::discovery::DiscoveryError::Forbidden);
    assert_eq!(report.updates.len(), 1);
    assert_eq!(report.updates[0].directory, "good");
    assert!(report.unchanged.is_empty());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
