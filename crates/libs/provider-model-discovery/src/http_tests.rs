use std::{collections::{HashSet}, time::Duration};
use crate::{merge_remote_models, test_support::*};
use crate::{discover_openai_compatible_models, http::{DiscoveryLimits, discover_openai_compatible_models_with_limits}};

#[tokio::test]
async fn connected_provider_discovers_openai_compatible_models() {
    let server = serve_once(
        "200 OK",
        &[("Content-Type", "application/json")],
        r#"{"data":[{"id":"zeta"},{"id":" alpha "},{"id":"alpha"},{"id":""},{"name":"ignored"}]}"#,
        Duration::ZERO,
    );

    let models = discover_openai_compatible_models(&server.api_base, "secret-token")
        .await
        .expect("model discovery should succeed");
    let request = server
        .request
        .recv_timeout(Duration::from_secs(2))
        .expect("request should reach local server");

    assert_eq!(models, vec!["alpha", "zeta"]);
    assert!(request.starts_with("GET /models HTTP/1.1\r\n"));
    assert!(
        request.lines().any(|line| line.eq_ignore_ascii_case("authorization: Bearer secret-token"))
    );
}

#[tokio::test]
async fn remote_models_reports_http_status_without_returning_models() {
    let server = serve_once("404 Not Found", &[], "missing", Duration::ZERO);

    let error = discover_openai_compatible_models(&server.api_base, "secret-token")
        .await
        .expect_err("404 should not produce discovered models");

    assert_eq!(error.category(), "http_status");
}

#[tokio::test]
async fn remote_models_reports_invalid_json_without_returning_models() {
    let server = serve_once("200 OK", &[], "{not-json", Duration::ZERO);

    let error = discover_openai_compatible_models(&server.api_base, "secret-token")
        .await
        .expect_err("invalid JSON should not produce discovered models");

    assert_eq!(error.category(), "invalid_response");
}

#[tokio::test]
async fn remote_models_empty_data_leaves_existing_models_unchanged() {
    let server = serve_once("200 OK", &[], r#"{"data":[]}"#, Duration::ZERO);
    let mut provider = test_provider();
    let before = provider.models.keys().cloned().collect::<HashSet<_>>();

    let remote_ids = discover_openai_compatible_models(&server.api_base, "secret-token")
        .await
        .expect("empty data is a valid discovery response");
    let changed = merge_remote_models(
        &mut provider,
        &server.api_base,
        &HashSet::new(),
        remote_ids,
        &HashMap::new(),
    );

    assert_eq!(changed, 0);
    assert_eq!(provider.models.keys().cloned().collect::<HashSet<_>>(), before);
}

#[tokio::test]
async fn remote_models_rejects_oversized_response() {
    let body = json!({"data": [{"id": "x".repeat(256)}]}).to_string();
    let server = serve_once("200 OK", &[], &body, Duration::ZERO);

    let error = discover_openai_compatible_models_with_limits(
        &server.api_base,
        "secret-token",
        DiscoveryLimits::new(Duration::from_secs(1), 64),
    )
    .await
    .expect_err("oversized response should be rejected");

    assert_eq!(error.category(), "response_too_large");
}

#[tokio::test]
async fn remote_models_rejects_even_same_origin_redirects() {
    let server = serve_once("302 Found", &[("Location", "/other-models")], "", Duration::ZERO);

    let error = discover_openai_compatible_models(&server.api_base, "secret-token")
        .await
        .expect_err("redirect should not be followed");
    let request = server
        .request
        .recv_timeout(Duration::from_secs(2))
        .expect("original request should reach local server");

    assert_eq!(error.category(), "redirect");
    assert!(request.starts_with("GET /models HTTP/1.1\r\n"));
}

#[tokio::test]
async fn remote_models_reports_timeout_without_returning_models() {
    let server =
        serve_once("200 OK", &[], r#"{"data":[{"id":"too-late"}]}"#, Duration::from_millis(150));

    let error = discover_openai_compatible_models_with_limits(
        &server.api_base,
        "secret-token",
        DiscoveryLimits::new(Duration::from_millis(25), 1024),
    )
    .await
    .expect_err("slow response should time out");

    assert_eq!(error.category(), "timeout");
}
