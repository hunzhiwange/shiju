//! 组合模型设置子页，并将领域消息与弹窗映射到桌面壳。

use crate::app::components::system_settings_common::{settings_error_banner, settings_tab_button, settings_tab_row, settings_modal_overlay};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::i18n::settings_models::text as model_text;
use crate::app::message::settings::{ModelConfigTabMessage, SettingsMessage};
use crate::app::state::ModelConfigTab;
use crate::app::{App, Message};
use iced::widget::column;
use iced::{Element, Length};

pub(super) const MODEL_TABS: &[SettingsTabDescriptor<ModelConfigTab>] = &[
    SettingsTabDescriptor::new(
        ModelConfigTab::Models,
        "模型",
        &[
            "models",
            "搜尋模型",
            "搜索模型",
            "模型详情",
            "提供商",
            "模型 id",
            "上下文窗口",
            "最大输出",
            "温度",
        ],
    ),
    SettingsTabDescriptor::new(
        ModelConfigTab::ModelRoutes,
        "模型路由",
        &["model routes", "模型路由", "routing", "匹配模式", "pattern", "priority", "优先级"],
    ),
    SettingsTabDescriptor::new(
        ModelConfigTab::QueryClassification,
        "模型分类",
        &["model classification", "模型分類", "classification", "rules", "查询分类", "规则"],
    ),
    SettingsTabDescriptor::new(
        ModelConfigTab::Multimodal,
        "多模态",
        &[
            "multimodal",
            "image",
            "vision",
            "图片模型",
            "OCR",
            "文字提取",
            "远程抓取",
            "remote fetch",
        ],
    ),
];

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
    let tabs = settings_tab_row(MODEL_TABS.iter().map(|descriptor| {
        settings_tab_button(
            model_text(app.desktop_locale, descriptor.label),
            descriptor.tab == active_tab,
            Some(Message::Settings(SettingsMessage::ModelConfigTab(
                ModelConfigTabMessage::Selected(descriptor.tab),
            ))),
        )
        .into()
    }))
    .wrap();
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

    let mut sections = column![tabs].spacing(16).width(Length::Fill);
    if let Some(error) = active_tab_save_error(app) {
        let message = match app.model_settings.active_tab {
            ModelConfigTab::Models => {
                app.model_settings.save_error.as_ref().map(|e| e.display(app.desktop_locale))
            }
            ModelConfigTab::ModelRoutes => {
                app.model_routes_settings.save_error.as_ref().map(|e| e.display(app.desktop_locale))
            }
            ModelConfigTab::QueryClassification => Some(
                app.query_classification_settings
                    .save_error_translation
                    .as_ref()
                    .map(|message| {
                        message
                            .render(crate::app::i18n::I18n::new(app.desktop_locale))
                            .expect("valid classification error template")
                    })
                    .unwrap_or_else(|| error.to_string()),
            ),
            _ => Some(error.to_string()),
        };
        if let Some(message) = message {
            sections = sections.push(settings_error_banner(message));
        }
    }

    sections.push(content).into()
}

/// 委派模型目录视图。
pub fn main_view(app: &App) -> Element<'_, Message> {
    desktop_settings_providers::models_view::main_view(&app.model_settings, view_style(app))
        .map(map_view_message)
}
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let Some(mm) = &app.model_settings.detail_modal else { return dialog; };
    let (close, card) = desktop_settings_providers::models_view::detail_card(mm, view_style(app));
    settings_modal_overlay(Some(dialog), map_view_message(close), card.map(map_view_message))
}

use desktop_settings_providers::view_message as domain;
pub(crate) fn map_view_message(message: domain::Message) -> crate::app::Message {
 use crate::app::{Message, message::settings::{SettingsMessage, ModelRoutesMessage}};
 match message {
 domain::Message::CopyCode(text) => Message::CopyCode(text),
 domain::Message::None => Message::None,
 domain::Message::Settings(message) => Message::Settings(match message {
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
        locale: app.desktop_locale, theme: &app.app_theme, help_text: help_text,
    }
}
fn help_text<'a>(title: &'a str, body: std::borrow::Cow<'a, str>, color: Option<iced::Color>, i18n: crate::app::i18n::I18n) -> iced::Element<'a, desktop_settings_providers::view_message::Message> {
    crate::app::components::chat_panel::message_view::selectable_titled_text(title, body, color, i18n)
        .map(|message| match message {
            Message::CopyCode(text) => desktop_settings_providers::view_message::Message::CopyCode(text),
            _ => desktop_settings_providers::view_message::Message::None,
        })
}

#[cfg(test)]
#[path = "system_settings_models_tests.rs"]
mod system_settings_models_tests;

#[cfg(test)]
#[path = "system_settings_models_ui_refresh_10_tests.rs"]
mod ui_refresh_10_tests;
