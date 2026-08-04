//! 处理模型提供商设置子模块的目录、认证和连接状态。

use crate::app::{
    App, Message,
    state::{
        CustomProviderDraft, CustomProviderModelDraft, CustomProviderModelModalState,
        ProviderConnectState, ProviderHeaderDraft, SHIJU_OFFICIAL_PROVIDER_NAME,
        is_shiju_official_provider_id,
    },
};
use iced::Task;

use super::super::messages::SettingsMessage;
use super::helpers::{is_valid_provider_id, summarize_providers};
use super::models::{
    connect_provider, current_provider_api_key, disconnect_provider,
    discover_custom_provider_models, load_custom_provider_draft, merge_discovered_model_drafts,
    save_custom_provider,
};
use super::tasks::{
    load_catalog_task, refresh_task, save_popular_providers_task, sync_remote_task,
};

fn reset_models_sync_state(app: &mut App) {
    app.provider_settings.models_syncing = false;
    app.provider_settings.models_sync_progress = 0.0;
    app.provider_settings.models_sync_label.clear();
}

fn replace_popular_patterns(app: &mut App, patterns: Vec<String>) {
    app.provider_settings.popular_patterns =
        patterns.into_iter().filter(|pattern| !is_shiju_official_provider_id(pattern)).collect();
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::ProvidersRefresh => {
            app.provider_settings.loading = true;
            app.provider_settings.connect_error = None;
            app.provider_settings.save_error = None;
            app.provider_settings.disconnect_confirm_provider_id = None;
            refresh_task()
        }
        SettingsMessage::ProviderModelsSyncRemote => {
            if app.provider_settings.loading || app.provider_settings.models_syncing {
                return Task::none();
            }
            app.provider_settings.connect_error = None;
            app.provider_settings.save_error = None;
            app.provider_settings.disconnect_confirm_provider_id = None;
            app.provider_settings.models_syncing = true;
            app.provider_settings.models_sync_progress = 0.08;
            app.provider_settings.models_sync_label = "正在从 models.dev 拉取模型目录…".to_string();
            sync_remote_task()
        }
        SettingsMessage::ProviderModelsSyncTick => {
            if !app.provider_settings.models_syncing {
                return Task::none();
            }
            let next = (app.provider_settings.models_sync_progress + 0.07).min(0.9);
            app.provider_settings.models_sync_progress = next;
            app.provider_settings.models_sync_label = if next < 0.35 {
                "正在从 models.dev 拉取模型目录…".to_string()
            } else if next < 0.7 {
                "正在写入本地缓存…".to_string()
            } else {
                "正在刷新提供商视图…".to_string()
            };
            Task::none()
        }
        SettingsMessage::ProviderModelsSyncDone(res) => match res {
            Ok((providers, patterns, configured, catalog)) => {
                app.provider_settings.providers = summarize_providers(providers);
                if configured {
                    replace_popular_patterns(app, patterns);
                }
                if !catalog.is_empty() {
                    app.provider_settings.catalog_items = catalog;
                }
                app.provider_settings.catalog_loading = false;
                reset_models_sync_state(app);
                Task::none()
            }
            Err(e) => {
                app.provider_settings.save_error = Some(e);
                app.provider_settings.catalog_loading = false;
                reset_models_sync_state(app);
                Task::none()
            }
        },
        SettingsMessage::ProvidersRefreshed(res) => match res {
            Ok((providers, patterns, configured, catalog)) => {
                app.provider_settings.loading = false;
                app.provider_settings.providers = summarize_providers(providers);
                if configured {
                    replace_popular_patterns(app, patterns);
                }
                if !catalog.is_empty() {
                    app.provider_settings.catalog_items = catalog;
                }
                app.provider_settings.catalog_loading = false;
                Task::none()
            }
            Err(e) => {
                app.provider_settings.loading = false;
                app.provider_settings.save_error = Some(e);
                app.provider_settings.catalog_loading = false;
                Task::none()
            }
        },
        SettingsMessage::ProviderConnectOpen(provider_id) => {
            let provider_name = app
                .provider_settings
                .catalog_items
                .iter()
                .find(|p| p.provider_id == provider_id)
                .map(|p| p.provider_name.clone())
                .or_else(|| {
                    app.provider_settings
                        .providers
                        .iter()
                        .find(|p| p.id == provider_id)
                        .map(|p| p.name.clone())
                })
                .unwrap_or_else(|| {
                    if is_shiju_official_provider_id(&provider_id) {
                        SHIJU_OFFICIAL_PROVIDER_NAME.to_string()
                    } else {
                        provider_id.clone()
                    }
                });
            let api_key = current_provider_api_key(&provider_id).unwrap_or_default();
            app.provider_settings.connect_modal =
                Some(ProviderConnectState { provider_id, provider_name, api_key });
            app.provider_settings.connect_error = None;
            app.provider_settings.catalog_open = false;
            app.provider_settings.custom_model_modal = None;
            app.provider_settings.custom_provider_modal_open = false;
            app.provider_settings.custom_editing_provider_id = None;
            app.provider_settings.disconnect_confirm_provider_id = None;
            Task::none()
        }
        SettingsMessage::ProviderConnectClose => {
            app.provider_settings.connect_modal = None;
            app.provider_settings.connect_error = None;
            Task::none()
        }
        SettingsMessage::ProviderConnectApiKeyChanged(v) => {
            if let Some(m) = &mut app.provider_settings.connect_modal {
                m.api_key = v;
            }
            Task::none()
        }
        SettingsMessage::ProviderConnectSubmit => {
            let Some(m) = app.provider_settings.connect_modal.clone() else {
                return Task::none();
            };
            if m.api_key.trim().is_empty() {
                app.provider_settings.connect_error = Some("请输入 API 密钥".to_string());
                return Task::none();
            }
            app.provider_settings.loading = true;
            app.provider_settings.connect_error = None;
            Task::perform(connect_provider(m.provider_id, m.api_key), |res| {
                Message::Settings(SettingsMessage::ProviderConnectDone(res))
            })
        }
        SettingsMessage::ProviderConnectDone(res) => match res {
            Ok((providers, patterns, configured, catalog)) => {
                app.provider_settings.loading = false;
                app.provider_settings.providers = summarize_providers(providers);
                if configured {
                    replace_popular_patterns(app, patterns);
                }
                if !catalog.is_empty() {
                    app.provider_settings.catalog_items = catalog;
                }
                app.provider_settings.connect_modal = None;
                app.provider_settings.connect_error = None;
                Task::none()
            }
            Err(e) => {
                app.provider_settings.loading = false;
                app.provider_settings.connect_error = Some(e);
                Task::none()
            }
        },
        SettingsMessage::ProviderDisconnectRequested(provider_id) => {
            app.provider_settings.disconnect_confirm_provider_id = Some(provider_id);
            Task::none()
        }
        SettingsMessage::ProviderDisconnectCanceled => {
            app.provider_settings.disconnect_confirm_provider_id = None;
            Task::none()
        }
        SettingsMessage::ProviderDisconnectConfirmed(provider_id) => {
            app.provider_settings.disconnect_confirm_provider_id = None;
            app.provider_settings.loading = true;
            app.provider_settings.connect_error = None;
            Task::perform(disconnect_provider(provider_id), |res| {
                Message::Settings(SettingsMessage::ProviderDisconnectDone(res))
            })
        }
        SettingsMessage::ProviderDisconnect(provider_id) => {
            app.provider_settings.disconnect_confirm_provider_id = None;
            app.provider_settings.loading = true;
            app.provider_settings.connect_error = None;
            Task::perform(disconnect_provider(provider_id), |res| {
                Message::Settings(SettingsMessage::ProviderDisconnectDone(res))
            })
        }
        SettingsMessage::ProviderDisconnectDone(res) => match res {
            Ok((providers, patterns, configured, catalog)) => {
                app.provider_settings.loading = false;
                app.provider_settings.providers = summarize_providers(providers);
                if configured {
                    replace_popular_patterns(app, patterns);
                }
                if !catalog.is_empty() {
                    app.provider_settings.catalog_items = catalog;
                }
                app.provider_settings.disconnect_confirm_provider_id = None;
                Task::none()
            }
            Err(e) => {
                app.provider_settings.loading = false;
                app.provider_settings.save_error = Some(e);
                app.provider_settings.disconnect_confirm_provider_id = None;
                Task::none()
            }
        },
        SettingsMessage::CustomProviderOpen => {
            app.provider_settings.custom_provider_modal_open = true;
            app.provider_settings.custom_editing_provider_id = None;
            app.provider_settings.custom_models_discovering = false;
            app.provider_settings.custom = CustomProviderDraft::default();
            app.provider_settings.save_error = None;
            app.provider_settings.connect_modal = None;
            app.provider_settings.connect_error = None;
            app.provider_settings.catalog_open = false;
            app.provider_settings.custom_model_modal = None;
            app.provider_settings.disconnect_confirm_provider_id = None;
            Task::none()
        }
        SettingsMessage::CustomProviderEditOpen(provider_id) => {
            app.provider_settings.loading = true;
            app.provider_settings.custom_models_discovering = false;
            app.provider_settings.save_error = None;
            app.provider_settings.connect_modal = None;
            app.provider_settings.connect_error = None;
            app.provider_settings.catalog_open = false;
            app.provider_settings.custom_model_modal = None;
            app.provider_settings.custom_provider_modal_open = false;
            app.provider_settings.custom_editing_provider_id = None;
            Task::perform(load_custom_provider_draft(provider_id), |res| {
                Message::Settings(SettingsMessage::CustomProviderEditLoaded(res))
            })
        }
        SettingsMessage::CustomProviderEditLoaded(res) => match res {
            Ok(draft) => {
                app.provider_settings.loading = false;
                app.provider_settings.custom_models_discovering = false;
                app.provider_settings.custom_editing_provider_id = Some(draft.provider_id.clone());
                app.provider_settings.custom = draft;
                app.provider_settings.custom_provider_modal_open = true;
                Task::none()
            }
            Err(e) => {
                app.provider_settings.loading = false;
                app.provider_settings.custom_models_discovering = false;
                app.provider_settings.save_error = Some(e);
                Task::none()
            }
        },
        SettingsMessage::CustomProviderClose => {
            app.provider_settings.custom_provider_modal_open = false;
            app.provider_settings.custom_editing_provider_id = None;
            app.provider_settings.custom_models_discovering = false;
            app.provider_settings.save_error = None;
            Task::none()
        }
        SettingsMessage::CustomProviderIdChanged(v) => {
            app.provider_settings.custom.provider_id = v;
            Task::none()
        }
        SettingsMessage::CustomProviderNameChanged(v) => {
            app.provider_settings.custom.display_name = v;
            Task::none()
        }
        SettingsMessage::CustomProviderBaseUrlChanged(v) => {
            app.provider_settings.custom.base_url = v;
            Task::none()
        }
        SettingsMessage::CustomProviderApiKeyChanged(v) => {
            app.provider_settings.custom.api_key = v;
            Task::none()
        }
        SettingsMessage::CustomProviderHeaderAdd => {
            app.provider_settings.custom.headers.push(ProviderHeaderDraft::default());
            Task::none()
        }
        SettingsMessage::CustomProviderHeaderRemove(idx) => {
            if app.provider_settings.custom.headers.len() > 1
                && idx < app.provider_settings.custom.headers.len()
            {
                app.provider_settings.custom.headers.remove(idx);
            }
            Task::none()
        }
        SettingsMessage::CustomProviderHeaderKeyChanged(idx, v) => {
            if let Some(h) = app.provider_settings.custom.headers.get_mut(idx) {
                h.key = v;
            }
            Task::none()
        }
        SettingsMessage::CustomProviderHeaderValueChanged(idx, v) => {
            if let Some(h) = app.provider_settings.custom.headers.get_mut(idx) {
                h.value = v;
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelOpen(edit_index) => {
            let (model_id, display_name) = edit_index
                .and_then(|i| app.provider_settings.custom.models.get(i).cloned())
                .map(|m| (m.model_id, m.display_name))
                .unwrap_or_default();
            app.provider_settings.custom_model_modal =
                Some(CustomProviderModelModalState { edit_index, model_id, display_name });
            app.provider_settings.catalog_open = false;
            app.provider_settings.connect_modal = None;
            Task::none()
        }
        SettingsMessage::CustomProviderModelClose => {
            app.provider_settings.custom_model_modal = None;
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalIdChanged(v) => {
            if let Some(m) = &mut app.provider_settings.custom_model_modal {
                m.model_id = v;
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalNameChanged(v) => {
            if let Some(m) = &mut app.provider_settings.custom_model_modal {
                m.display_name = v;
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalSave => {
            let Some(m) = app.provider_settings.custom_model_modal.clone() else {
                return Task::none();
            };
            let model_id = m.model_id.trim().to_string();
            if model_id.is_empty() {
                app.provider_settings.save_error = Some("模型 ID 不能为空".to_string());
                return Task::none();
            }
            let new_model = CustomProviderModelDraft {
                model_id,
                display_name: m.display_name.trim().to_string(),
            };
            if let Some(i) = m.edit_index {
                if i < app.provider_settings.custom.models.len() {
                    app.provider_settings.custom.models[i] = new_model;
                }
            } else {
                app.provider_settings.custom.models.push(new_model);
            }
            app.provider_settings.custom_model_modal = None;
            Task::none()
        }
        SettingsMessage::CustomProviderModelRemove(idx) => {
            if idx < app.provider_settings.custom.models.len() {
                app.provider_settings.custom.models.remove(idx);
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelsDiscover => {
            if !app.provider_settings.custom_provider_modal_open
                || app.provider_settings.custom_editing_provider_id.is_none()
                || app.provider_settings.custom_models_discovering
            {
                return Task::none();
            }
            app.provider_settings.custom_models_discovering = true;
            app.provider_settings.save_error = None;
            let draft = app.provider_settings.custom.clone();
            let provider_id = draft.provider_id.clone();
            Task::perform(discover_custom_provider_models(draft), move |result| {
                Message::Settings(SettingsMessage::CustomProviderModelsDiscovered {
                    provider_id: provider_id.clone(),
                    result,
                })
            })
        }
        SettingsMessage::CustomProviderModelsDiscovered { provider_id, result } => {
            if app.provider_settings.custom_editing_provider_id.as_deref()
                != Some(provider_id.as_str())
            {
                return Task::none();
            }
            app.provider_settings.custom_models_discovering = false;
            if !app.provider_settings.custom_provider_modal_open {
                return Task::none();
            }
            match result {
                Ok(models) => {
                    merge_discovered_model_drafts(&mut app.provider_settings.custom.models, models);
                    app.provider_settings.save_error = None;
                }
                Err(error) => {
                    app.provider_settings.save_error = Some(error);
                }
            }
            Task::none()
        }
        SettingsMessage::CustomProviderSave => {
            if app.provider_settings.custom_models_discovering {
                return Task::none();
            }
            let draft = app.provider_settings.custom.clone();
            let provider_id = draft.provider_id.trim().to_string();
            if !is_valid_provider_id(&provider_id) {
                app.provider_settings.save_error =
                    Some("提供商 ID 仅支持小写字母、数字、连字符或下划线".to_string());
                return Task::none();
            }
            let base_url = draft.base_url.trim().to_string();
            if base_url.is_empty() {
                app.provider_settings.save_error = Some("基础 URL 不能为空".to_string());
                return Task::none();
            }

            let has_models = draft.models.iter().any(|m| !m.model_id.trim().is_empty());
            if !has_models {
                app.provider_settings.save_error = Some("至少添加一个模型".to_string());
                return Task::none();
            }

            app.provider_settings.loading = true;
            app.provider_settings.save_error = None;
            Task::perform(save_custom_provider(draft), |res| {
                Message::Settings(SettingsMessage::CustomProviderSaveDone(res))
            })
        }
        SettingsMessage::CustomProviderSaveDone(res) => match res {
            Ok((providers, patterns, configured, catalog)) => {
                app.provider_settings.loading = false;
                app.provider_settings.custom_models_discovering = false;
                app.provider_settings.providers = summarize_providers(providers);
                if configured {
                    replace_popular_patterns(app, patterns);
                }
                if !catalog.is_empty() {
                    app.provider_settings.catalog_items = catalog;
                }
                app.provider_settings.custom = CustomProviderDraft::default();
                app.provider_settings.custom_editing_provider_id = None;
                app.provider_settings.save_error = None;
                app.provider_settings.custom_provider_modal_open = false;
                Task::none()
            }
            Err(e) => {
                app.provider_settings.loading = false;
                app.provider_settings.custom_models_discovering = false;
                app.provider_settings.save_error = Some(e);
                Task::none()
            }
        },
        SettingsMessage::PopularProviderRemove(idx) => {
            if app
                .provider_settings
                .popular_patterns
                .get(idx)
                .is_some_and(|pattern| is_shiju_official_provider_id(pattern))
            {
                return Task::none();
            }
            if idx < app.provider_settings.popular_patterns.len() {
                app.provider_settings.popular_patterns.remove(idx);
            }
            app.provider_settings.save_error = None;
            save_popular_providers_task(app.provider_settings.popular_patterns.clone())
        }
        SettingsMessage::PopularProvidersSaved(res) => {
            if let Err(e) = res {
                app.provider_settings.save_error = Some(e);
            }
            Task::none()
        }
        SettingsMessage::ProviderCatalogOpen => {
            app.provider_settings.catalog_open = true;
            app.provider_settings.catalog_query.clear();
            app.provider_settings.save_error = None;
            app.provider_settings.connect_modal = None;
            app.provider_settings.custom_model_modal = None;
            app.provider_settings.custom_provider_modal_open = false;
            app.provider_settings.catalog_loading = app.provider_settings.catalog_items.is_empty();
            load_catalog_task()
        }
        SettingsMessage::ProviderCatalogClose => {
            app.provider_settings.catalog_open = false;
            Task::none()
        }
        SettingsMessage::ProviderCatalogQueryChanged(v) => {
            app.provider_settings.catalog_query = v;
            Task::none()
        }
        SettingsMessage::ProviderCatalogLoaded(res) => match res {
            Ok(items) => {
                app.provider_settings.catalog_items = items;
                app.provider_settings.catalog_loading = false;
                Task::none()
            }
            Err(e) => {
                app.provider_settings.catalog_loading = false;
                app.provider_settings.save_error = Some(e);
                Task::none()
            }
        },
        SettingsMessage::ProviderCatalogAddToPopular(provider_id) => {
            if is_shiju_official_provider_id(&provider_id) {
                return Task::none();
            }
            if !app.provider_settings.popular_patterns.iter().any(|s| s == &provider_id) {
                app.provider_settings.popular_patterns.push(provider_id);
            }
            app.provider_settings.save_error = None;
            save_popular_providers_task(app.provider_settings.popular_patterns.clone())
        }
        _ => Task::none(),
    }
}
#[cfg(test)]
#[path = "updates_tests.rs"]
mod updates_tests;
