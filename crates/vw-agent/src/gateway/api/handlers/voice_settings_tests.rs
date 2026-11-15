use super::*;
#[test]
fn voice_task_13_gateway_patch_rejects_secrets_devices_and_invalid_values() {
    let config = VoiceConfig::default();
    for patch in [
        serde_json::json!({"api_key":"secret"}),
        serde_json::json!({"microphone":"local"}),
        serde_json::json!({"voice":"cove"}),
        serde_json::json!({"dictionary":["<bad>"]}),
        serde_json::json!({"realtime_enabled":null}),
    ] {
        assert!(validate_patch(&config, &patch).is_err());
    }
    assert!(
        validate_patch(
            &config,
            &serde_json::json!({"voice":"cedar", "language":"zh", "provider_id":null})
        )
        .is_ok()
    );
}

#[tokio::test]
async fn voice_task_13_gateway_save_persists_language_dictionary_and_voice_without_changing_other_fields()
 {
    use serde_json::json;
    let tenant = TenantScope::new(format!("voice13-{}", uuid::Uuid::new_v4()), "owner").unwrap();
    crate::config::update_global_for_tenant(
        &tenant,
        json!({
            "gateway":{"auth_enabled":true,"skeys":[{"skey":"voice13-fixture", "name":"fixture"}]},
            "app_ui":{"voice":{"provider_id":"openai","dictation_enabled":true}}
        }),
    )
    .await
    .unwrap();
    let mut headers = HeaderMap::new();
    headers.insert("authorization", "Bearer voice13-fixture".parse().unwrap());
    let patch = json!({"language":"zh", "dictionary":["识矩"], "voice":"cedar"});
    let denied = save(Extension(tenant.clone()), HeaderMap::new(), Json(patch.clone())).await;
    assert_eq!(denied.status(), axum::http::StatusCode::UNAUTHORIZED);
    let response = save(Extension(tenant.clone()), headers, Json(patch)).await;
    assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);
    let saved = crate::config::get_global_for_tenant(&tenant).await.unwrap().app_ui.voice;
    assert!(saved.dictation_enabled);
    assert_eq!(saved.provider_id.as_deref(), Some("openai"));
    assert_eq!(saved.language, vw_config_types::voice::RecognitionLanguage::Zh);
    assert_eq!(saved.dictionary, ["识矩"]);
    assert_eq!(saved.voice, VoiceName::Cedar);
}
