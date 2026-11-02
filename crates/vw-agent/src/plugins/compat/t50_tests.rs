use super::npm_archive::install_archive;
use crate::plugins::NpmPluginArchive;
use sha2::{Digest, Sha256};
use std::io::Write;

fn archive(extra: Option<(&str, u8)>, package: &str) -> NpmPluginArchive {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);
    let mut append = |name: &str, kind: u8, bytes: &[u8]| {
        let mut header = tar::Header::new_ustar();
        header.as_mut_bytes()[..100].fill(0);
        header.as_mut_bytes()[..name.len()].copy_from_slice(name.as_bytes());
        header.set_entry_type(tar::EntryType::new(kind));
        header.set_size(bytes.len() as u64);
        header.set_mode(0o755);
        if kind == b'2' || kind == b'1' {
            header.set_link_name("/tmp/escape").unwrap();
        }
        header.set_cksum();
        tar.append(&header, bytes).unwrap();
    };
    append("package/package.json", b'0', format!(r#"{{"name":"{package}","version":"1.2.3","scripts":{{"install":"touch SCRIPT_MUST_NOT_RUN"}}}}"#).as_bytes());
    append("package/.codex-plugin/plugin.json", b'0', br#"{"name":"fixture","version":"1.2.3"}"#);
    if let Some((path, kind)) = extra {
        append(path, kind, b"");
    }
    let bytes = tar.into_inner().unwrap().finish().unwrap();
    NpmPluginArchive {
        package: "@scope/pkg".into(),
        version: "1.2.3".into(),
        registry: "https://registry.npmjs.org/".into(),
        sha256: hex::encode(Sha256::digest(&bytes)),
        bytes,
    }
}
#[test]
fn codex_plugin_compat_t50_rejects_archive_escape() {
    for (path, kind) in [
        ("package/../../escape", b'0'),
        ("/tmp/escape", b'0'),
        ("other/file", b'0'),
        ("package/link", b'2'),
        ("package/hard", b'1'),
        ("package/pipe", b'6'),
        ("package/package.json", b'0'),
    ] {
        let out = tempfile::tempdir().unwrap();
        let error = install_archive(archive(Some((path, kind)), "@scope/pkg"), out.path(), None)
            .unwrap_err();
        assert!(error.to_string().contains("unsafe npm archive"), "{path}: {error:#}");
        assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
    }
}
#[test]
fn codex_plugin_compat_t50_rejects_invalid_input() {
    let out = tempfile::tempdir().unwrap();
    assert!(
        install_archive(archive(None, "wrong"), out.path(), None)
            .unwrap_err()
            .to_string()
            .contains("package metadata mismatch")
    );
    assert!(
        crate::plugins::install_plugin_from_source("npm:../bad@1.2.3", out.path())
            .unwrap_err()
            .to_string()
            .contains("invalid npm package")
    );
    let mut bomb = archive(None, "@scope/pkg");
    let mut header = tar::Header::new_ustar();
    header.set_path("package/bomb").unwrap();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(0o644);
    header.set_size(250 * 1024 * 1024 + 1);
    header.set_cksum();
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(header.as_bytes()).unwrap();
    bomb.bytes = encoder.finish().unwrap();
    bomb.sha256 = hex::encode(Sha256::digest(&bomb.bytes));
    assert!(
        install_archive(bomb, out.path(), None)
            .unwrap_err()
            .to_string()
            .contains("extracted size limit")
    );
    let mut bad = archive(None, "@scope/pkg");
    bad.bytes.write_all(b"tamper").unwrap();
    assert!(install_archive(bad, out.path(), None).is_err());
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
}
#[test]
fn codex_plugin_compat_t50_preserves_isolation() {
    let out = tempfile::tempdir().unwrap();
    let installed =
        install_archive(archive(None, "@scope/pkg"), out.path(), Some("catalog")).unwrap();
    assert_eq!(installed.identity.as_key(), "fixture@catalog");
    assert_eq!(
        std::fs::read(installed.installed_dir.join(".codex-plugin/plugin.json")).unwrap(),
        br#"{"name":"fixture","version":"1.2.3"}"#
    );
    assert!(!installed.installed_dir.join("SCRIPT_MUST_NOT_RUN").exists());
    let metadata =
        super::identity::read_install_metadata(&installed.installed_dir, &installed.manifest)
            .unwrap()
            .unwrap();
    let source: serde_json::Value = serde_json::from_str(&metadata.source).unwrap();
    assert_eq!(source["npm"], "@scope/pkg");
    assert_eq!(source["version"], "1.2.3");
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 2);
}
