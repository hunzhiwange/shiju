//! Workflow view components with local workflow messages.
use desktop_workflow_model::{
    WorkflowNodeIconDescriptor, WorkflowNodeTypeDescriptor,
    supported_node_types, workflow_node_accent_color,
};
#[cfg(test)]
use desktop_workflow_model::{WorkflowEdge, WorkflowNode};
use desktop_workflow_state::state;
use desktop_workflow_state::state::{
    WorkflowCanvasContextMenuTarget, WorkflowDataNodeEdit, WorkflowHttpRequestEdit,
    WorkflowNodeRetryDraft, WorkflowState, WorkflowVariablePanelKind,
};
use crate::message::WorkflowMessage;
type Message = WorkflowMessage;
use desktop_assets::{self as assets, Icon};
use desktop_ui_controls::overlays::PointBelowOverlay;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, danger_action_btn_style,
    primary_action_btn_style, round_icon_btn_style, rounded_action_btn_style,
    settings_close_button, settings_modal_card_style,
    settings_modal_header, settings_modal_layout, settings_modal_overlay,
    settings_muted_text_style, settings_panel_style, settings_select, settings_text_editor_style, settings_text_input_style, settings_value_badge,
};
use iced::widget::{
    Space, button, checkbox, container, image, scrollable,
    scrollable::{Direction, Scrollbar},
    stack, svg, text, text_editor, text_input,
    tooltip::{Position as TooltipPosition, Tooltip},
};
use iced::{Alignment, Background, Border, Color, Element, Length, Point, Shadow, Theme, Vector};

const FLOATING_MARGIN: f32 = 14.0;
const ACTION_BAR_BUTTON_SIZE: f32 = 30.0;
const ACTION_BAR_PADDING: f32 = 6.0;
const ACTION_BAR_HEIGHT: f32 = ACTION_BAR_PADDING * 2.0 + ACTION_BAR_BUTTON_SIZE;
const ACTION_MENU_GAP: f32 = 8.0;
const TOOLBAR_WIDTH: f32 = 248.0;
const TOOLBAR_HEIGHT: f32 = 40.0;
const QUICK_INSERT_GAP: f32 = 8.0;
const ZOOM_CONTROL_WIDTH: f32 = 156.0;
const ZOOM_CONTROL_HEIGHT: f32 = 32.0;
const ZOOM_MENU_GAP: f32 = 8.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowAppSwitchOption {
    pub id: String,
    pub label: String,
    pub source: WorkflowAppSwitchSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowAppSwitchSource {
    SavedUuid(String),
    OpenApp(String),
}

impl std::fmt::Display for WorkflowAppSwitchOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

pub mod app_list;
use app_list::*;
pub mod canvas_shell;
use canvas_shell::*;
pub mod data_nodes;
use data_nodes::*;
pub mod header;
use header::*;
pub mod http_request;
use http_request::*;
pub mod menu;
use menu::*;
pub mod node_editor;
use node_editor::*;
pub mod start_meta;
use start_meta::*;
pub mod styles;
use styles::*;
pub mod toolbar;
use toolbar::*;
pub mod variables;
use variables::*;

pub fn editor_style(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    settings_text_editor_style(theme, status)
}

pub fn is_dark_theme(theme: &Theme) -> bool {
    let background = theme.palette().background;
    background.r + background.g + background.b < 1.5
}

pub use crate::app_editor::workflow_text_input;

pub fn workflow_app_display_name(i18n: desktop_i18n::I18n, name: &str) -> String {
    if name.trim().is_empty() { i18n.__("未命名应用").to_string() } else { name.to_string() }
}

pub fn workflow_app_switch_options(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Vec<WorkflowAppSwitchOption> {
    let mut options = state
        .saved_apps
        .iter()
        .map(|app| {
            let is_active = state.local_uuid.as_deref() == Some(app.uuid.as_str());
            let dirty = is_active && state.active_is_dirty;
            let label = if dirty {
                format!("🤖 {} *", workflow_app_display_name(i18n, &app.name))
            } else {
                format!("🤖 {}", workflow_app_display_name(i18n, &app.name))
            };

            WorkflowAppSwitchOption {
                id: app.uuid.clone(),
                label,
                source: WorkflowAppSwitchSource::SavedUuid(app.uuid.clone()),
            }
        })
        .collect::<Vec<_>>();

    for app in state.apps.iter().filter(|app| {
        app.local_uuid
            .as_deref()
            .is_none_or(|uuid| !state.saved_apps.iter().any(|saved| saved.uuid == uuid))
    }) {
        let is_active = state.active_app_id.as_deref() == Some(app.id.as_str());
        let name = if is_active {
            workflow_app_display_name(i18n, &state.source_name)
        } else {
            workflow_app_display_name(i18n, &app.meta.name)
        };
        let dirty = if is_active { state.active_is_dirty } else { app.is_dirty };
        let label = if dirty {
            format!("{} {} *", app.meta.icon, name)
        } else {
            format!("{} {}", app.meta.icon, name)
        };

        options.push(WorkflowAppSwitchOption {
            id: app.id.clone(),
            label,
            source: WorkflowAppSwitchSource::OpenApp(app.id.clone()),
        });
    }

    options
}

pub fn build_app_switcher(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Element<'static, Message> {
    let options = workflow_app_switch_options(i18n, state);
    let selected = state
        .local_uuid
        .as_ref()
        .and_then(|uuid| {
            options.iter().find(|option| {
                matches!(&option.source, WorkflowAppSwitchSource::SavedUuid(saved_uuid) if saved_uuid == uuid)
            })
        })
        .or_else(|| {
            state.active_app_id.as_ref().and_then(|active_id| {
                options.iter().find(|option| {
                    matches!(&option.source, WorkflowAppSwitchSource::OpenApp(app_id) if app_id == active_id)
                })
            })
        })
        .cloned();

    if options.is_empty() {
        container(Space::new().width(Length::Shrink).height(Length::Fixed(0.0)))
            .width(Length::Fill)
            .into()
    } else {
        settings_select(options, selected, |option| match option.source {
            WorkflowAppSwitchSource::SavedUuid(uuid) => {
                WorkflowMessage::OpenSavedApp(uuid)
            }
            WorkflowAppSwitchSource::OpenApp(id) => {
                WorkflowMessage::SelectApp(id)
            }
        })
        .width(Length::Fill)
        .into()
    }
}
