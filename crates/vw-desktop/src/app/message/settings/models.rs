//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::components::input_panel::MODEL_POPOVER_SCROLL_ID;
use crate::app::config;
use crate::app::i18n::settings_models::messages::Feedback;
use crate::app::{App, Message, state::ProviderModelsSummary};
use iced::Task;

use super::messages::SettingsMessage;
use super::types::ModelStatusChange;
use super::util::load_gateway_providers;

pub(super) use desktop_settings_providers::models::summarize_models;
async fn load_models_for_settings() -> Result<Vec<ProviderModelsSummary>, Feedback> {
    Ok(summarize_models(load_gateway_providers().await.map_err(Feedback::gateway)?))
}

use desktop_settings_providers::models::{collect_model_status_changes, collect_provider_model_status_changes, model_status_patch, provider_model_status_reset_patch, set_local_model_status};

fn changes_disable_selected_model(app: &App, changes: &[ModelStatusChange]) -> bool {
    changes.iter().any(|change| {
        if change.enabled {
            return false;
        }
        let full = format!("{}/{}", change.provider_id, change.model_id);
        if app.model.contains('/') { app.model == full } else { app.model == change.model_id }
    })
}

fn begin_model_status_save(app: &mut App, changes: &[ModelStatusChange]) -> Option<Task<Message>> {
    if changes.is_empty() || app.model_settings.loading || app.model_settings.saving {
        return None;
    }

    let preferences = if !app.auto_model && changes_disable_selected_model(app, changes) {
        app.save_automatic_model_preference()
    } else {
        Task::none()
    };

    for change in changes {
        set_local_model_status(&mut app.model_settings.providers, change, change.enabled);
    }
    app.model_settings.saving = true;
    app.model_settings.save_error = None;
    app.model_settings.detail_modal = None;
    Some(preferences)
}

fn save_model_statuses(app: &mut App, changes: Vec<ModelStatusChange>) -> Task<Message> {
    let Some(preferences) = begin_model_status_save(app, &changes) else {
        return Task::none();
    };
    let save = Task::perform(
        async move {
            let patch = model_status_patch(&changes);
            let result =
                config::patch_full_agent_config_async(patch).await.map_err(Feedback::gateway);
            (changes, result)
        },
        |(changes, result)| Message::Settings(SettingsMessage::ModelStatusesSaved(changes, result)),
    );
    Task::batch([preferences, save])
}

fn reset_provider_model_statuses(
    app: &mut App,
    provider_id: String,
    changes: Vec<ModelStatusChange>,
) -> Task<Message> {
    let Some(preferences) = begin_model_status_save(app, &changes) else {
        return Task::none();
    };

    let save = Task::perform(
        async move {
            let result = async {
                let full_config =
                    config::load_full_agent_config_async().await.map_err(Feedback::gateway)?;
                if let Some(patch) = provider_model_status_reset_patch(&full_config, &provider_id) {
                    config::patch_full_agent_config_async(patch)
                        .await
                        .map_err(Feedback::gateway)?;
                }
                load_models_for_settings().await
            }
            .await;
            (changes, result)
        },
        |(changes, result)| {
            Message::Settings(SettingsMessage::ProviderModelsResetCompleted(changes, result))
        },
    );
    Task::batch([preferences, save])
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;

/// 处理 `models_refresh_task` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn models_refresh_task() -> Task<Message> {
    Task::perform(async move { load_models_for_settings().await }, |res| {
        Message::Settings(SettingsMessage::ModelsRefreshed(res))
    })
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::ModelsRefresh => {
            if app.model_settings.saving {
                return Task::none();
            }
            app.model_settings.loading = true;
            app.model_settings.save_error = None;
            app.model_settings.detail_modal = None;
            models_refresh_task()
        }
        SettingsMessage::ModelsRefreshed(res) => match res {
            Ok(providers) => {
                app.model_settings.loading = false;
                app.model_settings.providers = providers;
                app.model_settings.detail_modal = None;
                Task::none()
            }
            Err(e) => {
                app.model_settings.loading = false;
                app.model_settings.save_error = Some((e).into());
                app.model_settings.detail_modal = None;
                Task::none()
            }
        },
        SettingsMessage::ModelQueryChanged(v) => {
            let reset_chat_results = app.show_model_popover && app.model_settings.query != v;
            app.model_settings.query = v;
            if reset_chat_results {
                iced::widget::operation::scroll_to(
                    iced::widget::Id::new(MODEL_POPOVER_SCROLL_ID),
                    iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
                )
            } else {
                Task::none()
            }
        }
        SettingsMessage::ModelToggle(provider_id, model_id, enabled) => {
            let change = app
                .model_settings
                .providers
                .iter()
                .find(|provider| provider.id == provider_id)
                .and_then(|provider| provider.models.iter().find(|model| model.id == model_id))
                .filter(|model| model.enabled != enabled)
                .map(|model| ModelStatusChange {
                    provider_id,
                    model_id,
                    previous_enabled: model.enabled,
                    enabled,
                });
            save_model_statuses(app, change.into_iter().collect())
        }
        SettingsMessage::ModelsSetAll(enabled) => {
            let changes = collect_model_status_changes(&app.model_settings.providers, enabled);
            save_model_statuses(app, changes)
        }
        SettingsMessage::ProviderModelsSetAll(provider_id, enabled) => {
            let changes = collect_provider_model_status_changes(
                &app.model_settings.providers,
                &provider_id,
                enabled,
                false,
            );
            save_model_statuses(app, changes)
        }
        SettingsMessage::ProviderModelsReset(provider_id) => {
            let changes = collect_provider_model_status_changes(
                &app.model_settings.providers,
                &provider_id,
                false,
                true,
            );
            reset_provider_model_statuses(app, provider_id, changes)
        }
        SettingsMessage::ProviderModelsResetCompleted(changes, result) => {
            app.model_settings.saving = false;
            match result {
                Ok(providers) => {
                    app.model_settings.providers = providers;
                }
                Err(error) => {
                    for change in &changes {
                        set_local_model_status(
                            &mut app.model_settings.providers,
                            change,
                            change.previous_enabled,
                        );
                    }
                    app.model_settings.save_error = Some((error).into());
                }
            }
            Task::none()
        }
        SettingsMessage::ProviderModelsExpansionToggled(provider_id) => {
            if !app.model_settings.expanded_provider_ids.insert(provider_id.clone()) {
                app.model_settings.expanded_provider_ids.remove(&provider_id);
            }
            Task::none()
        }
        SettingsMessage::ModelStatusesSaved(changes, result) => {
            app.model_settings.saving = false;
            if let Err(error) = result {
                for change in &changes {
                    set_local_model_status(
                        &mut app.model_settings.providers,
                        change,
                        change.previous_enabled,
                    );
                }
                app.model_settings.save_error = Some((error).into());
            }
            Task::none()
        }
        SettingsMessage::ModelDetailOpen(provider_id, model_id) => {
            let provider = app.model_settings.providers.iter().find(|p| p.id == provider_id);
            let Some(provider) = provider else {
                app.model_settings.save_error = Some(Feedback::owned("未找到提供商", vec![]));
                return Task::none();
            };
            let model = provider.models.iter().find(|m| m.id == model_id);
            let Some(model) = model else {
                app.model_settings.save_error = Some(Feedback::owned("未找到模型", vec![]));
                return Task::none();
            };

            app.model_settings.detail_modal = Some(
                desktop_settings_providers::models::model_detail(provider, model),
            );
            Task::none()
        }
        SettingsMessage::ModelDetailClose => {
            app.model_settings.detail_modal = None;
            Task::none()
        }
        SettingsMessage::ModelDetailToggleRaw => {
            if let Some(m) = &mut app.model_settings.detail_modal {
                m.show_raw = !m.show_raw;
            }
            Task::none()
        }
        _ => Task::none(),
    }
}
