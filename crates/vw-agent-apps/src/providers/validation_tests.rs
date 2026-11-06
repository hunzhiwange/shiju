use super::{create::create, update::update};
use serde_json::json;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{create::AppProviderCreate, update::AppProviderUpdate},
};
fn input(options: serde_json::Value) -> AppProviderCreate {
    serde_json::from_value(json!({"name":"fixture","protocol":"anthropic","apiKey":{"action":"replace","value":"fixture-secret"},"baseUrl":"https://fixture.test","model":"fixture","addToLive":false,"requestOptions":options})).unwrap()
}
#[test]
fn cc04_validation_rejects_invalid_headers_and_endpoint() {
    for options in [
        json!({"requestHeaders":{"Bad Header":"ok"}}),
        json!({"requestHeaders":{"X-Test":"a\nb"}}),
        json!({"localProxyRequestOverrides":{"headers":{"AUTHORIZATION":"fixture-secret"}}}),
        json!({"localProxyRequestOverrides":{"body":{"stream":false}}}),
    ] {
        let mut inserted = false;
        assert!(
            create(AppId::Claude, "fixture", &input(options), |_| {
                inserted = true;
                Ok(true)
            })
            .is_err()
        );
        assert!(!inserted);
    }
    let mut request = input(json!({}));
    request.base_url = "https://user:secret@fixture.test".into();
    assert!(
        create(AppId::Claude, "fixture", &request, |_| panic!("invalid endpoint persisted"))
            .is_err()
    );
}
#[test]
fn cc04_validation_keeps_app_specific_fields() {
    let before = json!({"name":"fixture","protocol":"anthropic","apiKey":"fixture-secret","baseUrl":"https://fixture.test","model":"fixture","settingsConfig":{"env":{"APP_PRIVATE":"keep"}},"meta":{"unknown":{"foo":42}},"requestOptions":{"requestHeaders":{"authorization":"fixture-header-secret"},"apiKeyField":"ANTHROPIC_API_KEY"}});
    let request:AppProviderUpdate=serde_json::from_value(json!({"name":"edited","requestOptions":{"localProxyRequestOverrides":{"body":{"temperature":0.2}}}})).unwrap();
    let after = update(AppId::Claude, "fixture", &before, &request).unwrap();
    assert_eq!(after["settingsConfig"], before["settingsConfig"]);
    assert_eq!(after["meta"], before["meta"]);
    assert_eq!(
        after["requestOptions"]["requestHeaders"],
        before["requestOptions"]["requestHeaders"]
    );
    assert_eq!(after["requestOptions"]["apiKeyField"], before["requestOptions"]["apiKeyField"]);
}

#[test]
fn cc04_validation_distinguishes_ordinary_and_proxy_headers_and_explicit_clear() {
    use super::validation::{endpoint, merge, validate};
    use vw_api_types::app_providers::validation::ProviderRequestOptions;
    let normal: ProviderRequestOptions = serde_json::from_value(
        json!({"requestHeaders":{" Authorization ":"fixture-secret","X-Test":"hello\tworld"}}),
    )
    .unwrap();
    let normalized = merge(AppId::Claude, None, &normal).unwrap();
    assert_eq!(normalized.request_headers.as_ref().unwrap()["authorization"], "fixture-secret");
    for header in [
        "host",
        "Content-Length",
        "CONTENT-TYPE",
        "Authorization",
        "x-goog-api-key",
        "traceparent",
        "X-Azure-Ref",
    ] {
        let value: ProviderRequestOptions = serde_json::from_value(
            json!({"localProxyRequestOverrides":{"headers":{header:"value"}}}),
        )
        .unwrap();
        assert!(validate(AppId::Claude, &value).is_err());
    }
    for headers in [
        json!({"X-Test":"1","x-test":"2"}),
        json!({" X-Test ":"1","x-test":"2"}),
        json!({"draft-header:fixture":"value"}),
        json!({"X-Test":"bad\u{007f}"}),
    ] {
        let options: ProviderRequestOptions =
            serde_json::from_value(json!({"requestHeaders":headers})).unwrap();
        assert!(validate(AppId::Claude, &options).is_err());
    }
    for url in [
        "https://fixture.test/a b",
        "https://fixture.test\\path",
        "http://fixture.test",
        "https://fixture.test?secret=1",
        "https://fixture.test#secret",
        "https:fixture.test",
    ] {
        assert!(endpoint(url).is_err());
    }
    for url in [
        "https://fixture.test/v1",
        "http://localhost:8080",
        "http://127.0.0.1:8080",
        "http://[::1]:8080",
    ] {
        assert!(endpoint(url).is_ok());
    }
    let old=serde_json::from_value(json!({"requestHeaders":{"x-test":"private"},"apiKeyField":"ANTHROPIC_API_KEY","localProxyRequestOverrides":{"body":{"temperature":0.2}}})).unwrap();
    let clear=serde_json::from_value(json!({"requestHeaders":{},"apiKeyField":"ANTHROPIC_AUTH_TOKEN","localProxyRequestOverrides":{}})).unwrap();
    assert_eq!(merge(AppId::Claude, Some(&old), &clear).unwrap(), clear);
    assert!(validate(AppId::Gemini, &clear).is_err());
}
#[test]
fn cc04_validation_preview_rejects_options_and_source_metadata_without_files() {
    use vw_api_types::app_providers::preview::ProviderPreviewRequest;
    let root = tempfile::tempdir().unwrap();
    let mut request:ProviderPreviewRequest=serde_json::from_value(json!({"targetHostId":"fixture","settings":{"env":{}},"requestOptions":{"localProxyRequestOverrides":{"body":{"stream":true}}}})).unwrap();
    assert!(
        super::preview::read(root.path(), AppId::Claude, &request, &Default::default()).is_err()
    );
    request.request_options = None;
    request.meta = Some(json!({"apiKeyField":"INJECT_ENV"}));
    assert!(
        super::preview::read(root.path(), AppId::Claude, &request, &Default::default()).is_err()
    );
    request.meta = Some(json!({"localProxyRequestOverrides":{"headers":{"host":"fixture"}}}));
    assert!(
        super::preview::read(root.path(), AppId::Claude, &request, &Default::default()).is_err()
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn cc04_validation_launch_rejects_unimplemented_options_before_creating_files() {
    use crate::apps::launch::{provider::prepare_provider_launch, request::LaunchMaterial};
    use vw_api_types::app_providers::AppProviderWrite;
    let root = tempfile::tempdir().unwrap();
    let program = root.path().join("fixture");
    std::fs::write(&program, "#!/bin/sh\nexit 0\n").unwrap();
    let mut profile:AppProviderWrite=serde_json::from_value(json!({"name":"fixture","protocol":"anthropic","apiKey":"fixture-key","baseUrl":"https://fixture.test","model":"fixture","requestOptions":{"requestHeaders":{"x-test":"fixture-secret"}}})).unwrap();
    let material = || LaunchMaterial {
        program: program.clone(),
        cwd: root.path().into(),
        args: vec![],
        skip_permissions: false,
    };
    let result = prepare_provider_launch(AppId::Claude, &profile, material(), root.path());
    assert!(
        matches!(result,Err(ref error) if error=="provider request options require a native application adapter")
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    // Explicit clearing leaves the original no-options launch semantics usable.
    profile.request_options = Some(
        serde_json::from_value(json!({"requestHeaders":{},"localProxyRequestOverrides":{}}))
            .unwrap(),
    );
    let launch = prepare_provider_launch(AppId::Claude, &profile, material(), root.path()).unwrap();
    drop(launch);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn cc04_validation_previews_reject_native_endpoints_and_headers() {
    use vw_api_types::app_providers::preview::ProviderPreviewRequest;
    for (app, settings) in [
        (AppId::Claude, json!({"env":{"ANTHROPIC_BASE_URL":"https://user:secret@fixture.test"}})),
        (AppId::Gemini, json!({"env":{"GOOGLE_GEMINI_BASE_URL":"http://fixture.test"}})),
        (
            AppId::Codex,
            json!({"auth":{},"config":"model_provider='vendor'\n[model_providers.vendor]\nbase_url='http://fixture.test'\n"}),
        ),
        (
            AppId::Codex,
            json!({"auth":{},"config":"model_provider='vendor'\n[model_providers.vendor]\nbase_url='https://fixture.test'\nhttp_headers={'Bad Header'='fixture-secret'}\n"}),
        ),
        (
            AppId::GrokBuild,
            json!({"config":"[models]\ndefault='vendor'\n[model.vendor]\nname='fixture'\nmodel='fixture'\nbase_url='http://fixture.test'\napi_key='fixture-secret'\napi_backend='responses'\ncontext_window=1000\n"}),
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let request: ProviderPreviewRequest =
            serde_json::from_value(json!({"targetHostId":"fixture","settings":settings})).unwrap();
        assert!(super::preview::read(root.path(), app, &request, &Default::default()).is_err());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
