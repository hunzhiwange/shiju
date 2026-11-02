use super::npm_source::fetch;
use base64::{Engine, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256, Sha512};
use std::io::{Read, Write};

fn registry(integrity: &str, redirect: bool) -> (String, std::thread::JoinHandle<()>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let root = format!("http://{}", listener.local_addr().unwrap());
    let metadata = serde_json::json!({"name":"@scope/pkg","version":"1.2.3","dist":{"tarball":format!("{root}/pkg.tgz"),"integrity":integrity}}).to_string();
    let handle = std::thread::spawn(move || {
        for _ in 0..if redirect { 1 } else { 2 } {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let n = stream.read(&mut request).unwrap();
            let req = String::from_utf8_lossy(&request[..n]);
            assert!(!req.to_ascii_lowercase().contains("authorization:"));
            if redirect {
                write!(stream, "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/secret\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
            } else {
                let body = if req.contains("/pkg.tgz ") {
                    "original archive bytes"
                } else {
                    metadata.as_str()
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        }
    });
    (root, handle)
}
#[tokio::test]
async fn codex_plugin_compat_t49_rejects_integrity_mismatch() {
    let (root, server) = registry(&format!("sha512-{}", STANDARD.encode([0; 64])), false);
    let error = fetch("@scope/pkg", Some("1.2.3"), &root, true).await.unwrap_err();
    assert!(error.to_string().contains("integrity mismatch"), "{error}");
    server.join().unwrap();
}
#[tokio::test]
async fn codex_plugin_compat_t49_rejects_invalid_input() {
    for (pkg, version, root) in [
        ("../bad", Some("1"), "https://registry.npmjs.org"),
        ("pkg", Some("file:../bad"), "https://registry.npmjs.org"),
        ("pkg", None, "http://127.0.0.1:1"),
        ("pkg", None, "https://user:secret@example.com"),
    ] {
        assert!(super::npm_source::fetch_npm_plugin(pkg, version, Some(root)).await.is_err());
    }
}
#[tokio::test]
async fn codex_plugin_compat_t49_preserves_isolation() {
    let bytes = b"original archive bytes";
    let (root, server) =
        registry(&format!("sha512-{}", STANDARD.encode(Sha512::digest(bytes))), false);
    let archive = fetch("@scope/pkg", None, &root, true).await.unwrap();
    assert_eq!(archive.version, "1.2.3");
    assert_eq!(archive.package, "@scope/pkg");
    assert_eq!(archive.bytes, bytes);
    assert_eq!(archive.sha256, hex::encode(Sha256::digest(bytes)));
    server.join().unwrap();
    let (root, server) = registry("unused", true);
    assert!(
        fetch("@scope/pkg", None, &root, true)
            .await
            .unwrap_err()
            .to_string()
            .contains("HTTP status 302")
    );
    server.join().unwrap();
}

#[tokio::test]
async fn codex_plugin_compat_t49_rejects_oversized_metadata() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let root = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 4096];
        stream.read_exact(&mut request[..1]).unwrap();
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: 2097153\r\nConnection: close\r\n\r\n")
            .unwrap();
    });
    assert!(fetch("pkg", None, &root, true).await.unwrap_err().to_string().contains("size limit"));
    server.join().unwrap();
}
