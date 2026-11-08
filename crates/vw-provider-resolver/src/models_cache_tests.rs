use super::*;
use crate::models::native::ModelsCache;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

fn spawn_http(
    handler: impl FnOnce(TcpStream, String) + Send + 'static,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        stream.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            request.extend(byte);
        }
        handler(stream, String::from_utf8(request).unwrap());
    });
    (format!("http://{addr}/api.json"), handle)
}

fn spawn_response(status: &str, body: &str) -> (String, thread::JoinHandle<()>) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    spawn_http(move |mut stream, _| stream.write_all(response.as_bytes()).unwrap())
}

fn write_old_snapshot(path: &Path) -> (String, SystemTime) {
    let original = provider_map_json("last-valid");
    fs::write(path, &original).unwrap();
    let fetched_at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    fs::File::options().write(true).open(path).unwrap().set_modified(fetched_at).unwrap();
    (original, fetched_at)
}

async fn assert_last_valid_snapshot_kept(url: String, error_contains: &str) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("models.json");
    let (original, fetched_at) = write_old_snapshot(&path);
    let cache = Arc::new(ModelsCache::new(path.clone(), None, url.clone()));
    let previous = cache.snapshot().await;

    let result = cache.refresh(false).await;
    assert_eq!(result.status, ModelsRefreshStatus::Failed);
    let error = result.last_error.unwrap();
    assert!(error.contains(error_contains), "{error}");
    assert_eq!(result.fetched_at, Some(fetched_at));
    assert!(Arc::ptr_eq(&previous, &cache.snapshot().await));
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);

    let reopened = Arc::new(ModelsCache::new(path, None, url)).snapshot().await;
    assert_provider(&reopened.providers, "last-valid");
    assert!(reopened.providers.contains_key("shiju"));
    assert_eq!(reopened.fetched_at, Some(fetched_at));
}

async fn assert_bad_response_is_rejected(body: &str) {
    let (url, server) = spawn_response("200 OK", body);
    assert_last_valid_snapshot_kept(url, "usable model catalog").await;
    server.join().unwrap();
}

#[tokio::test]
async fn catalog_modelsdev_cache_keeps_last_valid_snapshot() {
    assert_bad_response_is_rejected("{").await;
}

#[tokio::test]
async fn catalog_modelsdev_cache_rejects_empty_provider_map() {
    assert_bad_response_is_rejected("{}").await;
}

#[tokio::test]
async fn catalog_modelsdev_cache_rejects_empty_wrapped_map() {
    assert_bad_response_is_rejected(r#"{"providers": {}}"#).await;
}

#[tokio::test]
async fn catalog_modelsdev_cache_rejects_empty_provider_array() {
    assert_bad_response_is_rejected("[]").await;
}

#[tokio::test]
async fn catalog_modelsdev_cache_rejects_unusable_model_entries() {
    let body = provider_map_json("invalid").replace("\"id\": \"invalid-model\"", "\"id\": \"\"");
    assert_bad_response_is_rejected(&body).await;
}

#[tokio::test]
async fn catalog_modelsdev_cache_preserves_snapshot_on_http_error() {
    let (url, server) = spawn_response("503 Service Unavailable", "offline");
    assert_last_valid_snapshot_kept(url, "503").await;
    server.join().unwrap();
}

#[tokio::test]
async fn catalog_modelsdev_cache_preserves_snapshot_when_offline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/api.json", listener.local_addr().unwrap());
    drop(listener);
    assert_last_valid_snapshot_kept(url, "Could not download").await;
}

#[tokio::test]
async fn catalog_modelsdev_cache_preserves_snapshot_on_incomplete_body() {
    let (url, server) = spawn_http(|mut stream, _| {
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\nshort")
            .unwrap();
    });
    assert_last_valid_snapshot_kept(url, "Could not download").await;
    server.join().unwrap();
}

#[tokio::test]
async fn catalog_modelsdev_cache_rejects_oversized_content_length() {
    let (url, server) = spawn_http(|mut stream, _| {
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            32 * 1024 * 1024 + 1
        )
        .unwrap();
    });
    assert_last_valid_snapshot_kept(url, "exceeds").await;
    server.join().unwrap();
}

#[tokio::test]
async fn catalog_modelsdev_cache_bounds_chunked_response() {
    let (url, server) = spawn_http(|mut stream, _| {
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            )
            .unwrap();
        let chunk = vec![b'x'; 1024 * 1024];
        for _ in 0..33 {
            if stream
                .write_all(b"100000\r\n")
                .and_then(|()| stream.write_all(&chunk))
                .and_then(|()| stream.write_all(b"\r\n"))
                .is_err()
            {
                return;
            }
        }
        let _ = stream.write_all(b"0\r\n\r\n");
    });
    assert_last_valid_snapshot_kept(url, "exceeds").await;
    server.join().unwrap();
}

#[tokio::test]
async fn catalog_modelsdev_cache_applies_timeout_to_body() {
    let (url, server) = spawn_http(|mut stream, _| {
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{")
            .unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(15))).unwrap();
        let mut byte = [0];
        assert_eq!(stream.read(&mut byte).unwrap(), 0);
    });
    assert_last_valid_snapshot_kept(url, "timed out").await;
    server.join().unwrap();
}

#[tokio::test]
async fn catalog_modelsdev_cache_rejects_cross_origin_redirect() {
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    target.set_nonblocking(true).unwrap();
    let destination = target.local_addr().unwrap();
    let (url, server) = spawn_http(move |mut stream, request| {
        assert!(request.to_ascii_lowercase().contains("authorization: basic "));
        write!(stream, "HTTP/1.1 302 Found\r\nLocation: http://{destination}/api.json\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
    });
    let url = url.replace("http://", "http://test-user:test-password@");
    assert_last_valid_snapshot_kept(url, "302").await;
    assert_eq!(target.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
    server.join().unwrap();
}

#[tokio::test]
async fn catalog_modelsdev_cache_atomically_replaces_snapshot_and_coalesces_refreshes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("models.json");
    let (old_body, old_time) = write_old_snapshot(&path);
    let mut open_reader = fs::File::open(&path).unwrap();
    let body = format!(r#"{{"data": {}}}"#, provider_map_json("remote"));
    let (url, server) = spawn_response("200 OK", &body);
    let cache = Arc::new(ModelsCache::new(path.clone(), None, url.clone()));
    let old = cache.snapshot().await;
    assert_provider(&old.providers, "last-valid");

    let (first, second) = tokio::join!(cache.refresh(false), cache.refresh(false));
    server.join().unwrap();
    assert_eq!(first.status, ModelsRefreshStatus::Updated);
    assert_eq!(second, first);
    assert_eq!(first.last_error, None);
    assert!(first.fetched_at.unwrap() > old_time);
    assert_eq!(fs::read_to_string(&path).unwrap(), body);
    let mut old_contents = String::new();
    open_reader.read_to_string(&mut old_contents).unwrap();
    assert_eq!(old_contents, old_body);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);

    let cached = cache.refresh(false).await;
    assert_eq!(cached.status, ModelsRefreshStatus::Fresh);
    assert_eq!(cached.fetched_at, first.fetched_at);
    let active = cache.snapshot().await;
    assert_provider(&active.providers, "remote");
    assert!(active.providers.contains_key("shiju"));
    let offline = Arc::new(ModelsCache::new(path, None, url)).snapshot().await;
    assert_provider(&offline.providers, "remote");
    assert_eq!(offline.fetched_at, first.fetched_at);
}

#[tokio::test]
async fn catalog_modelsdev_cache_coalesces_failed_refreshes_but_allows_retry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("models.json");
    let (_, old_time) = write_old_snapshot(&path);
    let (url, server) = spawn_response("503 Service Unavailable", "");
    let cache = Arc::new(ModelsCache::new(path, None, url));

    let (first, second) = tokio::join!(cache.refresh(false), cache.refresh(false));
    server.join().unwrap();
    assert_eq!(first, second);
    assert_eq!(first.status, ModelsRefreshStatus::Failed);
    assert_eq!(first.fetched_at, Some(old_time));
    assert!(first.last_error.unwrap().contains("503"));
    assert!(cache.refresh(false).await.last_error.unwrap().contains("Could not download"));
}

#[tokio::test]
async fn catalog_modelsdev_cache_keeps_custom_path_and_disable_fetch_semantics() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("models.json");
    let (_, old_time) = write_old_snapshot(&path);
    let custom_path = dir.path().join("custom.json");
    let custom_body = provider_map_json("custom");
    fs::write(&custom_path, &custom_body).unwrap();
    let (url, server) = spawn_response("200 OK", &provider_map_json("remote"));
    let cache = Arc::new(ModelsCache::new(path.clone(), Some(custom_path.clone()), url));

    let disabled = cache.refresh(true).await;
    assert_eq!(disabled.status, ModelsRefreshStatus::Disabled);
    assert_eq!(disabled.fetched_at, Some(old_time));
    let updated = cache.refresh(false).await;
    assert_eq!(updated.status, ModelsRefreshStatus::Updated);
    server.join().unwrap();
    assert!(fs::read_to_string(path).unwrap().contains("remote"));
    assert_eq!(fs::read_to_string(custom_path).unwrap(), custom_body);
    let snapshot = cache.snapshot().await;
    assert_provider(&snapshot.providers, "custom");
    assert_eq!(snapshot.fetched_at, None);
}

#[tokio::test]
async fn catalog_modelsdev_cache_uses_bundled_fallback_without_claiming_a_fetch() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("models.json");
    let cache = Arc::new(ModelsCache::new(path.clone(), None, String::new()));

    for invalid in ["{}", "{"] {
        fs::write(&path, invalid).unwrap();
        cache.invalidate();
        let snapshot = cache.snapshot().await;
        assert!(snapshot.providers.contains_key("openai"));
        assert!(snapshot.providers.contains_key("shiju"));
        assert_eq!(snapshot.fetched_at, None);
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
    }
    fs::remove_file(&path).unwrap();
    cache.invalidate();
    assert!(cache.snapshot().await.providers.contains_key("shiju"));
    assert!(!path.exists());
}

#[tokio::test]
async fn catalog_modelsdev_cache_reloads_disk_after_invalidation() {
    let dir = tempfile::tempdir().unwrap();
    let path: PathBuf = dir.path().join("models.json");
    write_old_snapshot(&path);
    let cache = Arc::new(ModelsCache::new(path.clone(), None, String::new()));
    let snapshot = cache.snapshot().await;

    fs::write(&path, provider_map_json("new")).unwrap();
    assert!(Arc::ptr_eq(&snapshot, &cache.snapshot().await));
    cache.invalidate();
    assert_provider(&cache.snapshot().await.providers, "new");
}

#[tokio::test]
async fn catalog_modelsdev_cache_keeps_bundled_snapshot_when_publish_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("models.json");
    fs::create_dir(&path).unwrap();
    let marker = path.join("existing");
    fs::write(&marker, "existing file").unwrap();
    let (url, server) = spawn_response("200 OK", &provider_map_json("remote"));
    let cache = Arc::new(ModelsCache::new(path, None, url));
    let snapshot = cache.snapshot().await;

    let result = cache.refresh(false).await;
    server.join().unwrap();
    assert_eq!(result.status, ModelsRefreshStatus::Failed);
    assert!(result.last_error.unwrap().contains("Could not replace models.json"));
    assert_eq!(result.fetched_at, None);
    assert!(Arc::ptr_eq(&snapshot, &cache.snapshot().await));
    assert_eq!(fs::read_to_string(marker).unwrap(), "existing file");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}
