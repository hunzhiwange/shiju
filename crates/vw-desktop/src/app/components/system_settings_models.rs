//! 组合模型设置子页，并将领域消息与弹窗映射到桌面壳。

use crate::app::components::system_settings_common::settings_modal_overlay;
use crate::app::message::settings::{ModelConfigTabMessage, SettingsMessage};
use crate::app::state::ModelConfigTab;
use crate::app::{App, Message};
use iced::Element;

pub(super) use desktop_settings_providers::models_page::MODEL_TABS;

pub(super) fn active_tab_save_error(app: &App) -> Option<&str> {
    match app.model_settings.active_tab {
        ModelConfigTab::Models => app.model_settings.save_error.as_deref(),
        ModelConfigTab::ModelRoutes => app.model_routes_settings.save_error.as_deref(),
        ModelConfigTab::QueryClassification => {
            app.query_classification_settings.save_error.as_deref()
        }
        ModelConfigTab::Multimodal => app.multimodal_settings.save_error.as_deref(),
    }
}

/// 渲染模型配置页及其内部标签。
pub fn view(app: &App) -> Element<'_, Message> {
    let active_tab = app.model_settings.active_tab;
    let content = match active_tab {
        ModelConfigTab::Models => main_view(app),
        ModelConfigTab::ModelRoutes => {
            crate::app::components::system_settings_model_routes::view(app)
        }
        ModelConfigTab::QueryClassification => {
            crate::app::components::system_settings_query_classification::view(app)
        }
        ModelConfigTab::Multimodal => crate::app::components::system_settings_multimodal::view(app),
    };

    let error = if let Some(error) = active_tab_save_error(app) {
        match app.model_settings.active_tab {
            ModelConfigTab::Models => {
                app.model_settings.save_error.as_ref().map(|e| e.display(app.general_settings.desktop_locale))
            }
            ModelConfigTab::ModelRoutes => {
                app.model_routes_settings.save_error.as_ref().map(|e| e.display(app.general_settings.desktop_locale))
            }
            ModelConfigTab::QueryClassification => Some(
                app.query_classification_settings
                    .save_error_translation
                    .as_ref()
                    .map(|message| {
                        message
                            .render(crate::app::i18n::I18n::new(app.general_settings.desktop_locale))
                            .expect("valid classification error template")
                    })
                    .unwrap_or_else(|| error.to_string()),
            ),
            _ => Some(error.to_string()),
        }
    } else {
        None
    };
    desktop_settings_providers::models_page::view(
        active_tab,
        app.general_settings.desktop_locale,
        content,
        error,
        |tab| {
            Message::Settings(SettingsMessage::ModelConfigTab(ModelConfigTabMessage::Selected(tab)))
        },
    )
}

/// 委派模型目录视图。
pub fn main_view(app: &App) -> Element<'_, Message> {
    desktop_settings_providers::models_view::main_view(&app.model_settings, view_style(app))
        .map(map_view_message)
}
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let Some(mm) = &app.model_settings.detail_modal else {
        return dialog;
    };
    let (close, card) = desktop_settings_providers::models_view::detail_card(mm, view_style(app));
    settings_modal_overlay(Some(dialog), map_view_message(close), card.map(map_view_message))
}

use desktop_settings_providers::view_message as domain;
pub(crate) fn map_view_message(message: domain::Message) -> crate::app::Message {
 use crate::app::{Message, message::settings::{SettingsMessage, ModelRoutesMessage}};
 match message {
 domain::Message::Catalog(event) => map_catalog_event(event),
 domain::Message::CopyCode(text) => Message::CopyCode(text),
 domain::Message::None => Message::None,
 domain::Message::Settings(message) => Message::Settings(match message {
        domain::SettingsMessage::ModelConfigTab(message) => SettingsMessage::ModelConfigTab(message),
        domain::SettingsMessage::CustomProviderApiKeyChanged(v0) => SettingsMessage::CustomProviderApiKeyChanged(v0),
        domain::SettingsMessage::CustomProviderBaseUrlChanged(v0) => SettingsMessage::CustomProviderBaseUrlChanged(v0),
        domain::SettingsMessage::CustomProviderClose => SettingsMessage::CustomProviderClose,
        domain::SettingsMessage::CustomProviderEditOpen(v0) => SettingsMessage::CustomProviderEditOpen(v0),
        domain::SettingsMessage::CustomProviderHeaderAdd => SettingsMessage::CustomProviderHeaderAdd,
        domain::SettingsMessage::CustomProviderHeaderKeyChanged(v0,v1) => SettingsMessage::CustomProviderHeaderKeyChanged(v0,v1),
        domain::SettingsMessage::CustomProviderHeaderRemove(v0) => SettingsMessage::CustomProviderHeaderRemove(v0),
        domain::SettingsMessage::CustomProviderHeaderValueChanged(v0,v1) => SettingsMessage::CustomProviderHeaderValueChanged(v0,v1),
        domain::SettingsMessage::CustomProviderIdChanged(v0) => SettingsMessage::CustomProviderIdChanged(v0),
        domain::SettingsMessage::CustomProviderModelEnabledToggled(v0,v1) => SettingsMessage::CustomProviderModelEnabledToggled(v0,v1),
        domain::SettingsMessage::CustomProviderModelFilterSelected(v0) => SettingsMessage::CustomProviderModelFilterSelected(v0),
        domain::SettingsMessage::CustomProviderModelOpen(v0) => SettingsMessage::CustomProviderModelOpen(v0),
        domain::SettingsMessage::CustomProviderModelRemove(v0) => SettingsMessage::CustomProviderModelRemove(v0),
        domain::SettingsMessage::CustomProviderModelsDiscover => SettingsMessage::CustomProviderModelsDiscover,
        domain::SettingsMessage::CustomProviderNameChanged(v0) => SettingsMessage::CustomProviderNameChanged(v0),
        domain::SettingsMessage::CustomProviderOpen => SettingsMessage::CustomProviderOpen,
        domain::SettingsMessage::CustomProviderSave => SettingsMessage::CustomProviderSave,
        domain::SettingsMessage::CustomProviderTabSelected(v0) => SettingsMessage::CustomProviderTabSelected(v0),
        domain::SettingsMessage::PopularProviderRemove(v0) => SettingsMessage::PopularProviderRemove(v0),
        domain::SettingsMessage::ProviderCatalogAddToPopular(v0) => SettingsMessage::ProviderCatalogAddToPopular(v0),
        domain::SettingsMessage::ProviderCatalogClose => SettingsMessage::ProviderCatalogClose,
        domain::SettingsMessage::ProviderCatalogOpen => SettingsMessage::ProviderCatalogOpen,
        domain::SettingsMessage::ProviderCatalogQueryChanged(v0) => SettingsMessage::ProviderCatalogQueryChanged(v0),
        domain::SettingsMessage::ProviderConnectApiKeyChanged(v0) => SettingsMessage::ProviderConnectApiKeyChanged(v0),
        domain::SettingsMessage::ProviderConnectClose => SettingsMessage::ProviderConnectClose,
        domain::SettingsMessage::ProviderConnectOpen(v0) => SettingsMessage::ProviderConnectOpen(v0),
        domain::SettingsMessage::ProviderConnectSubmit => SettingsMessage::ProviderConnectSubmit,
        domain::SettingsMessage::ProviderDisconnectCanceled => SettingsMessage::ProviderDisconnectCanceled,
        domain::SettingsMessage::ProviderDisconnectConfirmed(v0) => SettingsMessage::ProviderDisconnectConfirmed(v0),
        domain::SettingsMessage::ProviderDisconnectRequested(v0) => SettingsMessage::ProviderDisconnectRequested(v0),
        domain::SettingsMessage::ProviderModelsSyncRemote => SettingsMessage::ProviderModelsSyncRemote,
        domain::SettingsMessage::ProvidersRefresh => SettingsMessage::ProvidersRefresh,
        domain::SettingsMessage::EmbeddingRoutes(message) => SettingsMessage::EmbeddingRoutes(message),
        domain::SettingsMessage::CustomProviderModelClose => SettingsMessage::CustomProviderModelClose,
        domain::SettingsMessage::CustomProviderModelModalAttachmentChanged(v0) => SettingsMessage::CustomProviderModelModalAttachmentChanged(v0),
        domain::SettingsMessage::CustomProviderModelModalContextLimitChanged(v0) => SettingsMessage::CustomProviderModelModalContextLimitChanged(v0),
        domain::SettingsMessage::CustomProviderModelModalFamilyChanged(v0) => SettingsMessage::CustomProviderModelModalFamilyChanged(v0),
        domain::SettingsMessage::CustomProviderModelModalIdChanged(v0) => SettingsMessage::CustomProviderModelModalIdChanged(v0),
        domain::SettingsMessage::CustomProviderModelModalInputLimitChanged(v0) => SettingsMessage::CustomProviderModelModalInputLimitChanged(v0),
        domain::SettingsMessage::CustomProviderModelModalInputModalityToggled(v0,v1) => SettingsMessage::CustomProviderModelModalInputModalityToggled(v0,v1),
        domain::SettingsMessage::CustomProviderModelModalInterleavedChanged(v0) => SettingsMessage::CustomProviderModelModalInterleavedChanged(v0),
        domain::SettingsMessage::CustomProviderModelModalNameChanged(v0) => SettingsMessage::CustomProviderModelModalNameChanged(v0),
        domain::SettingsMessage::CustomProviderModelModalOutputLimitChanged(v0) => SettingsMessage::CustomProviderModelModalOutputLimitChanged(v0),
        domain::SettingsMessage::CustomProviderModelModalOutputModalityToggled(v0,v1) => SettingsMessage::CustomProviderModelModalOutputModalityToggled(v0,v1),
        domain::SettingsMessage::CustomProviderModelModalReasoningChanged(v0) => SettingsMessage::CustomProviderModelModalReasoningChanged(v0),
        domain::SettingsMessage::CustomProviderModelModalSave => SettingsMessage::CustomProviderModelModalSave,
        domain::SettingsMessage::CustomProviderModelModalTemperatureChanged(v0) => SettingsMessage::CustomProviderModelModalTemperatureChanged(v0),
        domain::SettingsMessage::CustomProviderModelModalToolCallChanged(v0) => SettingsMessage::CustomProviderModelModalToolCallChanged(v0),
        domain::SettingsMessage::CustomProviderModelTabSelected(v0) => SettingsMessage::CustomProviderModelTabSelected(v0),
        domain::SettingsMessage::ModelDetailClose => SettingsMessage::ModelDetailClose,
        domain::SettingsMessage::ModelDetailOpen(v0,v1) => SettingsMessage::ModelDetailOpen(v0,v1),
        domain::SettingsMessage::ModelDetailToggleRaw => SettingsMessage::ModelDetailToggleRaw,
        domain::SettingsMessage::ModelQueryChanged(v0) => SettingsMessage::ModelQueryChanged(v0),
        domain::SettingsMessage::ModelRoutes(message) => SettingsMessage::ModelRoutes(match message {
            domain::ModelRoutesMessage::AddRoute => ModelRoutesMessage::AddRoute,
            domain::ModelRoutesMessage::RemoveRoute(v0) => ModelRoutesMessage::RemoveRoute(v0),
            domain::ModelRoutesMessage::PatternChanged(v0,v1) => ModelRoutesMessage::PatternChanged(v0,v1),
            domain::ModelRoutesMessage::ProviderChanged(v0,v1) => ModelRoutesMessage::ProviderChanged(v0,v1),
            domain::ModelRoutesMessage::ModelChanged(v0,v1) => ModelRoutesMessage::ModelChanged(v0,v1),
            domain::ModelRoutesMessage::PriorityChanged(v0,v1) => ModelRoutesMessage::PriorityChanged(v0,v1),
}),
        domain::SettingsMessage::ModelToggle(v0,v1,v2) => SettingsMessage::ModelToggle(v0,v1,v2),
        domain::SettingsMessage::ModelsRefresh => SettingsMessage::ModelsRefresh,
        domain::SettingsMessage::ModelsSetAll(v0) => SettingsMessage::ModelsSetAll(v0),
        domain::SettingsMessage::ProviderModelsExpansionToggled(v0) => SettingsMessage::ProviderModelsExpansionToggled(v0),
        domain::SettingsMessage::ProviderModelsReset(v0) => SettingsMessage::ProviderModelsReset(v0),
        domain::SettingsMessage::ProviderModelsSetAll(v0,v1) => SettingsMessage::ProviderModelsSetAll(v0,v1),
}),
}
}

pub(crate) fn view_style(app: &App) -> desktop_settings_providers::view_style::ViewStyle<'_> {
    desktop_settings_providers::view_style::ViewStyle {
        locale: app.general_settings.desktop_locale,
        theme: &app.appearance.app_theme,
        help_text,
    }
}
pub(crate) fn help_text<'a>(
    title: &'a str,
    body: std::borrow::Cow<'a, str>,
    color: Option<iced::Color>,
    i18n: crate::app::i18n::I18n,
) -> iced::Element<'a, desktop_settings_providers::view_message::Message> {
    crate::app::components::chat_panel::message_view::selectable_titled_text(
        title, body, color, i18n,
    )
    .map(|message| match message {
        Message::CopyCode(text) => {
            desktop_settings_providers::view_message::Message::CopyCode(text)
        }
        _ => desktop_settings_providers::view_message::Message::None,
    })
}

#[cfg(test)]
#[path = "system_settings_models_tests.rs"]
mod system_settings_models_tests;

#[cfg(test)]
#[path = "system_settings_models_ui_refresh_10_tests.rs"]
mod ui_refresh_10_tests;

fn map_catalog_event(event: desktop_settings_providers::catalog_message::Event) -> Message {
 use desktop_settings_providers::catalog_message::{Event as D, FillEvent as F, DiscoveryEvent as V};
 use crate::app::message::settings::providers::{catalog_api::{self, Event as H}, metadata_fill, discovery};
 catalog_api::message(match event {
 D::Open(value) => H::Open(value),
 D::Close => H::Close,
 D::Query(value) => H::Query(value),
 D::Search => H::Search,
 D::ChooseEndpoint { generation, preset } => H::ChooseEndpoint { generation, preset },
 D::Toggle { generation, preset, model, checked } => H::Toggle { generation, preset, model, checked },
 D::Endpoint(value) => H::Endpoint(value),
 D::ManualId(value) => H::ManualId(value),
 D::Alias(value) => H::Alias(value),
 D::MakeDefault(value) => H::MakeDefault(value),
 D::EditModel { generation, model_id } => H::EditModel { generation, model_id },
 D::CancelEdit => H::CancelEdit,
 D::LoadCatalog => H::LoadCatalog,
 D::Save => H::Save,
 D::Fill(event) => H::Fill(match event { F::Preview => metadata_fill::Event::Preview, F::Commit => metadata_fill::Event::Commit, F::Silent(value) => metadata_fill::Event::Silent(value) }),
 D::Discovery(event) => H::Discovery(match event { V::Start(method) => discovery::Event::Start(method), V::Cancel => discovery::Event::Cancel, V::Toggle { run, index, checked } => discovery::Event::Toggle { run, index, checked } }),
 })
}
