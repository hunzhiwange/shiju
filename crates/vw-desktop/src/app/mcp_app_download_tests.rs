use super::*;

const SOURCE: &str = "https://files.example.test";

fn embedded(uri: &str, field: &str, data: &str) -> serde_json::Value {
    serde_json::json!({"contents":[{"type":"resource","resource":{"uri":uri,(field):data}}]})
}

fn resource_link(uri: &str) -> serde_json::Value {
    serde_json::json!({"contents":[{"type":"resource_link","uri":uri,"name":"Ignored"}]})
}

#[test]
fn mcp_app_download_decodes_text_and_binary_exactly() {
    for (field, data, expected) in
        [("text", "识矩\r\n", "识矩\r\n".as_bytes()), ("blob", "AP+AgA==", &[0, 255, 128, 128][..])]
    {
        let download =
            Download::parse(&embedded("file:///folder/report.dat", field, data), &[]).unwrap();
        assert_eq!(download.prompt.filename, "report.dat");
        assert_eq!(download.prompt.origin, None);
        let Contents::Inline(bytes) = download.contents else { panic!("expected inline data") };
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("chosen-name");
        Destination::new(target.clone()).unwrap().save(&bytes, || true).unwrap();
        assert_eq!(std::fs::read(target).unwrap(), expected);
        assert_eq!(directory.path().read_dir().unwrap().count(), 1);
    }
}

#[test]
fn mcp_app_download_sanitizes_uri_filename() {
    for (uri, expected) in [
        ("file:///tmp/report%20%E8%AF%86%E7%9F%A9.txt?secret=1#fragment", "report 识矩.txt"),
        ("file:///tmp/%2e%2e%2fprivate%5cname", "_private_name"),
        ("file:///tmp/NUL.txt", "_NUL.txt"),
        ("file:///tmp/COM%C2%B9.txt", "_COM¹.txt"),
        ("file:///tmp/report%3Aads%00.txt", "report_ads_.txt"),
        ("file:///tmp/%2e%2e", "download"),
        ("https://files.example.test/", "download"),
    ] {
        let download = Download::parse(&embedded(uri, "text", ""), &[]).unwrap();
        assert_eq!(download.prompt.filename, expected);
    }
    let long_name = format!("file:///{}.txt", "识".repeat(255));
    let download = Download::parse(&embedded(&long_name, "text", ""), &[]).unwrap();
    assert_eq!(download.prompt.filename, "download");
}

#[test]
fn mcp_app_download_rejects_malformed_and_oversized_contents() {
    for params in [
        embedded("file:///export", "text", &"x".repeat(MAX_INLINE_BYTES + 1)),
        embedded(
            "file:///export",
            "blob",
            &base64::engine::general_purpose::STANDARD.encode(vec![0; MAX_INLINE_BYTES + 1]),
        ),
        embedded("file:///export", "blob", "invalid"),
        serde_json::json!({"contents":[{"type":"resource","resource":{
            "uri":"file:///export","text":"text","blob":"AA=="
        }}]}),
        serde_json::json!({"contents":[]}),
        serde_json::json!({"contents":[{"type":"resource"},{"type":"resource"}]}),
    ] {
        assert!(Download::parse(&params, &[]).is_err());
    }
}

#[test]
fn mcp_app_download_does_not_replace_file_when_revoked_or_newly_created() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("report");
    let missing = Destination::new(target.clone()).unwrap();
    std::fs::write(&target, b"original").unwrap();
    assert!(missing.save(b"new", || true).is_err());
    let existing = Destination::new(target.clone()).unwrap();
    assert!(existing.save(b"new", || false).is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"original");
    assert_eq!(directory.path().read_dir().unwrap().count(), 1);
    Destination::new(target.clone()).unwrap().save(b"replaced", || true).unwrap();
    assert_eq!(std::fs::read(target).unwrap(), b"replaced");
}

#[test]
fn mcp_app_download_write_failure_leaves_no_temporary_file() {
    let directory = tempfile::tempdir().unwrap();
    let parent = directory.path().join("parent");
    let target = parent.join("report");
    std::fs::create_dir(&parent).unwrap();
    let destination = Destination::new(target.clone()).unwrap();
    std::fs::remove_dir(&parent).unwrap();
    assert!(destination.save(b"file", || true).is_err());
    assert!(!target.exists());
    assert_eq!(directory.path().read_dir().unwrap().count(), 0);
}

#[test]
fn mcp_app_download_rejects_unapproved_urls_and_redirects() {
    let domains = [SOURCE.to_owned(), "https://also-approved.example.test".to_owned()];
    for uri in [
        "http://files.example.test/report",
        "file:///private/report",
        "mcp://other-server/report",
        "https://someone:secret@files.example.test/report",
        "https://@files.example.test/report",
        "https://files.example.test.evil.test/report",
        "https://files.example.test:8443/report",
        "https://127.0.0.1/report",
    ] {
        assert!(Download::parse(&resource_link(uri), &domains).is_err(), "{uri}");
    }
    let download =
        Download::parse(&resource_link("https://files.example.test/old/report"), &domains).unwrap();
    assert_eq!(download.prompt.filename, "report");
    assert_eq!(download.prompt.origin.as_deref(), Some(SOURCE));
    let Contents::Https(url) = download.contents else { panic!("expected HTTPS resource") };
    assert_eq!(redirect_target(&url, "../new").unwrap().as_str(), format!("{SOURCE}/new"));
    for location in [
        "file:///private/report",
        "http://files.example.test/report",
        "//also-approved.example.test/report",
        "//127.0.0.1/report",
        "//name@files.example.test/report",
        "//@files.example.test/report",
        "/\\also-approved.example.test/report",
    ] {
        assert!(redirect_target(&url, location).is_err(), "{location}");
    }
}

#[test]
fn mcp_app_download_rejects_nonpublic_or_mixed_dns_answers() {
    let public = "8.8.8.8:443".parse().unwrap();
    for ip in [
        "0.1.2.3",
        "10.1.2.3",
        "100.64.0.1",
        "127.0.0.1",
        "169.254.169.254",
        "172.16.0.1",
        "192.168.0.1",
        "198.18.0.1",
        "203.0.113.1",
        "224.0.0.1",
        "240.0.0.1",
        "::",
        "::1",
        "::ffff:127.0.0.1",
        "64:ff9b::7f00:1",
        "2002:7f00:0001::1",
        "fc00::1",
        "fe80::1",
    ] {
        let address = SocketAddr::new(ip.parse().unwrap(), 443);
        assert!(validate_addresses(&[public, address]).is_err(), "{ip}");
    }
    assert!(validate_addresses(&[]).is_err());
    assert!(validate_addresses(&[public, "[2606:4700::1111]:443".parse().unwrap()]).is_ok());
}

#[tokio::test]
#[ignore = "requires outbound HTTPS access to the RFC Editor"]
async fn mcp_app_download_real_https_saves_expected_bytes() {
    use sha2::{Digest, Sha256};

    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("rfc20.txt");
    let origin = "https://www.rfc-editor.org";
    let download = Download::parse(
        &resource_link("https://www.rfc-editor.org/rfc/rfc20.txt"),
        &[origin.to_owned()],
    )
    .unwrap();
    download
        .save(
            |prompt| {
                assert_eq!(prompt.origin.as_deref(), Some(origin));
                assert_eq!(prompt.filename, "rfc20.txt");
                assert!(!target.exists());
                std::future::ready(Some(target.clone()))
            },
            || true,
        )
        .await
        .unwrap();
    let bytes = std::fs::read(&target).unwrap();
    assert_eq!(bytes.len(), 18504);
    assert_eq!(
        hex::encode(Sha256::digest(bytes)),
        "714d11bfcbc001f98cd8a92291a19e3f670c2236ad02771092e0eea826acd13a"
    );
}
