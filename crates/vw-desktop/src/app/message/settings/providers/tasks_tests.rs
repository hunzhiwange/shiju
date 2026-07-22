use super::*;
use serde_json::json;

#[test]
fn task_builders_schedule_runtime_work() {
    let tasks = [
        refresh_task(),
        sync_remote_task(),
        load_catalog_task(),
        save_popular_providers_task(vec!["OpenAI".to_string()]),
    ];

    for task in tasks {
        assert!(iced_runtime::task::into_stream(task.collect()).is_some());
    }
}

#[test]
fn providers_refreshed_message_preserves_success_payload() {
    let message = providers_refreshed_message(Ok((
        std::collections::HashMap::new(),
        vec!["openai".to_string()],
        true,
        Vec::new(),
    )));

    let Message::Settings(SettingsMessage::ProvidersRefreshed(Ok((
        providers,
        patterns,
        configured,
        catalog,
    )))) = message
    else {
        panic!("expected ProvidersRefreshed success message");
    };
    assert!(providers.is_empty());
    assert_eq!(patterns, vec!["openai"]);
    assert!(configured);
    assert!(catalog.is_empty());
}

#[test]
fn provider_result_messages_preserve_errors() {
    let refreshed = providers_refreshed_message(Err("refresh failed".to_string()));
    let synced = provider_models_sync_done_message(Err("sync failed".to_string()));

    assert!(matches!(
        refreshed,
        Message::Settings(SettingsMessage::ProvidersRefreshed(Err(error)))
            if error == "refresh failed"
    ));
    assert!(matches!(
        synced,
        Message::Settings(SettingsMessage::ProviderModelsSyncDone(Err(error)))
            if error == "sync failed"
    ));
}

#[test]
fn provider_models_sync_done_message_preserves_success_payload() {
    let message = provider_models_sync_done_message(Ok((
        std::collections::HashMap::new(),
        vec!["anthropic".to_string()],
        false,
        Vec::new(),
    )));

    let Message::Settings(SettingsMessage::ProviderModelsSyncDone(Ok((
        providers,
        patterns,
        configured,
        catalog,
    )))) = message
    else {
        panic!("expected ProviderModelsSyncDone success message");
    };
    assert!(providers.is_empty());
    assert_eq!(patterns, vec!["anthropic"]);
    assert!(!configured);
    assert!(catalog.is_empty());
}

#[test]
fn provider_catalog_loaded_message_preserves_success_and_error() {
    let loaded = provider_catalog_loaded_message(Ok(Vec::new()));
    let failed = provider_catalog_loaded_message(Err("catalog failed".to_string()));

    assert!(matches!(
        loaded,
        Message::Settings(SettingsMessage::ProviderCatalogLoaded(Ok(items))) if items.is_empty()
    ));
    assert!(matches!(
        failed,
        Message::Settings(SettingsMessage::ProviderCatalogLoaded(Err(error)))
            if error == "catalog failed"
    ));
}

#[test]
fn popular_providers_saved_message_preserves_success_and_error() {
    let saved = popular_providers_saved_message(Ok(()));
    let failed = popular_providers_saved_message(Err("save failed".to_string()));

    assert!(matches!(saved, Message::Settings(SettingsMessage::PopularProvidersSaved(Ok(())))));
    assert!(matches!(
        failed,
        Message::Settings(SettingsMessage::PopularProvidersSaved(Err(error)))
            if error == "save failed"
    ));
}

#[test]
fn popular_providers_patch_preserves_order_duplicates_and_characters() {
    let patch = popular_providers_patch(vec![
        "openai".to_string(),
        "供应商 \"A\"".to_string(),
        "openai".to_string(),
    ]);

    assert_eq!(patch, json!({"popular_providers": ["openai", "供应商 \"A\"", "openai"]}));
}

#[test]
fn popular_providers_patch_represents_empty_selection_as_array() {
    assert_eq!(popular_providers_patch(Vec::new()), json!({"popular_providers": []}));
}
