//! 测试 OpenAI OAuth 占位实现显式失败及响应模型序列化。
//! 网络入口当前必须返回明确未实现错误，防止占位数据被误当作有效认证。

use super::openai_oauth::{
    DeviceCodeResponse, OAuthTokens, PkceState, build_authorize_url, exchange_code_for_tokens,
    extract_account_id_from_jwt, generate_pkce_state, parse_code_from_redirect,
    poll_device_code_tokens, receive_loopback_code, start_device_code_flow,
};

#[test]
fn placeholder_pkce_and_parsers_are_explicitly_empty() {
    let pkce = generate_pkce_state();

    assert!(pkce.verifier.is_empty());
    assert!(pkce.challenge.is_empty());
    assert!(pkce.state.is_empty());
    assert!(build_authorize_url(&pkce).is_empty());
    assert_eq!(extract_account_id_from_jwt("not-a-jwt"), None);
    assert_eq!(parse_code_from_redirect("http://localhost/callback?code=abc"), None);
}

#[tokio::test]
async fn unimplemented_oauth_network_flows_fail_explicitly() {
    let client = reqwest::Client::new();
    let device = DeviceCodeResponse {
        device_code: "device".to_string(),
        user_code: "user".to_string(),
        verification_uri: "https://example.test".to_string(),
        expires_in: 600,
        interval: 5,
    };
    let pkce = generate_pkce_state();

    assert!(
        start_device_code_flow(&client)
            .await
            .unwrap_err()
            .to_string()
            .contains("not fully implemented")
    );
    assert!(
        poll_device_code_tokens(&client, &device)
            .await
            .unwrap_err()
            .to_string()
            .contains("not fully implemented")
    );
    assert!(
        receive_loopback_code(0).await.unwrap_err().to_string().contains("not fully implemented")
    );
    assert!(
        exchange_code_for_tokens(&client, "code", &pkce)
            .await
            .unwrap_err()
            .to_string()
            .contains("not fully implemented")
    );
}

#[test]
fn oauth_response_models_round_trip_through_json() {
    let device = DeviceCodeResponse {
        device_code: "device-code".to_string(),
        user_code: "USER-CODE".to_string(),
        verification_uri: "https://auth.example.test/device".to_string(),
        expires_in: 900,
        interval: 7,
    };
    let encoded = serde_json::to_string(&device).unwrap();
    let decoded: DeviceCodeResponse = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.device_code, device.device_code);
    assert_eq!(decoded.interval, 7);

    let tokens = OAuthTokens {
        access_token: "access".to_string(),
        refresh_token: "refresh".to_string(),
        expires_in: 3600,
    };
    let decoded: OAuthTokens =
        serde_json::from_value(serde_json::to_value(&tokens).unwrap()).unwrap();
    assert_eq!(decoded.access_token, "access");
    assert_eq!(decoded.refresh_token, "refresh");
    assert_eq!(decoded.expires_in, 3600);
}

#[test]
fn pkce_state_clone_preserves_placeholder_fields() {
    let pkce = PkceState {
        verifier: "verifier".to_string(),
        challenge: "challenge".to_string(),
        state: "state".to_string(),
    };
    let cloned = pkce.clone();

    assert_eq!(cloned.verifier, "verifier");
    assert_eq!(cloned.challenge, "challenge");
    assert_eq!(cloned.state, "state");
    assert!(format!("{cloned:?}").contains("verifier"));
}
