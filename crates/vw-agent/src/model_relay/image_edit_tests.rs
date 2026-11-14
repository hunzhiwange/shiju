use super::image_edit;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};

#[test]
fn cc_route_083_image_edit_expected_behavior() {
    let original = HeaderValue::from_static("multipart/form-data; boundary=\"image:boundary\"");
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, original.clone());
    assert_eq!(image_edit::content_type(&headers).unwrap(), original);
    let target = reqwest::Url::parse("https://example.invalid/prefix/responses?provider=1").unwrap();
    for path in ["/images/edits", "/v1/images/edits", "/v1/v1/images/edits", "/codex/v1/images/edits"] {
        assert!(image_edit::is_path(path));
        let incoming = format!("{path}?client=2").parse().unwrap();
        assert_eq!(image_edit::url(&target, &incoming).unwrap().as_str(),
            "https://example.invalid/prefix/images/edits?provider=1&client=2");
    }
}

#[test]
fn cc_route_083_rejects_missing_and_ambiguous_boundaries() {
    for media in ["multipart/form-data", "multipart/form-data; boundary=",
        "multipart/form-data; boundary=a; boundary=b", "multipart/form-data; boundary=\"bad \""] {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_str(media).unwrap());
        assert_eq!(image_edit::content_type(&headers), Err(StatusCode::BAD_REQUEST));
    }
}
