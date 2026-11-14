#[test]
fn cc_route_037_session_converter_remains_strict() {
    assert!(super::gemini::to_chat(&serde_json::json!({"contents":[],"unknown":true})).is_err());
}

#[test]
fn cc_route_037_native_gemini_expected_behavior() {
    use super::native_gemini::Request;
    for prefix in ["/v1beta", "/gemini/v1beta", "/gemini/v1"] {
        for (action, streaming) in [("generateContent", false), ("streamGenerateContent", true)] {
            let uri = format!("{prefix}/models/models/fixture:{action}").parse().unwrap();
            let request = Request::parse(&uri).unwrap();
            assert_eq!(request.model, "fixture");
            assert_eq!(request.streaming, streaming);
            let target = "https://fixture.invalid/v1/models/old:generateContent".parse().unwrap();
            let url = request.url(&target, "models/mapped").unwrap();
            assert_eq!(url.path(), format!("/v1/models/mapped:{action}"));
            assert_eq!(url.query(), streaming.then_some("alt=sse"));
        }
    }
    for uri in [
        "/v1/models/a:generateContent",
        "/v1beta/models/a:countTokens",
        "/v1beta/models/a:generateContent?alt=sse",
        "/v1beta/models/a:streamGenerateContent?key=bad",
        "/v1beta/models/a:streamGenerateContent?alt=sse&alt=json",
        "/v1beta/models/%2e%2e:generateContent",
        "/v1beta/models/a:generateContent/",
    ] {
        assert!(Request::parse(&uri.parse().unwrap()).is_none(), "{uri}");
    }
}
