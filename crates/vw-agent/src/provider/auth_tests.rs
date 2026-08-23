//! 验证供应商认证数据的序列化、授权边界与密钥存储行为。

use super::*;

#[test]
// 认证方式标签是持久化协议的一部分，大小写必须保持稳定。
fn method_variants_serialize_with_lowercase_type_tags() {
    let api = serde_json::to_value(Method::Api { label: "API Key".into() }).unwrap();
    let oauth = serde_json::to_value(Method::Oauth { label: "OAuth Login".into() }).unwrap();

    assert_eq!(api, serde_json::json!({"type": "api", "label": "API Key"}));
    assert_eq!(oauth, serde_json::json!({"type": "oauth", "label": "OAuth Login"}));
}

#[test]
fn method_variants_deserialize_from_lowercase_type_tags() {
    let api: Method =
        serde_json::from_value(serde_json::json!({"type": "api", "label": "Token"})).unwrap();
    let oauth: Method =
        serde_json::from_value(serde_json::json!({"type": "oauth", "label": "Browser"})).unwrap();

    assert!(matches!(api, Method::Api { label } if label == "Token"));
    assert!(matches!(oauth, Method::Oauth { label } if label == "Browser"));
}

#[test]
fn method_deserialization_rejects_unknown_or_missing_type_tags() {
    let unknown = serde_json::json!({"type": "sso", "label": "SSO"});
    let uppercase = serde_json::json!({"type": "API", "label": "API Key"});
    let missing = serde_json::json!({"label": "API Key"});

    assert!(serde_json::from_value::<Method>(unknown).is_err());
    assert!(serde_json::from_value::<Method>(uppercase).is_err());
    assert!(serde_json::from_value::<Method>(missing).is_err());
}

#[test]
fn auth_error_display_is_stable() {
    assert_eq!(Error::MissingCode.to_string(), "missing code");
    assert_eq!(Error::Unsupported.to_string(), "unsupported auth method");
}

#[test]
fn authorization_round_trip_preserves_all_fields() {
    let auth = Authorization {
        url: "https://example.test/oauth".to_string(),
        method: "oauth".to_string(),
        instructions: "Open the URL".to_string(),
    };
    let value = serde_json::to_value(&auth).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "url": "https://example.test/oauth",
            "method": "oauth",
            "instructions": "Open the URL",
        })
    );

    let decoded: Authorization = serde_json::from_value(value).unwrap();
    assert_eq!(decoded.url, "https://example.test/oauth");
    assert_eq!(decoded.method, "oauth");
    assert_eq!(decoded.instructions, "Open the URL");
}

#[test]
fn authorization_deserialization_requires_all_fields() {
    let missing_url = serde_json::json!({"method": "oauth", "instructions": "Open"});
    let missing_method = serde_json::json!({"url": "https://example.test", "instructions": "Open"});
    let missing_instructions =
        serde_json::json!({"url": "https://example.test", "method": "oauth"});

    assert!(serde_json::from_value::<Authorization>(missing_url).is_err());
    assert!(serde_json::from_value::<Authorization>(missing_method).is_err());
    assert!(serde_json::from_value::<Authorization>(missing_instructions).is_err());
}

#[tokio::test]
async fn auth_stubs_return_documented_values() {
    assert!(methods().await.is_empty());
    assert!(authorize("provider", 0).await.unwrap().is_none());
    assert!(matches!(callback("provider", 0, Some("code")).await, Err(Error::Unsupported)));
}

#[tokio::test]
// 空值和未配置输入不得伪造授权结果。
async fn authorize_returns_none_for_boundary_inputs() {
    assert!(authorize("", 0).await.unwrap().is_none());
    assert!(authorize("provider", usize::MAX).await.unwrap().is_none());
}

#[tokio::test]
async fn callback_rejects_missing_blank_and_nonempty_codes() {
    for code in [None, Some(""), Some("authorization-code")] {
        assert!(matches!(callback("provider", usize::MAX, code).await, Err(Error::Unsupported)));
    }
}

#[tokio::test]
// 重复写入同一供应商时覆盖旧密钥，验证存储只保留当前凭据。
async fn api_stores_and_replaces_key_in_auth_store() {
    let provider_id = format!("provider-auth-test-{}", uuid::Uuid::new_v4());
    api(&provider_id, "first-placeholder-key").await.expect("api key should store");

    let stored = crate::app::agent::auth::get(&provider_id).expect("auth should be stored");
    match stored {
        crate::app::agent::auth::Info::Api(info) => assert_eq!(info.key, "first-placeholder-key"),
        other => panic!("unexpected auth info: {other:?}"),
    }

    api(&provider_id, "replacement-placeholder-key")
        .await
        .expect("replacement api key should store");

    let stored = crate::app::agent::auth::get(&provider_id).expect("auth should be stored");
    match stored {
        crate::app::agent::auth::Info::Api(info) => {
            assert_eq!(info.key, "replacement-placeholder-key");
        }
        other => panic!("unexpected auth info: {other:?}"),
    }

    crate::app::agent::auth::remove(&provider_id).expect("auth cleanup should succeed");
}

#[test]
fn auth_error_from_io_and_display_variants() {
    let err: Error = std::io::Error::new(std::io::ErrorKind::Other, "disk full").into();
    assert_eq!(err.to_string(), "disk full");
    assert_eq!(Error::MissingOauth.to_string(), "missing oauth authorization");
    assert_eq!(Error::CallbackFailed.to_string(), "oauth callback failed");
}
