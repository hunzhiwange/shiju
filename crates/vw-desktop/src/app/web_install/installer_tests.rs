use super::*;
use std::io::Cursor;

fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in files {
        writer.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn github_source_requires_only_the_web_archive() {
    let payload = serde_json::json!({
        "tag_name": "v1.2.3", "draft": false, "prerelease": false,
        "assets": [{
            "name": "shiju-web-1.2.3.zip",
            "browser_download_url": format!("{DOWNLOAD_ROOT}v1.2.3/shiju-web-1.2.3.zip"),
        }],
    });
    let (name, zip) =
        release_urls(UpdateSource::Github, &serde_json::to_vec(&payload).unwrap()).unwrap();
    assert_eq!(name, "shiju-web-1.2.3.zip");
    assert_eq!(zip, format!("{DOWNLOAD_ROOT}v1.2.3/{name}"));
}

#[test]
fn archive_rejects_traversal_foreign_root_and_duplicate_files() {
    for files in [
        vec![("../outside", b"data".as_slice())],
        vec![("other/index.html", b"data".as_slice())],
        vec![("shiju-web-1.2.3/C:/outside", b"data".as_slice())],
        vec![
            ("shiju-web-1.2.3/index.html", b"a".as_slice()),
            ("shiju-web-1.2.3/index.html", b"b".as_slice()),
        ],
    ] {
        let destination = tempfile::tempdir().unwrap();
        assert!(extract(&archive(&files), destination.path(), "shiju-web-1.2.3").is_err());
    }
}

#[test]
fn archive_preserves_wasm_and_nested_bridge_modules() {
    let destination = tempfile::tempdir().unwrap();
    let bytes = archive(&[
        ("shiju-web-1.2.3/shi_ju_bg.wasm", b"wasm"),
        ("shiju-web-1.2.3/snippets/bridge/input.js", b"bridge"),
    ]);
    let package = extract(&bytes, destination.path(), "shiju-web-1.2.3").unwrap();
    assert_eq!(fs::read(package.join("snippets/bridge/input.js")).unwrap(), b"bridge");
    assert_eq!(fs::read(package.join("shi_ju_bg.wasm")).unwrap(), b"wasm");
    assert!(validate_package(&package).is_err());
}

#[test]
fn release_requires_matching_official_assets() {
    let mut release = Release {
        tag_name: "v1.2.3".into(),
        draft: false,
        prerelease: false,
        assets: vec![Asset {
            name: "shiju-web-1.2.3.zip".into(),
            browser_download_url: format!("{DOWNLOAD_ROOT}v1.2.3/shiju-web-1.2.3.zip"),
        }],
    };
    let name = archive_name(&release.tag_name).unwrap();
    assert!(asset_url(&release, &name).is_ok());
    release.assets[0].browser_download_url = "https://example.com/package.zip".into();
    assert!(asset_url(&release, &name).is_err());
    for tag in ["../outside", "v1.2.3/../../x", "v1.2.3-rc.1", ""] {
        assert!(archive_name(tag).is_err());
    }
}

#[test]
fn shiju_source_uses_shared_manifest_and_versioned_web_paths() {
    let payload = serde_json::json!({
        "schema_version": 1, "version": "1.2.3", "published_at": "2026-11-12T00:00:00Z",
        "release_url": "https://github.com/hunzhiwange/shiju/releases/tag/v1.2.3",
        "assets": [],
    });
    let (name, zip) =
        release_urls(UpdateSource::Shiju, &serde_json::to_vec(&payload).unwrap()).unwrap();
    assert_eq!(name, "shiju-web-1.2.3.zip");
    assert_eq!(zip, format!("{SHIJU_ROOT}/releases/v1.2.3/{name}"));
    for (key, value) in [
        ("schema_version", serde_json::json!(2)),
        ("version", serde_json::json!("1.2.3/../x")),
        ("release_url", serde_json::json!("https://example.com/release")),
    ] {
        let mut invalid = payload.clone();
        invalid[key] = value;
        assert!(release_urls(UpdateSource::Shiju, &serde_json::to_vec(&invalid).unwrap()).is_err());
    }
}
