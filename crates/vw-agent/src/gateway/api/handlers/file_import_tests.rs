use super::*;
use axum::{
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

async fn request(
    peer: Option<&str>,
    origin: Option<&str>,
    project: &str,
    target: &str,
    sources: Vec<String>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/files/import-local")
        .header("content-type", "application/json");
    if let Some(peer) = peer {
        builder = builder.extension(ConnectInfo(peer.parse::<std::net::SocketAddr>().unwrap()));
    }
    if let Some(origin) = origin {
        builder = builder.header("origin", origin);
    }
    let response = router::<()>().oneshot(builder.body(Body::from(serde_json::json!({"project_id":project,"target_directory":target,"source_paths":sources}).to_string())).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or_default())
}

#[tokio::test]
async fn local_import_rejects_untrusted_connections_and_origins() {
    for (peer, origin) in [
        (None, None),
        (Some("192.0.2.1:1"), None),
        (Some("127.0.0.1:1"), Some("https://evil.example")),
        (Some("127.0.0.1:1"), Some("http://localhost:123@evil.example")),
    ] {
        assert_eq!(
            request(peer, origin, "missing", "", vec!["/missing".into()]).await.0,
            StatusCode::FORBIDDEN
        );
    }
}

#[tokio::test]
async fn local_import_copies_binary_hidden_and_empty_directories_and_preserves_source() {
    let root = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let project = super::file_tests::RegisteredFileProject::new(root.path());
    std::fs::create_dir_all(source.path().join("目录/empty/nested")).unwrap();
    std::fs::write(source.path().join("目录/.binary"), [0, 255, 7]).unwrap();
    let (status, body) = request(
        Some("127.0.0.1:1"),
        None,
        &project.0,
        "",
        vec![source.path().join("目录").to_str().unwrap().into()],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"][0]["ok"], true);
    assert!(root.path().join("目录/empty/nested").is_dir());
    assert_eq!(std::fs::read(root.path().join("目录/.binary")).unwrap(), [0, 255, 7]);
    assert!(source.path().join("目录/.binary").exists());
    let (_, body) = request(
        Some("[::1]:1"),
        None,
        &project.0,
        "",
        vec![source.path().join("目录").to_str().unwrap().into()],
    )
    .await;
    assert_eq!(body["items"][0]["ok"], false);
    assert_eq!(std::fs::read(root.path().join("目录/.binary")).unwrap(), [0, 255, 7]);
}

#[tokio::test]
async fn local_import_rejects_invalid_targets_projects_and_item_counts() {
    let root = tempfile::tempdir().unwrap();
    let project = super::file_tests::RegisteredFileProject::new(root.path());
    for target in ["../escape", "/tmp", "missing"] {
        assert_eq!(
            request(Some("127.0.0.1:1"), None, &project.0, target, vec!["/missing".into()]).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        request(Some("127.0.0.1:1"), None, "missing", "", vec!["/missing".into()]).await.0,
        StatusCode::NOT_FOUND
    );
    for sources in [vec![], vec!["/missing".into(); 129]] {
        assert_eq!(
            request(Some("127.0.0.1:1"), None, &project.0, "", sources).await.0,
            StatusCode::BAD_REQUEST
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn local_import_recursive_failure_cleans_staging_and_continues() {
    let root = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let project = super::file_tests::RegisteredFileProject::new(root.path());
    std::fs::create_dir(source.path().join("bad")).unwrap();
    std::fs::write(source.path().join("bad/retained"), "source").unwrap();
    std::os::unix::fs::symlink("retained", source.path().join("bad/link")).unwrap();
    std::fs::write(source.path().join("good"), "good").unwrap();
    let (_, body) = request(
        Some("127.0.0.1:1"),
        None,
        &project.0,
        "",
        vec![
            source.path().join("bad").to_str().unwrap().into(),
            source.path().join("good").to_str().unwrap().into(),
            "relative".into(),
        ],
    )
    .await;
    assert_eq!(body["items"][0]["ok"], false);
    assert_eq!(body["items"][1]["ok"], true);
    assert_eq!(body["items"][2]["ok"], false);
    assert!(!root.path().join("bad").exists());
    assert_eq!(std::fs::read_to_string(source.path().join("bad/retained")).unwrap(), "source");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    std::os::unix::fs::symlink(source.path(), root.path().join("alias")).unwrap();
    assert_eq!(
        request(
            Some("127.0.0.1:1"),
            None,
            &project.0,
            "alias",
            vec![source.path().join("good").to_str().unwrap().into()]
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn local_import_rejects_copy_into_source() {
    let root = tempfile::tempdir().unwrap();
    let project = super::file_tests::RegisteredFileProject::new(root.path());
    std::fs::create_dir_all(root.path().join("folder/child")).unwrap();
    let (_, body) = request(
        Some("127.0.0.1:1"),
        None,
        &project.0,
        "folder/child",
        vec![root.path().join("folder").to_str().unwrap().into()],
    )
    .await;
    assert_eq!(body["items"][0]["ok"], false);
    assert_eq!(std::fs::read_dir(root.path().join("folder/child")).unwrap().count(), 0);
}

#[test]
fn local_import_exclusive_publication_preserves_raced_directory_and_file() {
    for directory in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("first");
        let second = root.path().join("second");
        let destination = root.path().join("destination");
        for (path, content) in [(&first, "first"), (&second, "second")] {
            if directory {
                std::fs::create_dir(path).unwrap();
                std::fs::write(path.join("data"), content).unwrap();
            } else {
                std::fs::write(path, content).unwrap();
            }
        }
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = [&first, &second]
                .into_iter()
                .map(|source| {
                    let barrier = barrier.clone();
                    let destination = &destination;
                    scope.spawn(move || {
                        barrier.wait();
                        super::file_import::publish_no_replace(source, destination)
                    })
                })
                .collect();
            handles.into_iter().map(|handle| handle.join().unwrap()).collect::<Vec<_>>()
        });
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(first.exists() as u8 + second.exists() as u8, 1);
        let content =
            std::fs::read_to_string(if directory { destination.join("data") } else { destination })
                .unwrap();
        assert!(content == "first" || content == "second");
    }
}

#[tokio::test]
async fn local_import_limits_json_metadata_body() {
    let body = "x".repeat(crate::app::agent::gateway::MAX_BODY_SIZE + 1);
    let response = router::<()>()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/files/import-local")
                .header("content-type", "application/json")
                .extension(ConnectInfo("127.0.0.1:1".parse::<std::net::SocketAddr>().unwrap()))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[cfg(unix)]
#[tokio::test]
async fn local_import_rejects_special_files_and_top_level_links() {
    use std::os::unix::ffi::OsStrExt;
    let root = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let project = super::file_tests::RegisteredFileProject::new(root.path());
    std::fs::write(source.path().join("file"), "retained").unwrap();
    std::os::unix::fs::symlink("file", source.path().join("link")).unwrap();
    let fifo = source.path().join("fifo");
    let c_path = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
    let (_, body) = request(
        Some("127.0.0.1:1"),
        None,
        &project.0,
        "",
        vec![
            source.path().join("link").to_str().unwrap().into(),
            fifo.to_str().unwrap().into(),
            format!("{}/", source.path().join("link").display()),
        ],
    )
    .await;
    assert_eq!(body["items"][0]["ok"], false);
    assert_eq!(body["items"][1]["ok"], false);
    assert_eq!(body["items"][2]["ok"], false);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    assert_eq!(std::fs::read_to_string(source.path().join("file")).unwrap(), "retained");
}
