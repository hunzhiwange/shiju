use super::*;
use axum::http::header;

fn bearer_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let value = format!("Bearer {token}");
    headers.insert(header::AUTHORIZATION, HeaderValue::from_str(&value).unwrap());
    headers
}

#[tokio::test]
async fn handle_api_config_get_requires_auth_when_pairing_is_enabled() {
    let token = "paired-token".to_string();
    let response = crate::handlers::handle_api_config_get::<TestHost>(
        State(state(config::Config::default(), true, std::slice::from_ref(&token))),
        Extension(TenantScope::default()),
        HeaderMap::new(),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(response_json(response).await["error"].as_str().unwrap().contains("Unauthorized"));
}
#[tokio::test]
async fn handle_api_config_put_rejects_unauthorized_before_parsing_body() {
    let token = "paired-token".to_string();
    let response = crate::handlers::handle_api_config_put::<TestHost>(
        State(state(config::Config::default(), true, std::slice::from_ref(&token))),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        "not = valid = toml".to_string(),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
#[tokio::test]
async fn handle_api_config_get_accepts_valid_bearer_token_when_pairing_is_enabled() {
    let _env_lock = global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let _home = EnvGuard::set("HOME", temp.path().as_os_str());
    let token = "paired-token".to_string();
    let response = crate::handlers::handle_api_config_get::<TestHost>(
        State(state(config::Config::default(), true, std::slice::from_ref(&token))),
        Extension(TenantScope::default()),
        bearer_headers(&token),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["format"], "toml");
}
