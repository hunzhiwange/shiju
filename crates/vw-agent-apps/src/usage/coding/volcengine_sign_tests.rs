use chrono::{DateTime, Utc};
use reqwest::{Method, header};
use vw_api_types::apps_usage::UsageQueryErrorCode;

use super::volcengine_sign::sign_volcengine_request;

fn fixed_time() -> DateTime<Utc> {
    "2024-06-21T00:00:00Z".parse().unwrap()
}

#[test]
fn usage_coding_volcengine_sign_returns_expected_result() {
    // The upstream fixed-time fixture, with the full signature pinned independently.
    let request = sign_volcengine_request(
        "AKLTtest",
        "secretkey",
        "cn-beijing",
        "GetAFPUsage",
        b"",
        fixed_time(),
    )
    .unwrap();

    assert_eq!(request.method(), Method::POST);
    assert_eq!(
        request.url().as_str(),
        "https://open.volcengineapi.com/?Action=GetAFPUsage&Region=cn-beijing&Version=2024-01-01"
    );
    assert_eq!(request.body().unwrap().as_bytes(), Some(b"".as_slice()));
    let headers = request.headers();
    assert_eq!(headers[header::HOST], "open.volcengineapi.com");
    assert_eq!(headers[header::CONTENT_TYPE], "application/json; charset=utf-8");
    assert_eq!(headers["x-date"], "20240621T000000Z");
    assert_eq!(
        headers["x-content-sha256"],
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        headers[header::AUTHORIZATION],
        concat!(
            "HMAC-SHA256 Credential=AKLTtest/20240621/cn-beijing/ark/request, ",
            "SignedHeaders=host;x-date;x-content-sha256;content-type, ",
            "Signature=4ceb5e7c3c834fe8604cccf04eeec5ab09045d2e5a36723921f8cea691b0a186"
        )
    );
    assert!(headers[header::AUTHORIZATION].is_sensitive());
    assert!(!format!("{request:?}").contains("AKLTtest"));
}

#[test]
fn usage_coding_volcengine_sign_binds_encoded_query_and_payload() {
    let body = br#"{"probe":true}"#;
    let request = sign_volcengine_request(
        "AKLTtest",
        "secretkey",
        "cn-beijing",
        "Get Coding+&/火~",
        body,
        fixed_time(),
    )
    .unwrap();

    assert_eq!(
        request.url().query(),
        Some("Action=Get%20Coding%2B%26%2F%E7%81%AB~&Region=cn-beijing&Version=2024-01-01")
    );
    assert_eq!(request.body().unwrap().as_bytes(), Some(body.as_slice()));
    assert_eq!(
        request.headers()["x-content-sha256"],
        "c775500ea34eded73c2a3c3bede193f0d839e6c14b36f21b6cc31472e0720a91"
    );
    assert!(
        request.headers()[header::AUTHORIZATION].to_str().unwrap().ends_with(
            "Signature=bb4e5d0a9973b6f22219931d3222d12d3aa0109272f07fb1cda27b9a481ad7c5"
        )
    );
}

#[test]
fn usage_coding_volcengine_sign_rejects_invalid_input() {
    use UsageQueryErrorCode::{CredentialUnavailable, MissingConfiguration};

    let cases = [
        (" ", "fixture-secret", "cn-beijing", "GetAFPUsage", CredentialUnavailable),
        ("fixture-ak", " ", "cn-beijing", "GetAFPUsage", CredentialUnavailable),
        (
            "fixture-ak\r\nInjected: header",
            "fixture-secret",
            "cn-beijing",
            "GetAFPUsage",
            CredentialUnavailable,
        ),
        (
            "fixture-ak/InjectedScope",
            "fixture-secret",
            "cn-beijing",
            "GetAFPUsage",
            CredentialUnavailable,
        ),
        (
            "fixture-ak",
            "fixture-secret\0suffix",
            "cn-beijing",
            "GetAFPUsage",
            CredentialUnavailable,
        ),
        (
            "fixture-ak",
            "fixture-secret",
            "cn-beijing/InjectedScope",
            "GetAFPUsage",
            MissingConfiguration,
        ),
        ("fixture-ak", "fixture-secret", "cn-beijing", "", MissingConfiguration),
    ];

    for (ak, sk, region, action, expected) in cases {
        let error =
            sign_volcengine_request(ak, sk, region, action, b"fixture-payload", fixed_time())
                .unwrap_err();
        assert_eq!(error, expected);
        let diagnostic = format!("{error:?}");
        for value in ["fixture-ak", "fixture-secret", "Injected", "fixture-payload"] {
            assert!(!diagnostic.contains(value));
        }
    }
}
