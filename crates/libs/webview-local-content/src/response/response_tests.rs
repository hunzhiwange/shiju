use super::*;
use crate::test_support::TestDir;
use std::fs;

fn request(path: &str) -> Request<Vec<u8>> {
    Request::builder().uri(format!("vibe://localhost{path}")).body(Vec::new()).unwrap()
}

fn body_text(response: Response<Cow<'static, [u8]>>) -> String {
    String::from_utf8(response.body().to_vec()).unwrap()
}

#[test]
fn is_safe_relative_path_rejects_escape_paths() {
    assert!(is_safe_relative_path(Path::new("assets/app.js")));
    assert!(!is_safe_relative_path(Path::new("../secret")));
    assert!(!is_safe_relative_path(Path::new("/absolute")));
}

#[test]
fn guess_mime_returns_known_types_and_default() {
    assert_eq!(guess_mime(Path::new("index.html")), "text/html; charset=utf-8");
    assert_eq!(guess_mime(Path::new("style.CSS")), "text/css; charset=utf-8");
    assert_eq!(guess_mime(Path::new("app.wasm")), "application/wasm");
    assert_eq!(guess_mime(Path::new("archive.bin")), "application/octet-stream");
}

#[test]
fn local_protocol_response_serves_index_for_root() {
    let dir = TestDir::new("root");
    fs::write(dir.path().join("index.html"), "<main>home</main>").unwrap();

    let response = local_protocol_response(dir.path(), request("/"));

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["Content-Type"], "text/html; charset=utf-8");
    assert_eq!(body_text(response), "<main>home</main>");
}

#[test]
fn local_protocol_response_serves_nested_file_with_mime() {
    let dir = TestDir::new("nested");
    fs::create_dir_all(dir.path().join("assets")).unwrap();
    fs::write(dir.path().join("assets/app.js"), "console.log(1);").unwrap();

    let response = local_protocol_response(dir.path(), request("/assets/app.js"));

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["Content-Type"], "text/javascript; charset=utf-8");
    assert_eq!(body_text(response), "console.log(1);");
}

#[test]
fn local_protocol_response_decodes_percent_encoded_path() {
    let dir = TestDir::new("encoded");
    fs::write(dir.path().join("hello world.txt"), "hello").unwrap();

    let response = local_protocol_response(dir.path(), request("/hello%20world.txt"));

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response), "hello");
}

#[test]
fn local_protocol_response_serves_directory_index() {
    let dir = TestDir::new("directory");
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs/index.html"), "docs").unwrap();

    let response = local_protocol_response(dir.path(), request("/docs"));

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response), "docs");
}

#[test]
fn local_protocol_response_falls_back_to_index_for_spa_route() {
    let dir = TestDir::new("spa");
    fs::write(dir.path().join("index.html"), "spa").unwrap();

    let response = local_protocol_response(dir.path(), request("/settings/profile"));

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response), "spa");
}

#[test]
fn local_protocol_response_returns_not_found_for_missing_asset() {
    let dir = TestDir::new("missing");
    fs::write(dir.path().join("index.html"), "spa").unwrap();

    let response = local_protocol_response(dir.path(), request("/missing.css"));

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(response.body().is_empty());
}

#[test]
fn local_protocol_response_returns_not_found_without_fallback_index() {
    let dir = TestDir::new("no-index");

    let response = local_protocol_response(dir.path(), request("/dashboard"));

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(response.body().is_empty());
}

#[test]
fn local_protocol_response_rejects_path_traversal_after_decode() {
    let dir = TestDir::new("forbidden");

    let response = local_protocol_response(dir.path(), request("/%2E%2E/secret.txt"));

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(response.body().is_empty());
}
