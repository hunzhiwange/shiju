use super::*;
use serde_json::{Value, json};

fn manifest() -> Value {
    let sha256 = "a".repeat(64);
    json!({
        "schema": 1, "app": "browse", "artifact": "cli", "channel": "stable",
        "release": {
            "version": "0.1.5",
            "targets": {
                "macos-arm64": {
                    "url": format!("/release/browse/files/{sha256}/opencode-browse-darwin-arm64-0.1.5.tgz"),
                    "size": 123,
                    "sha256": sha256,
                }
            },
            "revoked_versions": [],
        },
    })
}

#[test]
fn stable_manifest_requires_the_requested_target_and_a_content_addressed_browse_asset() {
    let release =
        |manifest: &Value, target| parse_release(&serde_json::to_vec(manifest).unwrap(), target);
    let mut payload = manifest();
    let stable = release(&payload, "macos-arm64").unwrap();
    assert_eq!(stable.version, Version::new(0, 1, 5));
    assert!(release(&payload, "windows-x64").is_err());

    payload["release"]["revoked_versions"] = json!(["0.1.5"]);
    assert!(release(&payload, "macos-arm64").is_err());
    payload["release"]["revoked_versions"] = json!([]);
    payload["release"]["targets"]["macos-arm64"]["url"] = json!("https://example.com/browse.tgz");
    assert!(release(&payload, "macos-arm64").is_err());
}

#[test]
fn only_the_browse_binary_is_extracted_from_the_archive() {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut archive = tar::Builder::new(encoder);
    let artifact = PathBuf::from("package").join("bin").join(binary_name());
    for (name, content) in [("package/package.json".into(), b"{}" as &[u8]), (artifact, b"binary")]
    {
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        archive.append_data(&mut header, name, content).unwrap();
    }
    let bytes = archive.into_inner().unwrap().finish().unwrap();
    let mut output = Vec::new();
    extract_binary(&bytes, &mut output).unwrap();
    assert_eq!(output, b"binary");
    assert!(extract_binary(b"not an archive", &mut Vec::new()).is_err());
}

#[test]
fn versioned_install_replaces_only_the_managed_auto_update_environment_setting() {
    let root = Path::new("gateway-bin").join("browse");
    let version = Version::new(0, 1, 5);
    let binary = binary_at(&root, &version);
    assert_eq!(managed_version(&root, &binary), Some(version));
    assert!(managed_version(Path::new("custom"), &binary).is_none());

    let mut config = json!({
        "args": ["mcp"],
        "env": [
            {"name": "OPENCODE_BROWSER_PORT", "value": "19988"},
            {"name": AUTO_UPDATE_ENV, "value": "on"}
        ]
    });
    set_managed_environment(&mut config).unwrap();
    assert_eq!(config["args"], json!(["mcp"]));
    assert_eq!(
        config["env"],
        json!([
            {"name": "OPENCODE_BROWSER_PORT", "value": "19988"},
            {"name": AUTO_UPDATE_ENV, "value": "off"},
        ])
    );
}
