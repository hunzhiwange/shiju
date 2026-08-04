use serde_json::json;

#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::net::TcpListener;
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;
#[cfg(not(target_arch = "wasm32"))]
use std::thread::JoinHandle;

#[cfg(not(target_arch = "wasm32"))]
fn update_env_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

#[cfg(not(target_arch = "wasm32"))]
struct EnvGuard {
    key: &'static str,
    previous: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, previous }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct TestServer {
    url: String,
    handle: Option<JoinHandle<String>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl TestServer {
    fn respond(status: &str, content_type: &str, body: Vec<u8>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let status = status.to_string();
        let content_type = content_type.to_string();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let bytes_read = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..bytes_read]).to_string();
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.write_all(&body).unwrap();
            request
        });
        Self { url, handle: Some(handle) }
    }

    fn join(&mut self) -> String {
        self.handle.take().unwrap().join().unwrap()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[test]
fn parse_release_manifest_accepts_github_payload() {
    let manifest = super::parse_release_manifest(json!({
        "tag_name": "v1.2.3",
        "assets": [
            {
                "name": "vibewindow-aarch64-apple-darwin.tar.gz",
                "browser_download_url": "https://example.com/vibewindow.tar.gz"
            }
        ]
    }))
    .unwrap();

    assert_eq!(manifest.version, "v1.2.3");
    assert_eq!(manifest.assets.len(), 1);
    assert_eq!(manifest.assets[0].name, "vibewindow-aarch64-apple-darwin.tar.gz");
    assert_eq!(manifest.assets[0].download_url, "https://example.com/vibewindow.tar.gz");
    assert_eq!(manifest.assets[0].binary_name, None);
    assert_eq!(manifest.assets[0].target, None);
}

#[test]
fn parse_release_manifest_accepts_custom_payload_and_infers_asset_name() {
    let manifest = super::parse_release_manifest(json!({
        "version": "1.2.3",
        "assets": [
            {
                "url": "https://example.com/releases/vibewindow.bin",
                "target": "aarch64-apple-darwin",
                "binary_name": "vibewindow-custom"
            },
            {
                "name": "named.bin",
                "url": "https://example.com/download"
            }
        ]
    }))
    .unwrap();

    assert_eq!(manifest.version, "1.2.3");
    assert_eq!(manifest.assets[0].name, "vibewindow.bin");
    assert_eq!(manifest.assets[0].target.as_deref(), Some("aarch64-apple-darwin"));
    assert_eq!(manifest.assets[0].binary_name.as_deref(), Some("vibewindow-custom"));
    assert_eq!(manifest.assets[1].name, "named.bin");
}

#[test]
fn parse_release_manifest_uses_default_name_for_empty_custom_url_tail() {
    let manifest = super::parse_release_manifest(json!({
        "version": "1.2.3",
        "assets": [
            {
                "url": "https://example.com/releases/"
            }
        ]
    }))
    .unwrap();

    assert_eq!(manifest.assets[0].name, "vibewindow-update.bin");
}

#[test]
fn parse_release_manifest_rejects_malformed_github_payload() {
    let error = super::parse_release_manifest(json!({
        "tag_name": "v1.2.3",
        "assets": [
            {
                "name": "missing-download-url"
            }
        ]
    }))
    .unwrap_err();

    assert!(error.to_string().contains("Failed to parse GitHub release payload"));
}

#[test]
fn parse_release_manifest_rejects_malformed_custom_payload() {
    let error = super::parse_release_manifest(json!({
        "version": "1.2.3",
        "assets": [
            {}
        ]
    }))
    .unwrap_err();

    assert!(error.to_string().contains("Failed to parse custom update payload"));
}

#[test]
fn parse_release_manifest_rejects_unsupported_payload() {
    let error = super::parse_release_manifest(json!({ "assets": [] })).unwrap_err();

    assert!(error.to_string().contains("Unsupported update payload"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn current_release_api_uses_default_for_missing_or_blank_env() {
    let _lock = update_env_lock().blocking_lock();
    let _guard = EnvGuard::set(super::APP_UPDATE_API_ENV, "   ");

    assert_eq!(
        super::current_release_api(),
        "https://api.github.com/repos/hunzhiwange/vibewindow/releases/latest"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn current_release_api_trims_custom_env() {
    let _lock = update_env_lock().blocking_lock();
    let _guard = EnvGuard::set(super::APP_UPDATE_API_ENV, " https://updates.example/latest ");

    assert_eq!(super::current_release_api(), "https://updates.example/latest");
}

#[test]
fn normalize_version_trims_outer_space_and_leading_v() {
    assert_eq!(super::normalize_version(" v1.2.3 "), "1.2.3");
    assert_eq!(super::normalize_version("1.2.3"), "1.2.3");
    assert_eq!(super::normalize_version("vv1.2.3"), "1.2.3");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn binary_and_archive_names_match_platform_conventions() {
    let target = super::get_target_triple().unwrap();
    let binary_name = super::get_binary_name();
    let archive_name = super::get_archive_name(&target);

    if cfg!(windows) {
        assert_eq!(binary_name, "shiju.exe");
        assert!(archive_name.ends_with(".zip"));
    } else {
        assert_eq!(binary_name, "shiju");
        assert!(archive_name.ends_with(".tar.gz"));
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn find_asset_for_platform_prefers_explicit_target() {
    let target = super::get_target_triple().unwrap();
    let archive_name = super::get_archive_name(&target);
    let release = super::ReleaseManifest {
        version: "1.2.3".to_string(),
        assets: vec![
            super::ReleaseAsset {
                name: archive_name,
                download_url: "https://example.com/archive.tar.gz".to_string(),
                binary_name: None,
                target: None,
            },
            super::ReleaseAsset {
                name: "explicit.bin".to_string(),
                download_url: "https://example.com/explicit.bin".to_string(),
                binary_name: Some("custom-bin".to_string()),
                target: Some(target),
            },
        ],
    };

    let asset = super::find_asset_for_platform(&release).unwrap();

    assert_eq!(asset.name, "explicit.bin");
    assert_eq!(asset.binary_name.as_deref(), Some("custom-bin"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn find_asset_for_platform_falls_back_to_name_contains_target() {
    let target = super::get_target_triple().unwrap();
    let release = super::ReleaseManifest {
        version: "1.2.3".to_string(),
        assets: vec![super::ReleaseAsset {
            name: format!("custom-{target}.bin"),
            download_url: "https://example.com/archive.bin".to_string(),
            binary_name: None,
            target: None,
        }],
    };

    let asset = super::find_asset_for_platform(&release).unwrap();

    assert_eq!(asset.name, format!("custom-{target}.bin"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn find_asset_for_platform_falls_back_to_download_url() {
    let target = super::get_target_triple().unwrap();
    let release = super::ReleaseManifest {
        version: "1.2.3".to_string(),
        assets: vec![super::ReleaseAsset {
            name: "release.bin".to_string(),
            download_url: format!("https://example.com/releases/{target}/release.bin"),
            binary_name: None,
            target: None,
        }],
    };

    let asset = super::find_asset_for_platform(&release).unwrap();

    assert_eq!(asset.name, "release.bin");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn find_asset_for_platform_reports_missing_asset() {
    let target = super::get_target_triple().unwrap();
    let release = super::ReleaseManifest { version: "1.2.3".to_string(), assets: Vec::new() };

    let error = super::find_asset_for_platform(&release).unwrap_err();

    assert!(error.to_string().contains(&format!("No release asset found for platform {target}")));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn find_asset_for_platform_falls_back_to_archive_name() {
    let target = super::get_target_triple().unwrap();
    let archive_name = super::get_archive_name(&target);
    let release = super::ReleaseManifest {
        version: "1.2.3".to_string(),
        assets: vec![super::ReleaseAsset {
            name: archive_name.clone(),
            download_url: "https://example.com/archive.tar.gz".to_string(),
            binary_name: None,
            target: None,
        }],
    };

    let asset = super::find_asset_for_platform(&release).unwrap();

    assert_eq!(asset.name, archive_name);
}

#[cfg(all(not(windows), not(target_arch = "wasm32")))]
#[test]
fn candidate_binary_names_prioritizes_explicit_name_and_removes_duplicates() {
    let current_exe = std::path::Path::new("/tmp/vibewindow-custom");

    let names = super::candidate_binary_names(current_exe, Some(" vibewindow-custom "));

    assert_eq!(names, vec!["vibewindow-custom", "shiju", "vw-webview"]);
}

#[cfg(all(not(windows), not(target_arch = "wasm32")))]
#[test]
fn candidate_binary_names_uses_current_exe_then_defaults() {
    let current_exe = std::path::Path::new("/tmp/custom-cli");

    let names = super::candidate_binary_names(current_exe, Some("  "));

    assert_eq!(names, vec!["custom-cli", "shiju", "vw-webview"]);
}

#[cfg(all(not(windows), not(target_arch = "wasm32")))]
#[test]
fn candidate_binary_names_uses_defaults_when_current_path_has_no_file_name() {
    let names = super::candidate_binary_names(std::path::Path::new("/"), None);

    assert_eq!(names, vec!["shiju", "vw-webview"]);
}

#[cfg(all(not(windows), not(target_arch = "wasm32")))]
#[test]
fn push_unique_ignores_none_and_duplicates() {
    let mut values = vec!["vibewindow".to_string()];

    super::push_unique(&mut values, None);
    super::push_unique(&mut values, Some("vibewindow".to_string()));
    super::push_unique(&mut values, Some("vw-webview".to_string()));

    assert_eq!(values, vec!["vibewindow", "vw-webview"]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn find_extracted_binary_prefers_named_binary_over_first_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let nested_dir = temp_dir.path().join("nested");
    std::fs::create_dir(&nested_dir).unwrap();
    std::fs::write(temp_dir.path().join("readme.txt"), "not executable").unwrap();
    std::fs::write(nested_dir.join("vibewindow"), "binary").unwrap();

    let binary =
        super::find_extracted_binary(temp_dir.path(), &["vibewindow".to_string()]).unwrap();

    assert_eq!(binary.file_name().and_then(|name| name.to_str()), Some("vibewindow"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn find_extracted_binary_returns_first_file_when_no_preferred_name_matches() {
    let temp_dir = tempfile::tempdir().unwrap();
    let first = temp_dir.path().join("readme.txt");
    std::fs::write(&first, "not executable").unwrap();

    let binary =
        super::find_extracted_binary(temp_dir.path(), &["vibewindow".to_string()]).unwrap();

    assert_eq!(binary, first);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn find_extracted_binary_reports_empty_archive() {
    let temp_dir = tempfile::tempdir().unwrap();

    let error = super::find_extracted_binary(temp_dir.path(), &[]).unwrap_err();

    assert!(error.to_string().contains("Binary not found in downloaded archive"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn find_extracted_binary_reports_missing_root() {
    let temp_dir = tempfile::tempdir().unwrap();
    let missing = temp_dir.path().join("missing");

    let error = super::find_extracted_binary(&missing, &[]).unwrap_err();

    assert!(error.to_string().contains("Failed to read"));
    assert!(error.to_string().contains(&missing.display().to_string()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn ensure_executable_sets_unix_mode() {
    let temp_dir = tempfile::tempdir().unwrap();
    let binary = temp_dir.path().join("vibewindow");
    std::fs::write(&binary, "binary").unwrap();

    super::ensure_executable(&binary).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&binary).unwrap().permissions().mode() & 0o777, 0o755);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn ensure_executable_reports_missing_path() {
    let temp_dir = tempfile::tempdir().unwrap();
    let missing = temp_dir.path().join("missing-vibewindow");

    let error = super::ensure_executable(&missing).unwrap_err();

    assert!(error.to_string().contains("Failed to set executable permissions"));
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn find_extracted_binary_skips_non_utf8_file_names() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let temp_dir = tempfile::tempdir().unwrap();
    std::fs::write(temp_dir.path().join(OsString::from_vec(vec![0xff, b'b'])), "skip").unwrap();
    let binary = temp_dir.path().join("vibewindow");
    std::fs::write(&binary, "binary").unwrap();

    let found = super::find_extracted_binary(temp_dir.path(), &["vibewindow".to_string()]).unwrap();

    assert_eq!(found, binary);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn fetch_latest_version_reads_custom_update_endpoint() {
    let _lock = update_env_lock().lock().await;
    let mut server = TestServer::respond(
        "200 OK",
        "application/json",
        br#"{"version":"9.9.9","assets":[]}"#.to_vec(),
    );
    let _guard = EnvGuard::set(super::APP_UPDATE_API_ENV, &server.url);

    let version = super::fetch_latest_version().await.unwrap();
    let request = server.join();

    assert_eq!(version, "9.9.9");
    assert!(request.starts_with("GET / HTTP/1.1"));
    assert!(request.contains("accept: application/vnd.github+json"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn check_for_update_returns_none_for_current_version() {
    let _lock = update_env_lock().lock().await;
    let body = format!(r#"{{"version":"v{}","assets":[]}}"#, super::current_version());
    let mut server = TestServer::respond("200 OK", "application/json", body.into_bytes());
    let _guard = EnvGuard::set(super::APP_UPDATE_API_ENV, &server.url);

    let update = super::check_for_update().await.unwrap();
    let _ = server.join();

    assert_eq!(update, None);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn check_for_update_formats_available_version() {
    let _lock = update_env_lock().lock().await;
    let mut server = TestServer::respond(
        "200 OK",
        "application/json",
        br#"{"version":"v999.0.0","assets":[]}"#.to_vec(),
    );
    let _guard = EnvGuard::set(super::APP_UPDATE_API_ENV, &server.url);

    let update = super::check_for_update().await.unwrap();
    let _ = server.join();

    assert_eq!(update, Some(format!("v999.0.0 (current: {})", super::current_version())));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn fetch_release_manifest_reports_http_status_errors() {
    let _lock = update_env_lock().lock().await;
    let mut server = TestServer::respond("503 Service Unavailable", "text/plain", b"down".to_vec());
    let _guard = EnvGuard::set(super::APP_UPDATE_API_ENV, &server.url);

    let error = super::fetch_release_manifest().await.unwrap_err();
    let _ = server.join();

    assert!(error.to_string().contains("Update API returned status: 503"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn fetch_release_manifest_reports_json_errors() {
    let _lock = update_env_lock().lock().await;
    let mut server = TestServer::respond("200 OK", "application/json", b"not-json".to_vec());
    let _guard = EnvGuard::set(super::APP_UPDATE_API_ENV, &server.url);

    let error = super::fetch_release_manifest().await.unwrap_err();
    let _ = server.join();

    assert!(error.to_string().contains("Failed to parse update payload"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn download_binary_stages_direct_binary_with_explicit_name() {
    let mut server = TestServer::respond("200 OK", "application/octet-stream", b"binary".to_vec());
    let temp_dir = tempfile::tempdir().unwrap();
    let asset = super::ReleaseAsset {
        name: "download.bin".to_string(),
        download_url: server.url.clone(),
        binary_name: Some("custom-bin".to_string()),
        target: None,
    };

    let binary =
        super::download_binary(&asset, temp_dir.path(), &["preferred".to_string()]).await.unwrap();
    let _ = server.join();

    assert_eq!(binary.file_name().and_then(|name| name.to_str()), Some("custom-bin"));
    assert_eq!(std::fs::read_to_string(binary).unwrap(), "binary");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn download_binary_reports_download_progress() {
    let body = b"progress-binary".to_vec();
    let mut server = TestServer::respond("200 OK", "application/octet-stream", body.clone());
    let temp_dir = tempfile::tempdir().unwrap();
    let asset = super::ReleaseAsset {
        name: "download.bin".to_string(),
        download_url: server.url.clone(),
        binary_name: Some("custom-bin".to_string()),
        target: None,
    };
    let mut progress_events = Vec::new();

    let binary = super::download_binary_with_progress(
        &asset,
        temp_dir.path(),
        &["preferred".to_string()],
        |progress| progress_events.push(progress),
    )
    .await
    .unwrap();
    let _ = server.join();

    assert_eq!(std::fs::read_to_string(binary).unwrap(), "progress-binary");
    assert!(progress_events.len() >= 2);
    assert_eq!(progress_events.first().unwrap().downloaded_bytes, 0);
    let last = progress_events.last().unwrap();
    assert_eq!(last.asset_name, "download.bin");
    assert_eq!(last.download_url, asset.download_url);
    assert_eq!(last.downloaded_bytes, body.len() as u64);
    assert_eq!(last.total_bytes, Some(body.len() as u64));
    assert!(last.bytes_per_second >= 0.0);
    assert!(last.average_bytes_per_second >= 0.0);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn download_binary_keeps_direct_binary_when_archive_name_matches_destination() {
    let mut server = TestServer::respond("200 OK", "application/octet-stream", b"binary".to_vec());
    let temp_dir = tempfile::tempdir().unwrap();
    let asset = super::ReleaseAsset {
        name: "preferred-bin".to_string(),
        download_url: server.url.clone(),
        binary_name: None,
        target: None,
    };

    let binary = super::download_binary(&asset, temp_dir.path(), &["preferred-bin".to_string()])
        .await
        .unwrap();
    let _ = server.join();

    assert_eq!(binary, temp_dir.path().join("preferred-bin"));
    assert_eq!(std::fs::read_to_string(binary).unwrap(), "binary");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn download_binary_reports_status_errors() {
    let mut server = TestServer::respond("404 Not Found", "text/plain", b"missing".to_vec());
    let temp_dir = tempfile::tempdir().unwrap();
    let asset = super::ReleaseAsset {
        name: "download.bin".to_string(),
        download_url: server.url.clone(),
        binary_name: None,
        target: None,
    };

    let error = super::download_binary(&asset, temp_dir.path(), &[]).await.unwrap_err();
    let _ = server.join();

    assert!(error.to_string().contains("Download failed with status: 404"));
}

#[cfg(all(not(windows), not(target_arch = "wasm32")))]
#[tokio::test]
async fn desktop_self_update_attempts_download_for_newer_version() {
    let _lock = update_env_lock().lock().await;
    let target = super::get_target_triple().unwrap();
    let mut download_server =
        TestServer::respond("404 Not Found", "text/plain", b"missing".to_vec());
    let manifest = json!({
        "version": "v999.0.0",
        "assets": [
            {
                "name": "vibewindow-test.bin",
                "url": download_server.url.clone(),
                "target": target,
                "binary_name": "vibewindow"
            }
        ]
    });
    let mut manifest_server =
        TestServer::respond("200 OK", "application/json", manifest.to_string().into_bytes());
    let _guard = EnvGuard::set(super::APP_UPDATE_API_ENV, &manifest_server.url);

    let error = super::desktop_self_update().await.unwrap_err();
    let _ = manifest_server.join();
    let _ = download_server.join();

    assert!(error.to_string().contains("Download failed with status: 404"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn self_update_force_attempts_download_before_installing() {
    let _lock = update_env_lock().lock().await;
    let target = super::get_target_triple().unwrap();
    let mut download_server =
        TestServer::respond("404 Not Found", "text/plain", b"missing".to_vec());
    let manifest = json!({
        "version": "v999.0.0",
        "assets": [
            {
                "name": "vibewindow-test.bin",
                "url": download_server.url.clone(),
                "target": target,
                "binary_name": "vibewindow"
            }
        ]
    });
    let mut manifest_server =
        TestServer::respond("200 OK", "application/json", manifest.to_string().into_bytes());
    let _guard = EnvGuard::set(super::APP_UPDATE_API_ENV, &manifest_server.url);

    let error = super::self_update(true, false).await.unwrap_err();
    let _ = manifest_server.join();
    let _ = download_server.join();

    assert!(error.to_string().contains("Download failed with status: 404"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn download_binary_extracts_tar_gz_archive() {
    let source_dir = tempfile::tempdir().unwrap();
    std::fs::write(source_dir.path().join("vibewindow"), "tar-binary").unwrap();
    let archive = source_dir.path().join("vibewindow-test.tar.gz");
    assert!(
        Command::new("tar")
            .arg("-czf")
            .arg(&archive)
            .arg("-C")
            .arg(source_dir.path())
            .arg("vibewindow")
            .status()
            .unwrap()
            .success()
    );
    let mut server =
        TestServer::respond("200 OK", "application/gzip", std::fs::read(&archive).unwrap());
    let temp_dir = tempfile::tempdir().unwrap();
    let asset = super::ReleaseAsset {
        name: "vibewindow-test.tar.gz".to_string(),
        download_url: server.url.clone(),
        binary_name: None,
        target: None,
    };

    let binary =
        super::download_binary(&asset, temp_dir.path(), &["vibewindow".to_string()]).await.unwrap();
    let _ = server.join();

    assert_eq!(std::fs::read_to_string(binary).unwrap(), "tar-binary");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn download_binary_extracts_zip_archive() {
    let source_dir = tempfile::tempdir().unwrap();
    std::fs::write(source_dir.path().join("vibewindow"), "zip-binary").unwrap();
    let archive = source_dir.path().join("vibewindow-test.zip");
    assert!(
        Command::new("zip")
            .arg("-q")
            .arg(&archive)
            .arg("vibewindow")
            .current_dir(source_dir.path())
            .status()
            .unwrap()
            .success()
    );
    let mut server =
        TestServer::respond("200 OK", "application/zip", std::fs::read(&archive).unwrap());
    let temp_dir = tempfile::tempdir().unwrap();
    let asset = super::ReleaseAsset {
        name: "vibewindow-test.zip".to_string(),
        download_url: server.url.clone(),
        binary_name: None,
        target: None,
    };

    let binary =
        super::download_binary(&asset, temp_dir.path(), &["vibewindow".to_string()]).await.unwrap();
    let _ = server.join();

    assert_eq!(std::fs::read_to_string(binary).unwrap(), "zip-binary");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn extract_tar_gz_reports_invalid_archive() {
    let temp_dir = tempfile::tempdir().unwrap();
    let archive = temp_dir.path().join("bad.tar.gz");
    std::fs::write(&archive, "bad").unwrap();

    let error = super::extract_tar_gz(&archive, temp_dir.path()).unwrap_err();

    assert!(error.to_string().contains("tar extraction failed"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn extract_zip_reports_invalid_archive() {
    let temp_dir = tempfile::tempdir().unwrap();
    let archive = temp_dir.path().join("bad.zip");
    std::fs::write(&archive, "bad").unwrap();

    let error = super::extract_zip(&archive, temp_dir.path()).unwrap_err();

    assert!(error.to_string().contains("unzip extraction failed"));
}

#[cfg(unix)]
#[test]
fn replace_binary_installs_new_binary_and_removes_backup() {
    let temp_dir = tempfile::tempdir().unwrap();
    let current = temp_dir.path().join("vibewindow");
    let new_binary = temp_dir.path().join("new-vibewindow");
    std::fs::write(&current, "old").unwrap();
    std::fs::write(&new_binary, "new").unwrap();

    super::replace_binary(&new_binary, &current).unwrap();

    assert_eq!(std::fs::read_to_string(&current).unwrap(), "new");
    assert!(!temp_dir.path().join(".vibewindow.backup").exists());
}

#[cfg(unix)]
#[test]
fn replace_binary_removes_stale_staged_and_backup_files_before_install() {
    let temp_dir = tempfile::tempdir().unwrap();
    let current = temp_dir.path().join("vibewindow");
    let new_binary = temp_dir.path().join("new-vibewindow");
    let staged = temp_dir.path().join(".vibewindow.update");
    let backup = temp_dir.path().join(".vibewindow.backup");
    std::fs::write(&current, "old").unwrap();
    std::fs::write(&new_binary, "new").unwrap();
    std::fs::write(&staged, "stale-stage").unwrap();
    std::fs::write(&backup, "stale-backup").unwrap();

    super::replace_binary(&new_binary, &current).unwrap();

    assert_eq!(std::fs::read_to_string(&current).unwrap(), "new");
    assert!(!staged.exists());
    assert!(!backup.exists());
}

#[cfg(unix)]
#[test]
fn replace_binary_reports_missing_source_without_touching_current_binary() {
    let temp_dir = tempfile::tempdir().unwrap();
    let current = temp_dir.path().join("vibewindow");
    let missing_new_binary = temp_dir.path().join("missing-vibewindow");
    std::fs::write(&current, "old").unwrap();

    let error = super::replace_binary(&missing_new_binary, &current).unwrap_err();

    assert!(error.to_string().contains("Failed to copy updated binary"));
    assert_eq!(std::fs::read_to_string(current).unwrap(), "old");
}

#[cfg(unix)]
#[test]
fn replace_binary_reports_missing_current_binary_after_staging_update() {
    let temp_dir = tempfile::tempdir().unwrap();
    let current = temp_dir.path().join("vibewindow");
    let new_binary = temp_dir.path().join("new-vibewindow");
    std::fs::write(&new_binary, "new").unwrap();

    let error = super::replace_binary(&new_binary, &current).unwrap_err();

    assert!(error.to_string().contains("Failed to move current binary"));
    assert_eq!(std::fs::read_to_string(temp_dir.path().join(".vibewindow.update")).unwrap(), "new");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn self_update_check_only_returns_without_requiring_assets() {
    let _lock = update_env_lock().lock().await;
    let mut server = TestServer::respond(
        "200 OK",
        "application/json",
        br#"{"version":"v999.0.0","assets":[]}"#.to_vec(),
    );
    let _guard = EnvGuard::set(super::APP_UPDATE_API_ENV, &server.url);

    super::self_update(false, true).await.unwrap();
    let _ = server.join();
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn self_update_returns_when_already_current_and_not_forced() {
    let _lock = update_env_lock().lock().await;
    let body = format!(r#"{{"version":"v{}","assets":[]}}"#, super::current_version());
    let mut server = TestServer::respond("200 OK", "application/json", body.into_bytes());
    let _guard = EnvGuard::set(super::APP_UPDATE_API_ENV, &server.url);

    super::self_update(false, false).await.unwrap();
    let _ = server.join();
}

#[test]
fn current_version_matches_cargo_package_version() {
    assert_eq!(super::current_version(), env!("CARGO_PKG_VERSION"));
    assert!(!super::current_version().is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn get_current_exe_returns_existing_path() {
    let exe = super::get_current_exe().unwrap();

    assert!(exe.exists());
}

#[cfg(all(not(windows), not(target_arch = "wasm32")))]
#[tokio::test]
async fn desktop_self_update_with_progress_short_circuits_for_current_version() {
    let _lock = update_env_lock().lock().await;
    let body = format!(r#"{{"version":"v{}","assets":[]}}"#, super::current_version());
    let mut server = TestServer::respond("200 OK", "application/json", body.into_bytes());
    let _guard = EnvGuard::set(super::APP_UPDATE_API_ENV, &server.url);

    let mut progress_events = Vec::new();
    let message =
        super::desktop_self_update_with_progress(|p| progress_events.push(p)).await.unwrap();
    let _ = server.join();

    assert!(message.contains("当前已是最新版本"));
    assert!(message.contains(super::current_version()));
    assert!(progress_events.is_empty());
}

#[cfg(all(not(windows), not(target_arch = "wasm32")))]
#[tokio::test]
async fn desktop_self_update_short_circuits_for_current_version() {
    let _lock = update_env_lock().lock().await;
    let body = format!(r#"{{"version":"v{}","assets":[]}}"#, super::current_version());
    let mut server = TestServer::respond("200 OK", "application/json", body.into_bytes());
    let _guard = EnvGuard::set(super::APP_UPDATE_API_ENV, &server.url);

    let message = super::desktop_self_update().await.unwrap();
    let _ = server.join();

    assert!(message.contains("当前已是最新版本"));
}
