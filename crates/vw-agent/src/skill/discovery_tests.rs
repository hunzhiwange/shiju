//! 验证旧版技能发现过程的目录优先级、过滤与去重行为。

use super::*;

#[cfg(not(target_arch = "wasm32"))]
fn spawn_http_server(routes: Vec<(String, u16, String)>) -> (String, std::thread::JoinHandle<()>) {
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let request_count = routes.len();
    let routes = routes
        .into_iter()
        .map(|(path, status, body)| (path.to_string(), (status, body)))
        .collect::<HashMap<_, _>>();

    let handle = std::thread::spawn(move || {
        for _ in 0..request_count {
            let Ok((mut stream, _)) = listener.accept() else {
                continue;
            };
            let mut buf = [0_u8; 2048];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");
            let (status, body) =
                routes.get(path).cloned().unwrap_or((404, "not found".to_string()));
            let reason = if status == 200 { "OK" } else { "Not Found" };
            let response = format!(
                "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status,
                reason,
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    (base_url, handle)
}

#[test]
fn index_deserializes_skill_file_lists() {
    let index: Index = serde_json::from_str(
        r#"{"skills":[{"name":"rust","description":"Rust helper","files":["SKILL.md"]}]}"#,
    )
    .unwrap();

    assert_eq!(index.skills.len(), 1);
    assert_eq!(index.skills[0].name, "rust");
    assert_eq!(index.skills[0].files, vec!["SKILL.md"]);
}

#[test]
fn index_allows_missing_description_and_rejects_missing_files() {
    let index: Index =
        serde_json::from_str(r#"{"skills":[{"name":"rust","files":["SKILL.md"]}]}"#).unwrap();

    assert_eq!(index.skills[0].description, None);
    assert!(serde_json::from_str::<Index>(r#"{"skills":[{"name":"rust"}]}"#).is_err());
}

#[test]
fn dir_returns_skills_subdirectory_of_global_cache() {
    assert_eq!(dir(), global::paths().cache.join("skills"));
}

#[tokio::test]
async fn pull_empty_url_returns_no_paths() {
    assert!(pull("  ").await.is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn download_if_missing_treats_existing_file_as_success_without_network() {
    let dir = tempfile::tempdir().expect("temp dir");
    let dest = dir.path().join("skill/SKILL.md");
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    std::fs::write(&dest, "already here").unwrap();

    assert!(download_if_missing("http://127.0.0.1:9/nope", &dest).await);
    assert_eq!(std::fs::read_to_string(dest).unwrap(), "already here");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn download_if_missing_writes_success_and_reports_http_errors() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (base_url, handle) = spawn_http_server(vec![
        ("/ok.md".to_string(), 200, "downloaded".to_string()),
        ("/missing.md".to_string(), 404, "missing".to_string()),
    ]);

    let ok_dest = dir.path().join("ok.md");
    let missing_dest = dir.path().join("missing.md");

    assert!(download_if_missing(&format!("{base_url}/ok.md"), &ok_dest).await);
    assert_eq!(std::fs::read_to_string(&ok_dest).unwrap(), "downloaded");
    assert!(!download_if_missing(&format!("{base_url}/missing.md"), &missing_dest).await);
    assert!(!missing_dest.exists());

    handle.join().expect("server thread");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn download_if_missing_reports_write_failure_for_directory_destination() {
    let dir = tempfile::tempdir().expect("temp dir");
    let dest = dir.path().join("existing-directory");
    std::fs::create_dir_all(&dest).unwrap();
    let (base_url, handle) =
        spawn_http_server(vec![("/file.md".to_string(), 200, "downloaded".to_string())]);

    assert!(!download_if_missing(&format!("{base_url}/file.md"), &dest).await);
    assert!(dest.is_dir());

    handle.join().expect("server thread");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn download_if_missing_rejects_invalid_url_without_creating_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let dest = dir.path().join("skill/SKILL.md");

    assert!(!download_if_missing("not a url", &dest).await);
    assert!(!dest.exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn pull_rejects_invalid_repository_url() {
    assert!(pull("not a url").await.is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn pull_rejects_unsuccessful_and_malformed_indexes() {
    let (error_base_url, error_handle) =
        spawn_http_server(vec![("/index.json".to_string(), 503, "unavailable".to_string())]);

    assert!(pull(&error_base_url).await.is_empty());
    error_handle.join().expect("error server thread");

    let (invalid_base_url, invalid_handle) =
        spawn_http_server(vec![("/index.json".to_string(), 200, "{".to_string())]);

    assert!(pull(&invalid_base_url).await.is_empty());
    invalid_handle.join().expect("invalid server thread");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn pull_downloads_valid_index_entries_and_skips_invalid_skills() {
    let skill_name = format!("served-skill-{}", std::process::id());
    let skill_root = dir().join(&skill_name);
    let _ = std::fs::remove_dir_all(&skill_root);

    let index = serde_json::json!({
        "skills": [
            {
                "name": skill_name,
                "description": "served over http",
                "files": ["SKILL.md", "nested/info.md", " ", "missing.md"]
            },
            { "name": "   ", "files": ["SKILL.md"] },
            { "name": "empty-files", "files": [] }
        ]
    })
    .to_string();
    let (base_url, handle) = spawn_http_server(vec![
        ("/index.json".to_string(), 200, index),
        (format!("/{skill_name}/SKILL.md"), 200, "# Served\n\nUse it.".to_string()),
        (format!("/{skill_name}/nested/info.md"), 200, "nested".to_string()),
        (format!("/{skill_name}/missing.md"), 404, "missing".to_string()),
    ]);

    let paths = pull(&base_url).await;

    assert_eq!(paths, vec![skill_root.clone()]);
    assert_eq!(
        std::fs::read_to_string(skill_root.join("SKILL.md")).unwrap(),
        "# Served\n\nUse it."
    );
    assert_eq!(std::fs::read_to_string(skill_root.join("nested/info.md")).unwrap(), "nested");
    assert!(!skill_root.join("missing.md").exists());

    let _ = std::fs::remove_dir_all(&skill_root);
    handle.join().expect("server thread");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn pull_trims_base_and_file_paths_but_excludes_skill_without_manifest() {
    let skill_name = format!("manifestless-skill-{}", std::process::id());
    let skill_root = dir().join(&skill_name);
    let _ = std::fs::remove_dir_all(&skill_root);
    let index = serde_json::json!({
        "skills": [{
            "name": skill_name,
            "files": [" docs/info.md "]
        }]
    })
    .to_string();
    let (base_url, handle) = spawn_http_server(vec![
        ("/index.json".to_string(), 200, index),
        (format!("/{skill_name}/docs/info.md"), 200, "details".to_string()),
    ]);

    let paths = pull(&format!("  {base_url}/  ")).await;

    assert!(paths.is_empty());
    assert_eq!(std::fs::read_to_string(skill_root.join("docs/info.md")).unwrap(), "details");

    let _ = std::fs::remove_dir_all(&skill_root);
    handle.join().expect("server thread");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn pull_preserves_cached_manifest() {
    let skill_name = format!("cached-skill-{}", std::process::id());
    let skill_root = dir().join(&skill_name);
    let manifest = skill_root.join("SKILL.md");
    let _ = std::fs::remove_dir_all(&skill_root);
    std::fs::create_dir_all(&skill_root).unwrap();
    std::fs::write(&manifest, "cached").unwrap();
    let index = serde_json::json!({
        "skills": [{
            "name": skill_name,
            "files": ["SKILL.md"]
        }]
    })
    .to_string();
    let (base_url, handle) = spawn_http_server(vec![("/index.json".to_string(), 200, index)]);

    let paths = pull(&base_url).await;

    assert_eq!(paths, vec![skill_root.clone()]);
    assert_eq!(std::fs::read_to_string(&manifest).unwrap(), "cached");

    let _ = std::fs::remove_dir_all(&skill_root);
    handle.join().expect("server thread");
}
