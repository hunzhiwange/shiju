//! 处理模型提供商设置子模块的目录、认证和连接状态。

use desktop_i18n::settings_models::messages::Feedback;
use crate::provider_state::*;
use crate::state::{CustomProviderModelModalState, CustomProviderModelTab, ModelSettingsState};
use crate::provider_message::{Message, SettingsMessage};
use desktop_config::ConfigAccess;
use gateway_endpoint::GatewayEndpoint;
use iced::Task;


use crate::provider_helpers::{is_valid_provider_id, summarize_providers};
use crate::provider_models::{
    current_provider_api_key,
    discover_custom_provider_models, merge_discovered_model_drafts,
    validate_model_drafts,
};
use crate::provider_refresh::{
    load_catalog_task, refresh_task, save_popular_providers_task, sync_remote_task,
};

fn reset_models_sync_state(state: &mut ProviderSettingsState) {
    state.models_syncing = false;
    state.models_sync_progress = 0.0;
    state.models_sync_label = String::new().into();
}

fn replace_popular_patterns(state: &mut ProviderSettingsState, patterns: Vec<String>) {
    state.popular_patterns =
        patterns.into_iter().filter(|pattern| !is_shiju_official_provider_id(pattern)).collect();
}

fn model_modal_state(
    edit_index: Option<usize>,
    model: Option<CustomProviderModelDraft>,
) -> CustomProviderModelModalState {
    let model = model.unwrap_or_default();
    CustomProviderModelModalState {
        active_tab: CustomProviderModelTab::default(),
        edit_index,
        model_id: model.model_id,
        display_name: model.display_name,
        family: model.family,
        context_limit: model.context_limit,
        input_limit: model.input_limit,
        output_limit: model.output_limit,
        reasoning: model.reasoning,
        tool_call: model.tool_call,
        temperature: model.temperature,
        attachment: model.attachment,
        interleaved: model.interleaved,
        input_modalities: model.input_modalities,
        output_modalities: model.output_modalities,
        inherited_capabilities: model.inherited_capabilities,
        original_model_id: model.original_model_id,
    }
}

fn toggle_modality(
    slot: &mut Option<Vec<String>>,
    modality: String,
    enabled: bool,
    inherited: Option<&[String]>,
) {
    let values = slot.get_or_insert_with(|| {
        inherited.map(ToOwned::to_owned).unwrap_or_else(|| vec!["text".to_string()])
    });
    if enabled {
        if !values.iter().any(|value| value == &modality) {
            values.push(modality);
        }
    } else {
        values.retain(|value| value != &modality);
    }
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
fn update_task(state: &mut ProviderSettingsState, models: &mut ModelSettingsState, endpoint: GatewayEndpoint, config: ConfigAccess, _account_active: bool, effect: &mut Option<Message>, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::ProvidersRefresh => {
            state.loading = true;
            state.connect_error = None;
            state.save_error = None;
            state.disconnect_confirm_provider_id = None;
            refresh_task(endpoint, config)
        }
        SettingsMessage::ProviderModelsSyncRemote => {
            if state.loading || state.models_syncing {
                return Task::none();
            }
            state.connect_error = None;
            state.save_error = None;
            state.disconnect_confirm_provider_id = None;
            state.models_syncing = true;
            state.models_sync_progress = 0.08;
            state.models_sync_label =
                Feedback::owned("正在从 models.dev 拉取模型目录…", vec![]);
            sync_remote_task(endpoint, config)
        }
        SettingsMessage::ProviderModelsSyncTick => {
            if !state.models_syncing {
                return Task::none();
            }
            let next = (state.models_sync_progress + 0.07).min(0.9);
            state.models_sync_progress = next;
            state.models_sync_label = if next < 0.35 {
                Feedback::owned("正在从 models.dev 拉取模型目录…", vec![])
            } else if next < 0.7 {
                Feedback::owned("正在写入本地缓存…", vec![])
            } else {
                Feedback::owned("正在刷新提供商视图…", vec![])
            };
            Task::none()
        }
        SettingsMessage::ProviderModelsSyncDone(res) => match res {
            Ok((providers, patterns, configured, catalog)) => {
                models.providers =
                    crate::models::summarize_models(providers.clone());
                state.providers = summarize_providers(providers);
                if configured {
                    replace_popular_patterns(state, patterns);
                }
                if !catalog.is_empty() {
                    state.catalog_items = catalog;
                }
                state.catalog_loading = false;
                reset_models_sync_state(state);
                Task::none()
            }
            Err(e) => {
                state.save_error = Some((e).into());
                state.catalog_loading = false;
                reset_models_sync_state(state);
                Task::none()
            }
        },
        SettingsMessage::ProvidersRefreshed(res) => match res {
            Ok((providers, patterns, configured, catalog)) => {
                state.loading = false;
                models.providers =
                    crate::models::summarize_models(providers.clone());
                state.providers = summarize_providers(providers);
                if configured {
                    replace_popular_patterns(state, patterns);
                }
                if !catalog.is_empty() {
                    state.catalog_items = catalog;
                }
                state.catalog_loading = false;
                Task::none()
            }
            Err(e) => {
                state.loading = false;
                state.save_error = Some((e).into());
                state.catalog_loading = false;
                Task::none()
            }
        },
        SettingsMessage::ProviderConnectOpen(provider_id) => {
            if is_shiju_official_provider_id(&provider_id) {
                state.connect_modal = None;
                #[cfg(not(target_arch = "wasm32"))]
                if !_account_active {
                    *effect = Some(Message::Warning(Feedback::owned("请先登录识矩账户，模型将自动配置", vec![])));
                    return Task::none();
                }
                #[cfg(not(target_arch = "wasm32"))]
                { *effect = Some(Message::SyncAccount); return Task::none(); }
                #[cfg(target_arch = "wasm32")]
                { *effect = Some(Message::Warning("请在桌面端登录识矩账户并配置模型".into())); return Task::none(); }
            }
            let provider_name = state
                .catalog_items
                .iter()
                .find(|p| p.provider_id == provider_id)
                .map(|p| p.provider_name.clone())
                .or_else(|| {
                    state
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
            state.connect_modal =
                Some(ProviderConnectState { provider_id, provider_name, api_key });
            state.connect_error = None;
            state.catalog_open = false;
            state.custom_model_modal = None;
            state.custom_provider_modal_open = false;
            state.custom_editing_provider_id = None;
            state.disconnect_confirm_provider_id = None;
            Task::none()
        }
        SettingsMessage::ProviderConnectClose => {
            state.connect_modal = None;
            state.connect_error = None;
            Task::none()
        }
        SettingsMessage::ProviderConnectApiKeyChanged(v) => {
            if let Some(m) = &mut state.connect_modal {
                m.api_key = v;
            }
            Task::none()
        }
        SettingsMessage::ProviderConnectSubmit => {
            let Some(m) = state.connect_modal.clone() else {
                return Task::none();
            };
            if is_shiju_official_provider_id(&m.provider_id) {
                #[cfg(not(target_arch = "wasm32"))]
                { *effect = Some(Message::SyncAccount); return Task::none(); }
                #[cfg(target_arch = "wasm32")]
                { *effect = Some(Message::Warning("请在桌面端登录识矩账户并配置模型".into())); return Task::none(); }
            }
            if m.api_key.trim().is_empty() {
                state.connect_error =
                    Some(Feedback::owned("请输入 API 密钥", vec![]));
                return Task::none();
            }
            state.loading = true;
            state.connect_error = None;
            Task::perform(crate::provider_tasks::connect_provider(endpoint, config, m.provider_id, m.api_key), |res| {
                Message::Settings(SettingsMessage::ProviderConnectDone(res))
            })
        }
        SettingsMessage::ProviderConnectDone(res) => match res {
            Ok((providers, patterns, configured, catalog)) => {
                state.loading = false;
                models.providers =
                    crate::models::summarize_models(providers.clone());
                state.providers = summarize_providers(providers);
                if configured {
                    replace_popular_patterns(state, patterns);
                }
                if !catalog.is_empty() {
                    state.catalog_items = catalog;
                }
                state.connect_modal = None;
                state.connect_error = None;
                Task::none()
            }
            Err(e) => {
                state.loading = false;
                state.connect_error = Some((e).into());
                Task::none()
            }
        },
        SettingsMessage::ProviderDisconnectRequested(provider_id) => {
            state.disconnect_confirm_provider_id = Some(provider_id);
            Task::none()
        }
        SettingsMessage::ProviderDisconnectCanceled => {
            state.disconnect_confirm_provider_id = None;
            Task::none()
        }
        SettingsMessage::ProviderDisconnectConfirmed(provider_id) => {
            state.disconnect_confirm_provider_id = None;
            state.loading = true;
            state.connect_error = None;
            Task::perform(crate::provider_tasks::disconnect_provider(endpoint, config, provider_id), |res| {
                Message::Settings(SettingsMessage::ProviderDisconnectDone(res))
            })
        }
        SettingsMessage::ProviderDisconnect(provider_id) => {
            state.disconnect_confirm_provider_id = None;
            state.loading = true;
            state.connect_error = None;
            Task::perform(crate::provider_tasks::disconnect_provider(endpoint, config, provider_id), |res| {
                Message::Settings(SettingsMessage::ProviderDisconnectDone(res))
            })
        }
        SettingsMessage::ProviderDisconnectDone(res) => match res {
            Ok((providers, patterns, configured, catalog)) => {
                state.loading = false;
                models.providers =
                    crate::models::summarize_models(providers.clone());
                state.providers = summarize_providers(providers);
                if configured {
                    replace_popular_patterns(state, patterns);
                }
                if !catalog.is_empty() {
                    state.catalog_items = catalog;
                }
                state.disconnect_confirm_provider_id = None;
                Task::none()
            }
            Err(e) => {
                state.loading = false;
                state.save_error = Some((e).into());
                state.disconnect_confirm_provider_id = None;
                Task::none()
            }
        },
        SettingsMessage::CustomProviderOpen => {
            state.custom_provider_modal_open = true;
            state.custom_provider_tab = CustomProviderTab::default();
            state.custom_editing_provider_id = None;
            state.custom_models_discovering = false;
            state.custom_models_filter = CustomProviderModelFilter::default();
            state.custom = CustomProviderDraft::default();
            state.save_error = None;
            state.connect_modal = None;
            state.connect_error = None;
            state.catalog_open = false;
            state.custom_model_modal = None;
            state.disconnect_confirm_provider_id = None;
            Task::none()
        }
        SettingsMessage::CustomProviderEditOpen(provider_id) => {
            state.loading = true;
            state.custom_provider_tab = CustomProviderTab::default();
            state.custom_models_discovering = false;
            state.custom_models_filter = CustomProviderModelFilter::default();
            state.save_error = None;
            state.connect_modal = None;
            state.connect_error = None;
            state.catalog_open = false;
            state.custom_model_modal = None;
            state.custom_provider_modal_open = false;
            state.custom_editing_provider_id = None;
            Task::perform(crate::provider_tasks::load_custom_provider_draft(config, provider_id), |res| {
                Message::Settings(SettingsMessage::CustomProviderEditLoaded(res))
            })
        }
        SettingsMessage::CustomProviderEditLoaded(res) => match res {
            Ok(draft) => {
                state.loading = false;
                state.custom_models_discovering = false;
                state.custom_editing_provider_id = Some(draft.provider_id.clone());
                state.custom = draft;
                state.custom_provider_tab = CustomProviderTab::default();
                state.custom_provider_modal_open = true;
                Task::none()
            }
            Err(e) => {
                state.loading = false;
                state.custom_models_discovering = false;
                state.save_error = Some((e).into());
                Task::none()
            }
        },
        SettingsMessage::CustomProviderClose => {
            state.custom_provider_modal_open = false;
            state.custom_editing_provider_id = None;
            state.custom_models_discovering = false;
            state.save_error = None;
            Task::none()
        }
        SettingsMessage::CustomProviderTabSelected(tab) => {
            state.custom_provider_tab = tab;
            Task::none()
        }
        SettingsMessage::CustomProviderIdChanged(v) => {
            state.custom.provider_id = v;
            Task::none()
        }
        SettingsMessage::CustomProviderNameChanged(v) => {
            state.custom.display_name = v;
            Task::none()
        }
        SettingsMessage::CustomProviderBaseUrlChanged(v) => {
            state.custom.base_url = v;
            Task::none()
        }
        SettingsMessage::CustomProviderApiKeyChanged(v) => {
            state.custom.api_key = v;
            Task::none()
        }
        SettingsMessage::CustomProviderHeaderAdd => {
            state.custom.headers.push(ProviderHeaderDraft::default());
            Task::none()
        }
        SettingsMessage::CustomProviderHeaderRemove(idx) => {
            if state.custom.headers.len() > 1
                && idx < state.custom.headers.len()
            {
                state.custom.headers.remove(idx);
            }
            Task::none()
        }
        SettingsMessage::CustomProviderHeaderKeyChanged(idx, v) => {
            if let Some(h) = state.custom.headers.get_mut(idx) {
                h.key = v;
            }
            Task::none()
        }
        SettingsMessage::CustomProviderHeaderValueChanged(idx, v) => {
            if let Some(h) = state.custom.headers.get_mut(idx) {
                h.value = v;
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelOpen(edit_index) => {
            let model =
                edit_index.and_then(|i| state.custom.models.get(i).cloned());
            state.custom_model_modal = Some(model_modal_state(edit_index, model));
            state.save_error = None;
            state.catalog_open = false;
            state.connect_modal = None;
            Task::none()
        }
        SettingsMessage::CustomProviderModelClose => {
            state.custom_model_modal = None;
            Task::none()
        }
        SettingsMessage::CustomProviderModelTabSelected(tab) => {
            if let Some(model) = &mut state.custom_model_modal {
                model.active_tab = tab;
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalIdChanged(v) => {
            if let Some(m) = &mut state.custom_model_modal {
                m.model_id = v;
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalNameChanged(v) => {
            if let Some(m) = &mut state.custom_model_modal {
                m.display_name = v;
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalFamilyChanged(v) => {
            if let Some(m) = &mut state.custom_model_modal {
                m.family = v;
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalContextLimitChanged(v) => {
            if let Some(m) = &mut state.custom_model_modal {
                m.context_limit = v;
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalInputLimitChanged(v) => {
            if let Some(m) = &mut state.custom_model_modal {
                m.input_limit = v;
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalOutputLimitChanged(v) => {
            if let Some(m) = &mut state.custom_model_modal {
                m.output_limit = v;
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalReasoningChanged(v) => {
            if let Some(m) = &mut state.custom_model_modal {
                m.reasoning = Some(v);
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalToolCallChanged(v) => {
            if let Some(m) = &mut state.custom_model_modal {
                m.tool_call = Some(v);
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalTemperatureChanged(v) => {
            if let Some(m) = &mut state.custom_model_modal {
                m.temperature = Some(v);
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalAttachmentChanged(v) => {
            if let Some(m) = &mut state.custom_model_modal {
                m.attachment = Some(v);
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalInterleavedChanged(v) => {
            if let Some(m) = &mut state.custom_model_modal {
                m.interleaved = Some(v);
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalInputModalityToggled(modality, enabled) => {
            if let Some(m) = &mut state.custom_model_modal {
                let inherited = m
                    .inherited_capabilities
                    .as_ref()
                    .map(|capabilities| capabilities.input_modalities.clone());
                toggle_modality(&mut m.input_modalities, modality, enabled, inherited.as_deref());
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalOutputModalityToggled(modality, enabled) => {
            if let Some(m) = &mut state.custom_model_modal {
                let inherited = m
                    .inherited_capabilities
                    .as_ref()
                    .map(|capabilities| capabilities.output_modalities.clone());
                toggle_modality(&mut m.output_modalities, modality, enabled, inherited.as_deref());
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelModalSave => {
            let Some(m) = state.custom_model_modal.clone() else {
                return Task::none();
            };
            let model_id = m.model_id.trim().to_string();
            if model_id.is_empty() {
                state.save_error =
                    Some(Feedback::owned("模型 ID 不能为空", vec![]));
                return Task::none();
            }
            let new_model = CustomProviderModelDraft {
                model_id,
                display_name: m.display_name.trim().to_string(),
                family: m.family.trim().to_string(),
                context_limit: m.context_limit.trim().to_string(),
                input_limit: m.input_limit.trim().to_string(),
                output_limit: m.output_limit.trim().to_string(),
                reasoning: m.reasoning,
                tool_call: m.tool_call,
                temperature: m.temperature,
                attachment: m.attachment,
                interleaved: m.interleaved,
                input_modalities: m.input_modalities,
                output_modalities: m.output_modalities,
                inherited_capabilities: m.inherited_capabilities,
                enabled: m
                    .edit_index
                    .and_then(|index| state.custom.models.get(index))
                    .map(|model| model.enabled)
                    .unwrap_or(true),
                from_catalog: m
                    .edit_index
                    .and_then(|index| state.custom.models.get(index))
                    .map(|model| model.from_catalog)
                    .unwrap_or(false),
                original_model_id: m.original_model_id,
            };
            if state
                .custom
                .models
                .iter()
                .enumerate()
                .filter(|(index, _)| Some(*index) != m.edit_index)
                .any(|(_, model)| model.model_id.trim() == new_model.model_id)
            {
                state.save_error =
                    Some(Feedback::owned("模型 ID 已存在：%s", vec![new_model.model_id.clone()]));
                return Task::none();
            }
            let mut next_models = state.custom.models.clone();
            if let Some(i) = m.edit_index {
                if i >= next_models.len() {
                    state.custom_model_modal = None;
                    return Task::none();
                }
                next_models[i] = new_model.clone();
            } else {
                next_models.push(new_model.clone());
            }
            if let Err(error) = validate_model_drafts(&next_models) {
                state.save_error = Some((error).into());
                return Task::none();
            }
            if let Some(i) = m.edit_index {
                state.custom.models[i] = new_model;
            } else {
                state.custom.models.push(new_model);
            }
            state.save_error = None;
            state.custom_model_modal = None;
            Task::none()
        }
        SettingsMessage::CustomProviderModelRemove(idx) => {
            if let Some(model) = state.custom.models.get_mut(idx) {
                if model.from_catalog {
                    model.enabled = false;
                } else {
                    state.custom.models.remove(idx);
                }
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelEnabledToggled(idx, enabled) => {
            if let Some(model) = state.custom.models.get_mut(idx) {
                model.enabled = enabled;
            }
            Task::none()
        }
        SettingsMessage::CustomProviderModelFilterSelected(filter) => {
            state.custom_models_filter = filter;
            Task::none()
        }
        SettingsMessage::CustomProviderModelsDiscover => {
            if !state.custom_provider_modal_open
                || state.custom_models_discovering
            {
                return Task::none();
            }
            state.custom_models_discovering = true;
            state.save_error = None;
            let draft = state.custom.clone();
            let provider_id = draft.provider_id.clone();
            Task::perform(discover_custom_provider_models(draft), move |result| {
                Message::Settings(SettingsMessage::CustomProviderModelsDiscovered {
                    provider_id: provider_id.clone(),
                    result,
                })
            })
        }
        SettingsMessage::CustomProviderModelsDiscovered { provider_id, result } => {
            if state.custom.provider_id.trim() != provider_id {
                return Task::none();
            }
            state.custom_models_discovering = false;
            if !state.custom_provider_modal_open {
                return Task::none();
            }
            match result {
                Ok(models) => {
                    merge_discovered_model_drafts(&mut state.custom.models, models);
                    state.save_error = None;
                }
                Err(error) => {
                    state.save_error = Some((error).into());
                }
            }
            Task::none()
        }
        SettingsMessage::CustomProviderSave => {
            if state.custom_models_discovering {
                return Task::none();
            }
            let draft = state.custom.clone();
            let provider_id = draft.provider_id.trim().to_string();
            if !is_valid_provider_id(&provider_id) {
                state.save_error =
                    Some(Feedback::owned("提供商 ID 仅支持小写字母、数字、连字符或下划线", vec![]));
                return Task::none();
            }
            let base_url = draft.base_url.trim().to_string();
            if base_url.is_empty() {
                state.save_error =
                    Some(Feedback::owned("基础 URL 不能为空", vec![]));
                return Task::none();
            }

            if let Err(error) = validate_model_drafts(&draft.models) {
                state.save_error = Some((error).into());
                return Task::none();
            }

            state.loading = true;
            state.save_error = None;
            Task::perform(crate::provider_tasks::save_custom_provider(endpoint, config, draft), |res| {
                Message::Settings(SettingsMessage::CustomProviderSaveDone(res))
            })
        }
        SettingsMessage::CustomProviderSaveDone(res) => match res {
            Ok((providers, patterns, configured, catalog)) => {
                state.loading = false;
                state.custom_models_discovering = false;
                models.providers =
                    crate::models::summarize_models(providers.clone());
                state.providers = summarize_providers(providers);
                if configured {
                    replace_popular_patterns(state, patterns);
                }
                if !catalog.is_empty() {
                    state.catalog_items = catalog;
                }
                state.custom = CustomProviderDraft::default();
                state.custom_editing_provider_id = None;
                state.save_error = None;
                state.custom_provider_modal_open = false;
                Task::perform(
                    async { provider_resolution::invalidate_cache().await },
                    |_| Message::Settings(SettingsMessage::ProvidersRefresh),
                )
            }
            Err(e) => {
                state.loading = false;
                state.custom_models_discovering = false;
                state.save_error = Some((e).into());
                Task::none()
            }
        },
        SettingsMessage::PopularProviderRemove(idx) => {
            if state
                .popular_patterns
                .get(idx)
                .is_some_and(|pattern| is_shiju_official_provider_id(pattern))
            {
                return Task::none();
            }
            if idx < state.popular_patterns.len() {
                state.popular_patterns.remove(idx);
            }
            state.save_error = None;
            save_popular_providers_task(config, state.popular_patterns.clone())
        }
        SettingsMessage::PopularProvidersSaved(res) => {
            if let Err(e) = res {
                state.save_error = Some((e).into());
            }
            Task::none()
        }
        SettingsMessage::ProviderCatalogOpen => {
            state.catalog_open = true;
            state.catalog_query.clear();
            state.save_error = None;
            state.connect_modal = None;
            state.custom_model_modal = None;
            state.custom_provider_modal_open = false;
            state.catalog_loading = state.catalog_items.is_empty();
            load_catalog_task(endpoint)
        }
        SettingsMessage::ProviderCatalogClose => {
            state.catalog_open = false;
            Task::none()
        }
        SettingsMessage::ProviderCatalogQueryChanged(v) => {
            state.catalog_query = v;
            Task::none()
        }
        SettingsMessage::ProviderCatalogLoaded(res) => match res {
            Ok(items) => {
                state.catalog_items = items;
                state.catalog_loading = false;
                Task::none()
            }
            Err(e) => {
                state.catalog_loading = false;
                state.save_error = Some((e).into());
                Task::none()
            }
        },
        SettingsMessage::ProviderCatalogAddToPopular(provider_id) => {
            if is_shiju_official_provider_id(&provider_id) {
                return Task::none();
            }
            if !state.popular_patterns.iter().any(|s| s == &provider_id) {
                state.popular_patterns.push(provider_id);
            }
            state.save_error = None;
            save_popular_providers_task(config, state.popular_patterns.clone())
        }
    }
}
#[cfg(test)]
#[path = "provider_update_tests.rs"]
mod updates_tests;

pub fn update(state: &mut ProviderSettingsState, models: &mut ModelSettingsState, endpoint: GatewayEndpoint, config: ConfigAccess, _account_active: bool, message: SettingsMessage) -> (Task<Message>, Option<Message>) {
    let mut effect = None;
    let task = update_task(state, models, endpoint, config, _account_active, &mut effect, message);
    (task, effect)
}
