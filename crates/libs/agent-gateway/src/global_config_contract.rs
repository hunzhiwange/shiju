//! Concurrent authenticated configuration contract over a real host listener.
use axum::http::StatusCode;
use serde_json::{Value, json};
use std::time::Duration;
use vw_api_transport::TenantScope;

pub async fn concurrent_fields_preserve_both_values<S, C, H>(
    url: String, token: String, tenant: TenantScope, other: TenantScope,
    state: &gateway_types::AppState<S, C, H>,
) {
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    let request = |method, scope: &TenantScope| {
        client
            .request(method, &url)
            .bearer_auth(&token)
            .header(vw_api_transport::PLATFORM_ID_HEADER, scope.platform_id().as_ref())
            .header(vw_api_transport::COMPANY_ID_HEADER, scope.company_id().as_ref())
    };
    let read_fonts = |response: reqwest::Response| async move {
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.json::<Value>().await.expect("JSON response");
        let settings = &body["app_ui"]["system_settings"];
        (settings["editor_font_size"].as_f64(), settings["terminal_font_size"].as_f64())
    };
    let get = |scope| request(reqwest::Method::GET, scope);
    let patch = |body| request(reqwest::Method::PATCH, &tenant).json(&body);
    assert_eq!(client.get(&url).send().await.unwrap().status(), StatusCode::UNAUTHORIZED);
    let other_before = read_fonts(get(&other).send().await.unwrap()).await;

    for round in 0..20 {
        if round > 0 {
            let reset = patch(json!({
                "app_ui": {"system_settings": {"editor_font_size": 14, "terminal_font_size": 14}}
            }))
            .send()
            .await
            .unwrap();
            assert_eq!(reset.status(), StatusCode::OK);
        }
        let (editor, terminal) = tokio::join!(
            patch(json!({"app_ui": {"system_settings": {"editor_font_size": 24}}}))
                .header("x-test-concurrent-patch", "1")
                .send(),
            patch(json!({"app_ui": {"system_settings": {"terminal_font_size": 19}}}))
                .header("x-test-concurrent-patch", "1")
                .send(),
        );
        assert_eq!(editor.unwrap().status(), StatusCode::OK);
        assert_eq!(terminal.unwrap().status(), StatusCode::OK);
        assert_eq!(
            read_fonts(get(&tenant).send().await.unwrap()).await,
            (Some(24.0), Some(19.0)),
            "persisted fonts in round {round}"
        );
        let cached = state.config.lock();
        let settings = &cached.app_ui.system_settings;
        assert_eq!((settings.editor_font_size, settings.terminal_font_size), (24.0, 19.0));
    }

    let invalid = patch(json!({
        "gateway": {"host": "", "skeys": []},
        "app_ui": {"system_settings": {"editor_font_size": 30}}
    }))
    .send()
    .await
    .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert!(state.pairing.is_authenticated(&token));
    assert_eq!(read_fonts(get(&tenant).send().await.unwrap()).await, (Some(24.0), Some(19.0)));
    assert_eq!(read_fonts(get(&other).send().await.unwrap()).await, other_before);
    {
        let cached = state.config.lock();
        let settings = &cached.app_ui.system_settings;
        assert_eq!((settings.editor_font_size, settings.terminal_font_size), (24.0, 19.0));
    }
}
