use std::{io::{Cursor, Write}, sync::{Arc, Mutex}};

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};
use zip::{ZipWriter, write::SimpleFileOptions};

use super::{discovery::{DiscoveryError, discover_at}, repos::SkillRepo};

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
async fn cp12_discovery_success() {
    let zip = archive(&[
        ("tools/z/SKILL.md", "---\nname: Zulu\ndescription: useful\n---\nbody"),
        ("tools/a/SKILL.md", "\u{feff}---\nname: alpha\n---\nbody"),
        ("tools/a/examples/SKILL.md", "---\nname: nested example\n---"),
        ("README.md", "repository docs"),
    ]);
    let server = serve(vec![
        ("/repos/org/good", 200, br#"{"default_branch":"feature/skills"}"#.to_vec()),
        ("/org/good/zip/refs/heads/feature%2Fskills", 200, zip.clone()),
        ("/org/GOOD/zip/refs/heads/release", 200, zip),
        ("/org/bad/zip/refs/heads/main", 403, b"secret upstream body".to_vec()),
    ]).await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let report = discover_at(&client, &[
        repo("bad", "main", true), repo("good", "HEAD", true),
        repo("GOOD", "release", true), repo("disabled", "main", false),
    ], &server.endpoint, &server.endpoint).await;
    assert_eq!(report.skills.len(), 2);
    assert_eq!(report.skills[0].name, "alpha");
    assert_eq!(report.skills[1].name, "Zulu");
    assert_eq!(report.skills[1].description, "useful");
    assert_eq!(report.skills[0].key, "org/good:tools/a");
    assert_eq!(report.skills[0].directory, "tools/a");
    assert_eq!(report.skills[0].repo_branch, "feature/skills");
    assert_eq!(report.skills[0].readme_url.as_deref(), Some("https://github.com/org/good/blob/feature%2Fskills/tools/a/SKILL.md"));
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].name, "bad");
    assert_eq!(report.failures[0].error, DiscoveryError::Forbidden);
    assert_eq!(server.requests.lock().unwrap().len(), 4);
    assert!(!serde_json::to_string(&report).unwrap().contains("secret upstream body"));
}

#[tokio::test]
async fn cp12_discovery_rejects_without_side_effects() {
    let server = serve(vec![
        ("/org/broken/zip/refs/heads/main", 200, b"invalid ZIP secret".to_vec()),
        ("/org/missing/zip/refs/heads/main", 404, b"secret".to_vec()),
    ]).await;
    let repos = vec![repo("../escape", "main", true), repo("broken", "main", true),
        repo("missing", "main", true), repo("disabled", "invalid?branch", false)];
    let before = repos.clone();
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let report = discover_at(&client, &repos, &server.endpoint, &server.endpoint).await;
    assert_eq!(repos, before);
    assert!(report.skills.is_empty());
    assert_eq!(report.failures.iter().map(|failure| failure.error).collect::<Vec<_>>(),
        vec![DiscoveryError::InvalidRepository, DiscoveryError::InvalidArchive, DiscoveryError::NotFound]);
    assert_eq!(server.requests.lock().unwrap().len(), 2);
    assert!(!serde_json::to_string(&report).unwrap().contains("secret"));
    // Discovery has no store or installation destination parameter to mutate.
}

#[tokio::test]
async fn cp12_discovery_root_skill_and_metadata_fallback() {
    let server = serve(vec![("/org/root/zip/refs/heads/release", 200, archive(&[
        ("SKILL.md", "---\nname: [invalid\n---\ntext"),
        ("nested/SKILL.md", "nested example"),
    ]))]).await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let report = discover_at(&client, &[repo("root", "release", true)], &server.endpoint, &server.endpoint).await;
    assert!(report.failures.is_empty());
    assert_eq!(report.skills.len(), 1);
    assert_eq!(report.skills[0].name, "root");
    assert_eq!(report.skills[0].directory, "root");
    assert_eq!(report.skills[0].description, "");
    assert_eq!(report.skills[0].readme_url.as_deref(), Some("https://github.com/org/root/blob/release/SKILL.md"));
}

#[tokio::test]
async fn cp12_discovery_invalid_default_branch_is_isolated() {
    let server = serve(vec![
        ("/repos/org/bad-default", 200, br#"{"default_branch":"../unsafe"}"#.to_vec()),
        ("/org/healthy/zip/refs/heads/stable", 200, archive(&[("SKILL.md", "plain markdown")])),
    ]).await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let disabled = discover_at(&client, &[repo("disabled", "", false)], &server.endpoint, &server.endpoint).await;
    assert_eq!(disabled, Default::default());
    assert!(server.requests.lock().unwrap().is_empty());
    let report = discover_at(&client, &[repo("bad-default", "", true), repo("healthy", "stable", true)], &server.endpoint, &server.endpoint).await;
    assert_eq!(report.skills.len(), 1);
    assert_eq!(report.skills[0].name, "healthy");
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].error, DiscoveryError::InvalidResponse);
    assert_eq!(server.requests.lock().unwrap().len(), 2);
}
