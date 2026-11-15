//! 知识库工作台视图。

use desktop_assets::assets::Icon;
use desktop_chat_input::input_panel::drop_area::DropArea;
use desktop_ui_controls::overlays::PointBelowOverlay;
use desktop_ui_controls::system_settings_common::{
    danger_action_btn_style, icon_svg, primary_action_btn_style, round_icon_btn_style,
    rounded_action_btn_style, settings_checkbox_style, settings_close_button,
    settings_error_banner, settings_modal_card, settings_modal_overlay,
    settings_muted_text_style, settings_panel, settings_select, settings_success_banner,
    settings_text_editor_style, settings_text_input, settings_text_input_style,
    settings_value_badge,
};
use desktop_i18n::I18n;
use crate::message::knowledge_tool::KnowledgeToolMessage;
use crate::state::{
    KnowledgeCreateDatasetTab, KnowledgeDatasetEmbeddingSelection, KnowledgeDetailTab,
    KnowledgeDocumentInputTab, KnowledgeImportPollingJob,
};

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{
    Space, button, checkbox, column, container, mouse_area, responsive, row, scrollable, text,
    text_editor, text_input,
};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Point, Shadow, Size, Theme, Vector,
};
use std::fmt;
use vw_api_knowledge::KnowledgeDocumentsBatchAction;
use {vw_api_knowledge::KnowledgeChunkDto, vw_api_knowledge::KnowledgeChunkType, vw_api_knowledge::KnowledgeChunkingMode, vw_api_knowledge::KnowledgeDatasetDto, vw_api_knowledge::KnowledgeDocumentDto, vw_api_knowledge::KnowledgeDocumentStatus, vw_api_knowledge::KnowledgeEmbeddingPresetDto, vw_api_knowledge::KnowledgeEmbeddingStatusDto, vw_api_knowledge::KnowledgeEmbeddingTestResponse, vw_api_knowledge::KnowledgeIndexingMode, vw_api_knowledge::KnowledgeMatchType, vw_api_knowledge::KnowledgeRetrievalMode};


/// 知识工作台实际使用的只读输入。
#[derive(Clone, Copy)]
pub struct ViewInput<'a> {
    pub knowledge: &'a crate::state::KnowledgeUiState,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: &'a Theme,
    pub files_dragging: bool,
    pub last_copied_code_hash: Option<u64>,
}
#[derive(Debug, Clone)]
pub enum ViewMessage { OpenUrlExternal(String) }
#[derive(Debug, Clone)]
pub enum Message {
    Knowledge(KnowledgeToolMessage),
    OpenMemorySettings,
    CopyCode(String),
    AppendToInput(String),
    View(ViewMessage),
    None,
    Selection { session_id: String, message_idx: usize, text_idx: usize, text: Option<String> },
}
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for Message {
    fn none() -> Self { Self::None }
    fn copy(text: String) -> Self { Self::CopyCode(text) }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        use desktop_ui_controls::text_editor_context_menu::TextContextEffect;
        match effect {
            TextContextEffect::Copy(text) => Self::CopyCode(text),
            TextContextEffect::Append(text) => Self::AppendToInput(text),
            TextContextEffect::OpenUrl(url) => Self::View(ViewMessage::OpenUrlExternal(url)),
        }
    }
    fn selection(session_id: String, message_idx: usize, text_idx: usize, text: Option<String>) -> Self {
        Self::Selection { session_id, message_idx, text_idx, text }
    }
}
fn settings_hover_help_label<'a>(
    i18n: I18n, theme: &Theme, title: &'a str, label: iced::widget::Text<'a>,
    description: impl Into<std::borrow::Cow<'a, str>>,
) -> iced::widget::Row<'a, Message> {
    desktop_ui_controls::system_settings_common::settings_hover_help_label(
        label,
        desktop_chat_view::message_view::markdown_selection::selectable_titled_text(
            title, description, Some(theme.palette().text), i18n,
        ),
    )
}

const DOCUMENTS_PER_PAGE: usize = 10;
const DOCUMENT_PAGE_SIZE_OPTIONS: [usize; 6] = [10, 20, 30, 40, 50, 100];
const KNOWLEDGE_SCROLLBAR_WIDTH: f32 = 4.0;
const KNOWLEDGE_SCROLLBAR_GUTTER: f32 = 12.0;
const DOCUMENT_ACTIONS_WIDTH: f32 = 152.0;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DocumentStatusFilterOption {
    label: &'static str,
    status: KnowledgeDocumentStatus,
}

#[derive(Debug)]
struct FilteredDocumentsPage<'a> {
    documents: Vec<&'a KnowledgeDocumentDto>,
    total_matches: usize,
    current_page: usize,
    total_pages: usize,
    page_size: usize,
    start_index: usize,
}

fn document_status_filter_options(i18n: desktop_i18n::I18n) -> [DocumentStatusFilterOption; 5] {
    [
        DocumentStatusFilterOption {
            label: i18n.__("可用"),
            status: KnowledgeDocumentStatus::Completed,
        },
        DocumentStatusFilterOption {
            label: i18n.__("索引中"),
            status: KnowledgeDocumentStatus::Processing,
        },
        DocumentStatusFilterOption {
            label: i18n.__("入队"),
            status: KnowledgeDocumentStatus::Pending,
        },
        DocumentStatusFilterOption {
            label: i18n.__("停用"),
            status: KnowledgeDocumentStatus::Disabled,
        },
        DocumentStatusFilterOption {
            label: i18n.__("归档"),
            status: KnowledgeDocumentStatus::Archived,
        },
    ]
}

pub fn view(app: ViewInput<'_>) -> Element<'_, Message> {
    let workspace = responsive(move |size| build_workspace(app, size));

    let mut content = column![].spacing(12).width(Length::Fill).height(Length::Fill);
    if let Some(raw) = &app.knowledge.notification {
        let notification = app
            .knowledge
            .notification_feedback
            .as_ref()
            .map(|feedback| {
                if feedback.as_str() == raw {
                    feedback.render(desktop_i18n::I18n::new(app.desktop_locale))
                } else {
                    raw.clone()
                }
            })
            .unwrap_or_else(|| raw.clone());
        let banner = if app.knowledge.notification_is_error {
            settings_error_banner(notification)
        } else {
            settings_success_banner(notification)
        };
        content = content.push(banner);
    }
    content = content.push(container(workspace).width(Length::Fill).height(Length::Fill));

    let base: Element<'_, Message> = container(content.padding([30, 34]))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(palette.background.base.color.into()),
                ..Default::default()
            }
        })
        .into();

    if app.knowledge.create_dataset_modal_open {
        build_create_dataset_modal(app, Some(base))
    } else if app.knowledge.pending_delete_dataset_id.is_some()
        || app.knowledge.pending_delete_document_id.is_some()
    {
        build_delete_confirm_modal(app, Some(base))
    } else if app.knowledge.rename_document_id.is_some() {
        build_document_rename_modal(app, Some(base))
    } else {
        base
    }
}

fn build_workspace<'a>(app: ViewInput<'a>, size: Size) -> Element<'a, Message> {
    if app.knowledge.showing_dataset_list || app.knowledge.selected_dataset_id.is_none() {
        build_dataset_catalog_view(app)
    } else {
        build_detail(app, size)
    }
}

mod catalog;
use catalog::*;
mod create_dataset;
use create_dataset::*;
mod dataset_cards;
use dataset_cards::*;
mod document_import;
use document_import::*;
mod documents;
use documents::*;
mod document_actions;
use document_actions::*;
mod previews;
use previews::*;
mod presentation;
use presentation::*;

#[cfg(test)]
#[path = "knowledge_tests.rs"]
mod knowledge_tests;

mod labels;
use labels::*;

mod dataset_configuration;
use dataset_configuration::*;
