use super::*;

#[test]
fn desktop_release_source_defaults_to_shiju() {
    assert_eq!(UpdateSource::default(), UpdateSource::Shiju);
    assert_eq!(serde_json::to_string(&UpdateSource::Github).unwrap(), "\"github\"");
}

use serde_json::{Value, json};
use std::io::Read;
use std::net::TcpListener;

fn manifests() -> (Value, Value) {
    let version = "1.2.10";
    let mut oss = Vec::new();
    let mut github = Vec::new();
    for (target, kind, name) in installer_specs(version) {
        let github_url = format!("{GITHUB_ROOT}/download/v{version}/{name}");
        oss.push(json!({"target": target, "kind": kind, "name": name,
            "url": format!("{SHIJU_ROOT}/releases/v{version}/{name}"),
            "github_url": github_url, "size": 4, "sha256": hex::encode(Sha256::digest(b"data"))}));
        github.push(json!({"name": name, "browser_download_url": github_url,
            "size": 4, "digest": format!("sha256:{}", hex::encode(Sha256::digest(b"data")))}));
    }
    (
        json!({"schema_version": 1, "version": version, "published_at": "2026-10-27T00:00:00Z",
        "release_url": format!("{GITHUB_ROOT}/tag/v{version}"), "assets": oss}),
        json!({"tag_name": format!("v{version}"), "draft": false, "prerelease": false, "assets": github}),
    )
}

#[test]
fn desktop_release_missing_config_defaults_and_keys_are_stable() {
    #[derive(serde::Deserialize)]
    struct Config {
        #[serde(default)]
        source: UpdateSource,
    }
    assert_eq!(serde_json::from_str::<Config>("{}").unwrap().source, UpdateSource::Shiju);
    assert_eq!(serde_json::to_string(&UpdateSource::Shiju).unwrap(), "\"shiju\"");
    assert!(serde_json::from_str::<UpdateSource>("\"other\"").is_err());
}

#[test]
fn desktop_release_sources_map_exactly_six_platforms() {
    let (oss, mut github) = manifests();
    github["assets"].as_array_mut().unwrap().push(json!({"name": "cli.zip"}));
    let oss = parse_desktop_release(UpdateSource::Shiju, oss).unwrap();
    let github = parse_desktop_release(UpdateSource::Github, github).unwrap();
    assert_eq!(oss.version, github.version);
    assert_eq!(oss.assets.len(), 6);
    for a in &oss.assets {
        let b = github.installer_for_target(&a.target).unwrap();
        assert_eq!((&a.name, a.kind, a.size, &a.sha256), (&b.name, b.kind, b.size, &b.sha256));
    }
    assert_eq!(
        oss.installer_for_target("aarch64-pc-windows-msvc").unwrap().kind,
        InstallerKind::Msi
    );
    assert!(oss.installer_for_target("armv7-unknown-linux-gnueabihf").is_err());
}

#[test]
fn desktop_release_rejects_invalid_manifest_fields() {
    let (oss, _) = manifests();
    for (pointer, value) in [
        ("/schema_version", json!(2)),
        ("/published_at", json!("")),
        ("/assets/0/target", json!("x86_64-unknown-linux-gnu")),
        ("/assets/0/kind", json!("dmg")),
        ("/assets/0/kind", json!("zip")),
        ("/assets/0/name", json!("../installer.rpm")),
        ("/assets/0/name", json!("shiju-1.2.9-1.aarch64.rpm")),
        ("/assets/0/name", json!("archive.tar.gz")),
        ("/assets/0/sha256", json!("a")),
        ("/assets/0/size", json!(0)),
        ("/assets/0/size", json!(-1)),
        ("/assets/0/url", json!("http://example.com/a.rpm")),
        ("/assets/0/github_url", json!("https://example.com/a.rpm")),
        ("/version", json!("1.2.10-beta")),
    ] {
        let mut invalid = oss.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert!(parse_desktop_release(UpdateSource::Shiju, invalid).is_err(), "{pointer}");
    }
    let mut duplicate = oss.clone();
    duplicate["assets"][1] = duplicate["assets"][0].clone();
    assert!(parse_desktop_release(UpdateSource::Shiju, duplicate).is_err());
    let mut missing = oss;
    missing["assets"].as_array_mut().unwrap().pop();
    assert!(parse_desktop_release(UpdateSource::Shiju, missing).is_err());
}

#[test]
fn desktop_release_rejects_invalid_github_assets_and_unstable_releases() {
    let (_, github) = manifests();
    for (pointer, value) in [
        ("/draft", json!(true)),
        ("/prerelease", json!(true)),
        ("/tag_name", json!("v1.2.10-rc1")),
        ("/assets/0/digest", Value::Null),
        ("/assets/0/name", json!("installer.zip")),
        ("/assets/0/name", json!("../installer.rpm")),
        ("/assets/0/browser_download_url", json!("http://example.com/a.rpm")),
        ("/assets/0/size", json!(0)),
    ] {
        let mut invalid = github.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert!(parse_desktop_release(UpdateSource::Github, invalid).is_err(), "{pointer}");
    }
    let mut duplicate = github;
    let asset = duplicate["assets"][0].clone();
    duplicate["assets"].as_array_mut().unwrap().push(asset);
    assert!(parse_desktop_release(UpdateSource::Github, duplicate).is_err());
}

#[test]
fn desktop_release_compares_numeric_stable_versions() {
    let release = parse_desktop_release(UpdateSource::Shiju, manifests().0).unwrap();
    assert!(release.is_newer_than("1.2.9").unwrap());
    assert!(!release.is_newer_than("v1.2.10").unwrap());
    assert!(!release.is_newer_than("1.10.0").unwrap());
    for invalid in [
        "1.2",
        "1.2.3.4",
        "1.2.3-beta",
        "01.2.3",
        "-1.2.3",
        "1.2.3 ",
        "vv1.2.3",
        "18446744073709551616.0.0",
    ] {
        assert!(release.is_newer_than(invalid).is_err(), "{invalid}");
    }
}

fn server(response: Vec<u8>, delay: Duration) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut request = [0; 4096];
        let count = stream.read(&mut request).unwrap();
        std::thread::sleep(delay);
        let _ = stream.write_all(&response);
        String::from_utf8_lossy(&request[..count]).into_owned()
    });
    (url, handle)
}

fn response(body: &[u8], length: Option<usize>) -> Vec<u8> {
    let header = length.map(|n| format!("Content-Length: {n}\r\n")).unwrap_or_default();
    let mut response = format!("HTTP/1.1 200 OK\r\n{header}Connection: close\r\n\r\n").into_bytes();
    response.extend_from_slice(body);
    response
}

#[tokio::test]
async fn desktop_release_fetch_uses_explicit_source_and_rejects_http_failures() {
    for (source, payload) in
        [(UpdateSource::Shiju, manifests().0), (UpdateSource::Github, manifests().1)]
    {
        let bytes = serde_json::to_vec(&payload).unwrap();
        let (url, handle) = server(response(&bytes, Some(bytes.len())), Duration::ZERO);
        let release =
            fetch_desktop_release_at(source, &url, &http_client(Duration::from_secs(2)).unwrap())
                .await
                .unwrap();
        assert_eq!(release.source, source);
        assert!(!handle.join().unwrap().to_lowercase().contains("authorization:"));
    }
    let (url, handle) =
        server(b"HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\n\r\n".to_vec(), Duration::ZERO);
    assert!(
        fetch_desktop_release_at(
            UpdateSource::Shiju,
            &url,
            &http_client(Duration::from_secs(2)).unwrap()
        )
        .await
        .is_err()
    );
    handle.join().unwrap();
}

#[tokio::test]
async fn desktop_release_download_is_atomic_and_reports_progress_without_content_length() {
    for length in [None, Some(4)] {
        let release = parse_desktop_release(UpdateSource::Shiju, manifests().0).unwrap();
        let asset = &release.assets[0];
        let directory = tempfile::tempdir().unwrap();
        let (url, handle) = server(response(b"data", length), Duration::ZERO);
        let mut progress = Vec::new();
        let path = download_installer_at(
            asset,
            &url,
            directory.path(),
            |p| {
                assert!(!directory.path().join(&asset.name).exists());
                progress.push(p);
            },
            &http_client(Duration::from_secs(2)).unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"data");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
        assert_eq!(progress.first().unwrap().downloaded_bytes, 0);
        assert_eq!(progress.last().unwrap().downloaded_bytes, 4);
        assert_eq!(progress.last().unwrap().total_bytes, Some(4));
        assert!(!handle.join().unwrap().to_lowercase().contains("authorization:"));
        assert!(
            download_installer_at(
                asset,
                "http://127.0.0.1:1",
                directory.path(),
                |_| {},
                &http_client(Duration::from_secs(2)).unwrap()
            )
            .await
            .is_err()
        );
        assert_eq!(std::fs::read(path).unwrap(), b"data");
    }
}

#[tokio::test]
async fn desktop_release_failed_downloads_remove_temporary_files() {
    for (body, length, delay) in [
        (&b"bad!"[..], Some(4), Duration::ZERO),
        (&b"da"[..], Some(4), Duration::ZERO),
        (&b"da"[..], None, Duration::ZERO),
        (&b"data!"[..], None, Duration::ZERO),
        (&b"data"[..], Some(5), Duration::ZERO),
        (&b"data"[..], Some(4), Duration::from_millis(150)),
    ] {
        let release = parse_desktop_release(UpdateSource::Shiju, manifests().0).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let (url, handle) = server(response(body, length), delay);
        let client = http_client(Duration::from_millis(75)).unwrap();
        assert!(
            download_installer_at(&release.assets[0], &url, directory.path(), |_| {}, &client)
                .await
                .is_err()
        );
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
        handle.join().unwrap();
    }
}

#[tokio::test]
async fn desktop_release_rejects_insecure_redirect_without_contacting_destination() {
    let (url, handle) = server(b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/forbidden\r\nContent-Length: 0\r\n\r\n".to_vec(), Duration::ZERO);
    let error = http_client(Duration::from_secs(2)).unwrap().get(url).send().await.unwrap_err();
    assert!(error.is_redirect());
    handle.join().unwrap();
}

#[tokio::test]
async fn desktop_release_revalidates_snapshot_before_downloading() {
    let mut release = parse_desktop_release(UpdateSource::Shiju, manifests().0).unwrap();
    release.assets[0].name = "../escape.rpm".into();
    let directory = tempfile::tempdir().unwrap();
    assert!(
        download_desktop_installer(&release, "aarch64-unknown-linux-gnu", directory.path(), |_| {})
            .await
            .is_err()
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn desktop_release_matches_documented_names_and_new_namespace() {
    assert_eq!(SHIJU_MANIFEST, "https://www.shiju.cloud/agents/shiju-agent/latest/manifest.json");
    let release = parse_desktop_release(UpdateSource::Shiju, manifests().0).unwrap();
    let expected = [
        ("aarch64-unknown-linux-gnu", InstallerKind::Rpm, "shiju-1.2.10-1.aarch64.rpm"),
        ("x86_64-unknown-linux-gnu", InstallerKind::Rpm, "shiju-1.2.10-1.x86_64.rpm"),
        ("aarch64-pc-windows-msvc", InstallerKind::Msi, "ShiJu-aarch64-pc-windows-msvc-dist.msi"),
        ("x86_64-pc-windows-msvc", InstallerKind::Msi, "ShiJu-x86_64-pc-windows-msvc-dist.msi"),
        ("aarch64-apple-darwin", InstallerKind::Dmg, "ShiJu-macos-aarch64-apple-darwin-dist.dmg"),
        ("x86_64-apple-darwin", InstallerKind::Dmg, "ShiJu-macos-x86_64-apple-darwin-dist.dmg"),
    ];
    for (target, kind, name) in expected {
        let asset = release.installer_for_target(target).unwrap();
        assert_eq!(asset.kind, kind);
        assert_eq!(asset.name, name);
        assert_eq!(
            asset.url,
            format!("https://www.shiju.cloud/agents/shiju-agent/releases/v1.2.10/{name}")
        );
    }
}

#[tokio::test]
async fn desktop_release_body_timeout_cleans_partial_download() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 4096];
        let count = stream.read(&mut request).unwrap();
        assert!(count > 0);
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nda").unwrap();
        std::thread::sleep(Duration::from_millis(250));
    });
    let directory = tempfile::tempdir().unwrap();
    let release = parse_desktop_release(UpdateSource::Shiju, manifests().0).unwrap();
    let mut progress = Vec::new();
    assert!(
        download_installer_at(
            &release.assets[0],
            &url,
            directory.path(),
            |p| progress.push(p),
            &http_client(Duration::from_millis(150)).unwrap()
        )
        .await
        .is_err()
    );
    handle.join().unwrap();
    assert!(progress.iter().any(|p| p.downloaded_bytes == 2));
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn desktop_release_publication_race_preserves_existing_file_and_cleans_temporary() {
    let release = parse_desktop_release(UpdateSource::Shiju, manifests().0).unwrap();
    let asset = &release.assets[0];
    let directory = tempfile::tempdir().unwrap();
    let final_path = directory.path().join(&asset.name);
    let (url, handle) = server(response(b"data", Some(4)), Duration::ZERO);
    let result = download_installer_at(
        asset,
        &url,
        directory.path(),
        |progress| {
            if progress.downloaded_bytes == 4 {
                std::fs::write(&final_path, b"existing").unwrap();
            }
        },
        &http_client(Duration::from_secs(2)).unwrap(),
    )
    .await;
    assert!(result.is_err());
    // Keep the returned error alive: it must not retain a temporary file.
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    assert_eq!(std::fs::read(final_path).unwrap(), b"existing");
    handle.join().unwrap();
}

#[tokio::test]
async fn desktop_release_consumes_prepared_publisher_verified_manifest() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .current_dir(root)
        .args(["-c", "import sys,json; sys.path.insert(0,'scripts/tests'); from test_prepare_oss_release import prepared_contract; print(json.dumps(prepared_contract()))"])
        .output()
        .expect("Python 3 is required for the prepare/publish/Rust contract");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    let release = parse_desktop_release(UpdateSource::Shiju, payload["manifest"].clone()).unwrap();
    let github = parse_desktop_release(UpdateSource::Github, payload["github"].clone()).unwrap();
    let body: Vec<u8> = serde_json::from_value(payload["body"].clone()).unwrap();
    assert_eq!(release.assets.len(), 6);
    for asset in &release.assets {
        let other = github.installer_for_target(&asset.target).unwrap();
        assert_eq!(
            (&asset.name, asset.kind, asset.size, &asset.sha256),
            (&other.name, other.kind, other.size, &other.sha256)
        );
        let directory = tempfile::tempdir().unwrap();
        let (url, handle) = server(response(&body, Some(body.len())), Duration::ZERO);
        let path = download_installer_at(
            asset,
            &url,
            directory.path(),
            |_| {},
            &http_client(Duration::from_secs(2)).unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(path).unwrap(), body);
        handle.join().unwrap();
    }
    let mut invalid = payload["manifest"].clone();
    invalid["assets"][0]["kind"] = json!("zip");
    assert!(parse_desktop_release(UpdateSource::Shiju, invalid).is_err());
}
