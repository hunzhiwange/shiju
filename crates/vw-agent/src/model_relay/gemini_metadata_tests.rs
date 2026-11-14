use super::gemini_metadata::Request;
use axum::http::{Method, StatusCode};

#[test]
fn cc_route_080_gemini_metadata_expected_behavior() {
    let target = "https://example.test/proxy/v1beta/models/old:generateContent".parse().unwrap();
    for (path, method, expected) in [
        ("/v1beta/models?pageSize=20&pageToken=next", Method::GET, "https://example.test/proxy/v1beta/models?pageSize=20&pageToken=next"),
        ("/gemini/v1/models/gemini-pro", Method::GET, "https://example.test/proxy/v1beta/models/gemini-pro"),
        ("/v1beta/models/gemini-pro:countTokens", Method::POST, "https://example.test/proxy/v1beta/models/gemini-pro:countTokens"),
    ] {
        let request = Request::parse(&path.parse().unwrap()).unwrap();
        assert_eq!(request.check_method(&method), Ok(()));
        assert_eq!(request.url(&target, None).unwrap().as_str(), expected);
        let wrong = if method == Method::GET { Method::POST } else { Method::GET };
        assert_eq!(request.check_method(&wrong), Err(StatusCode::METHOD_NOT_ALLOWED));
    }
}

#[test]
fn cc_route_080_rejects_generation_and_client_credentials() {
    for path in [
        "/v1beta/models/gemini-pro:generateContent",
        "/v1beta/models/gemini-pro:delete",
        "/v1beta/models?key=client-secret",
        "/v1beta/models?pageSize=0",
        "/v1beta/models?pageToken=a&pageToken=b",
        "/v1beta/models/gemini-pro:countTokens?alt=sse",
        "/v1beta/models/..",
        "/v1beta/models/a/b",
    ] {
        assert!(Request::parse(&path.parse().unwrap()).is_none(), "{path}");
    }
}
