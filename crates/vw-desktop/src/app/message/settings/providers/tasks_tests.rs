// 验证提供商异步任务构建及结果消息载荷保持。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use serde_json::json;

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
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

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
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

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
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

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn popular_providers_patch_represents_empty_selection_as_array() {
    assert_eq!(popular_providers_patch(Vec::new()), json!({"popular_providers": []}));
}
