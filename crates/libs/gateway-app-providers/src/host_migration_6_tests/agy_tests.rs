use vw_api_app_provider_profiles::agy::*;
use vw_api_transport::id::TenantScope;
use serde_json::json;
#[test]
fn cc04_agy_native_card_is_saved_without_live_activation() {
    let root = tempfile::tempdir().unwrap();
    let db = agent_database::GlobalDatabase::for_home(root.path()).unwrap();
    let store = app_provider_services::AppProviderStore::new(db).unwrap();
    let t = TenantScope::new("p", "a").unwrap();
    let p = AgyProfile {
        name: "Native login".into(),
        model: Some("Display Name".into()),
        api_key: None,
        base_url: None,
    };
    store.agy_save(&t, "one", &p).unwrap();
    assert_eq!(
        store.get_summary(&t, vw_api_app_identity::AppId::Agy, "one").unwrap().name,
        "Native login"
    );
    assert!(!root.path().join(".gemini").exists());
    assert!(store.agy_profile(&TenantScope::new("p", "b").unwrap(), "one").is_err());
}

#[test]
fn cc04_ui_agy_editor_preserves_metadata_and_latest_secret() {
use vw_api_app_provider_base::create::ProviderSecretWrite;
    let root = tempfile::tempdir().unwrap();
    let db = agent_database::GlobalDatabase::for_home(root.path()).unwrap();
    let store = app_provider_services::AppProviderStore::new(db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = app_settings_store::settings_store::GatewaySettingsStore::new(db).unwrap();
    let profile = AgyProfile {
        name: "Fixture".into(),
        model: Some("旧模型".into()),
        api_key: Some("fake-key".into()),
        base_url: Some("https://example.test".into()),
    };
    store.agy_save(&tenant, "one", &profile).unwrap();
    let mut row = store.agy_record(&tenant, "one").unwrap().unwrap();
    row["custom"] = json!({"extension":"preserved"});
    row["settingsConfig"]["config"]["unknown"] = json!("preserved");
    row["settingsConfig"]["env"]["UNRELATED"] = json!("preserved");
    settings.upsert(&tenant, "app_providers.records", "\"agy\":one", &row.to_string()).unwrap();
    let view = store.agy_form_read(&tenant, "one").unwrap();
    let body = AgyFormWrite {
        target_host_id: "host".into(),
        fields: view.fields.clone(),
        model: Some("新模型".into()),
        base_url: view.base_url.clone(),
        api_key: ProviderSecretWrite::Keep,
        revision: Some(view.revision.clone()),
    };
    row["agy"]["apiKey"] = json!("fake-rotated-key");
    settings.upsert(&tenant, "app_providers.records", "\"agy\":one", &row.to_string()).unwrap();
    assert_eq!(
        store.agy_form_read(&tenant, "one").unwrap().revision,
        view.revision,
        "public revision must not fingerprint secrets"
    );
    store.agy_form_write(&tenant, "one", &body, false).unwrap();
    let saved = store.agy_record(&tenant, "one").unwrap().unwrap();
    assert_eq!(saved["agy"]["apiKey"], "fake-rotated-key");
    assert_eq!(saved["custom"], row["custom"]);
    assert_eq!(saved["settingsConfig"]["config"]["unknown"], "preserved");
    assert_eq!(saved["settingsConfig"]["env"]["UNRELATED"], "preserved");
    let public = store.agy_form_read(&tenant, "one").unwrap();
    assert!(!serde_json::to_string(&public).unwrap().contains("fake-rotated-key"));
    let mut replace = AgyFormWrite {
        revision: Some(public.revision),
        api_key: ProviderSecretWrite::Replace { value: "FAKE-REPLACEMENT".into() },
        ..body
    };
    store.agy_form_write(&tenant, "one", &replace, false).unwrap();
    assert_eq!(
        store.agy_profile(&tenant, "one").unwrap().api_key.as_deref(),
        Some("FAKE-REPLACEMENT")
    );
    replace.revision = Some(store.agy_form_read(&tenant, "one").unwrap().revision);
    replace.api_key = ProviderSecretWrite::Replace { value: "[REDACTED]".into() };
    assert!(store.agy_form_write(&tenant, "one", &replace, false).is_err());
    let mut referenced = store.agy_record(&tenant, "one").unwrap().unwrap();
    referenced["meta"] = json!({"unknownReference":"retained"});
    settings
        .upsert(&tenant, "app_providers.records", "\"agy\":one", &referenced.to_string())
        .unwrap();
    assert!(store.agy_form_delete(&tenant, "one").is_err());
    assert_eq!(store.agy_record(&tenant, "one").unwrap(), Some(referenced));
}
