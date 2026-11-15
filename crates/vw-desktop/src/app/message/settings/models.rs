//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::components::input_panel::MODEL_POPOVER_SCROLL_ID;
use crate::app::config;
use crate::app::{App, Message};
use iced::Task;

use super::messages::SettingsMessage;
use super::types::ModelStatusChange;

pub(super) use desktop_settings_providers::models::summarize_models;
fn begin_model_status_save(app: &mut App, changes: &[ModelStatusChange]) -> Option<Task<Message>> {
    if !desktop_settings_providers::models::can_begin_status_save(&app.model_settings, changes) {
        return None;
    }

    let preferences = if !app.auto_model && desktop_settings_providers::models::changes_disable_selected_model(&app.model, changes) {
        app.save_automatic_model_preference()
    } else {
        Task::none()
    };

    desktop_settings_providers::models::begin_status_save(&mut app.model_settings, changes);
    Some(preferences)
}

fn save_model_statuses(app: &mut App, changes: Vec<ModelStatusChange>) -> Task<Message> {
    let Some(preferences) = begin_model_status_save(app, &changes) else {
        return Task::none();
    };
    let save = Task::perform(
        async move { desktop_settings_providers::model_tasks::save_statuses(config::gateway_client_endpoint(), changes).await },
        map_model_result,
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
        async move { desktop_settings_providers::model_tasks::reset_statuses(config::gateway_client_endpoint(), provider_id, changes).await },
        map_model_result,
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
    Task::perform(async move { desktop_settings_providers::model_tasks::load_models_for_settings(config::gateway_client_endpoint()).await }, |res| {
        Message::Settings(SettingsMessage::ModelsRefreshed(res))
    })
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    use desktop_settings_providers::models as domain;
    let message = match message {
        SettingsMessage::ModelsRefresh => domain::ModelMessage::ModelsRefresh,
        SettingsMessage::ModelsRefreshed(v0) => domain::ModelMessage::ModelsRefreshed(v0),
        SettingsMessage::ModelQueryChanged(v0) => domain::ModelMessage::ModelQueryChanged(v0),
        SettingsMessage::ModelToggle(v0,v1,v2) => domain::ModelMessage::ModelToggle(v0,v1,v2),
        SettingsMessage::ModelsSetAll(v0) => domain::ModelMessage::ModelsSetAll(v0),
        SettingsMessage::ProviderModelsSetAll(v0,v1) => domain::ModelMessage::ProviderModelsSetAll(v0,v1),
        SettingsMessage::ProviderModelsReset(v0) => domain::ModelMessage::ProviderModelsReset(v0),
        SettingsMessage::ProviderModelsResetCompleted(v0,v1) => domain::ModelMessage::ProviderModelsResetCompleted(v0,v1),
        SettingsMessage::ProviderModelsExpansionToggled(v0) => domain::ModelMessage::ProviderModelsExpansionToggled(v0),
        SettingsMessage::ModelStatusesSaved(v0,v1) => domain::ModelMessage::ModelStatusesSaved(v0,v1),
        SettingsMessage::ModelDetailOpen(v0,v1) => domain::ModelMessage::ModelDetailOpen(v0,v1),
        SettingsMessage::ModelDetailClose => domain::ModelMessage::ModelDetailClose,
        SettingsMessage::ModelDetailToggleRaw => domain::ModelMessage::ModelDetailToggleRaw,
        _ => return Task::none(),
    };
    match domain::update(&mut app.model_settings, message) {
        domain::ModelEffect::None => Task::none(),
        domain::ModelEffect::Refresh => models_refresh_task(),
        domain::ModelEffect::Save(changes) => save_model_statuses(app, changes),
        domain::ModelEffect::Reset(provider, changes) => reset_provider_model_statuses(app, provider, changes),
        domain::ModelEffect::QueryChanged if app.show_model_popover => iced::widget::operation::scroll_to(
            iced::widget::Id::new(MODEL_POPOVER_SCROLL_ID),
            iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
        ),
        domain::ModelEffect::QueryChanged => Task::none(),
    }
}

fn map_model_result(message: desktop_settings_providers::models::ModelMessage) -> Message {
    use desktop_settings_providers::models::ModelMessage as Local;
    Message::Settings(match message {
        Local::ModelStatusesSaved(changes, result) => SettingsMessage::ModelStatusesSaved(changes, result),
        Local::ProviderModelsResetCompleted(changes, result) => SettingsMessage::ProviderModelsResetCompleted(changes, result),
        _ => unreachable!("model persistence only returns save completion messages"),
    })
}
