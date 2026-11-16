//! Workflow view components with local workflow messages.
use desktop_workflow_model::{
    WorkflowNodeIconDescriptor, WorkflowNodeTypeDescriptor,
    supported_node_types, workflow_node_accent_color,
};
use desktop_workflow_model::{WorkflowEdge, WorkflowNode, pretty_block_type, workflow_node_icon};
use desktop_workflow_state::state;
use desktop_workflow_state::state::{
    WorkflowCanvasContextMenuTarget, WorkflowDataNodeEdit, WorkflowHttpRequestEdit,
    WorkflowNodeRetryDraft, WorkflowState, WorkflowVariablePanelKind,
    WorkflowNodeEditorTab, WorkflowNodeVisualDraft, WorkflowNodeEditorMode,
};
use crate::message::WorkflowMessage;
type Message = WorkflowMessage;
use desktop_assets::assets::{self, Icon};
use desktop_ui_controls::overlays::PointBelowOverlay;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, danger_action_btn_style,
    primary_action_btn_style, round_icon_btn_style, rounded_action_btn_style,
    settings_close_button, settings_modal_card_style,
    settings_modal_header, settings_modal_layout, settings_modal_overlay,
    settings_muted_text_style, settings_panel_style, settings_select, settings_text_editor_style, settings_text_input_style, settings_value_badge,
};
use desktop_ui_controls::system_settings_common::settings_segment_button_style;
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
pub mod if_else;
use if_else::*;
pub mod next_step;
use next_step::*;
pub mod node_editor_fields;
pub mod node_visual;
pub mod node_visual_integrations;
pub mod run_preview;
#[cfg(test)]
mod node_editor_fields_tests;
pub mod menu;
use menu::*;
pub mod node_editor;
use node_editor::*;
pub mod start_inputs;
use start_inputs::*;
pub mod start_panel;
pub mod start_meta;
use start_meta::{start_builtin_variables, start_variable_badge, start_variable_value_type_label, start_variable_summary_error, start_variable_default_error, start_variable_advanced_hint, build_start_variable_type_selector};
pub mod styles;
use styles::*;
pub mod toolbar;
use toolbar::*;
pub mod video_edit;
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

use run_preview::{build_saved_app_run_preview_layer, build_saved_app_run_detail_layer};

pub fn view<'a, M: Clone + Send + 'static>(
    i18n: desktop_i18n::I18n,
    state: &'a WorkflowState,
    app_editor: Option<Element<'a, M>>,
    node_editor: Option<Element<'a, M>>,
    map_message: fn(WorkflowMessage) -> M,
    ignored_message: M,
) -> Element<'a, M> {
    if state.active_app_id.is_none() {
        let mut layers: Vec<Element<'_, M>> = vec![
            container(build_saved_apps_view(i18n, state).map(map_message))
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        ];

        if let Some(error) = state.error_text(i18n) {
            layers.push(
                container(error_banner(i18n, error).map(map_message))
                    .padding(iced::Padding {
                        top: FLOATING_MARGIN,
                        right: FLOATING_MARGIN,
                        bottom: 0.0,
                        left: FLOATING_MARGIN,
                    })
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Top)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
            );
        }

        if let Some(app_editor) = app_editor {
            layers.push(app_editor);
        }
        if let Some(confirm_dialog) = build_saved_app_delete_confirm_dialog(i18n, state).map(|element| element.map(map_message)) {
            layers.push(confirm_dialog);
        }
        if let Some(run_prompt) = build_saved_app_run_prompt_dialog(i18n, state).map(|element| element.map(map_message)) {
            layers.push(run_prompt);
        }
        if let Some(preview) = &state.saved_app_run_preview {
            layers.push(build_saved_app_run_preview_layer(i18n, preview, state.run_error_copied).map(map_message));
        }
        if let Some(detail) = &state.saved_app_run_detail {
            layers.push(build_saved_app_run_detail_layer(i18n, detail).map(map_message));
        }

        if state.video_edit.open {
            layers.push(video_edit::modal(state).map(map_message));
        }
        return stack(layers).width(Length::Fill).height(Length::Fill).into();
    }

    let canvas_base: Element<'_, WorkflowMessage> = if !state.has_apps() || state.document.nodes.is_empty()
    {
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(canvas_panel_style)
            .into()
    } else {
        let canvas: Element<'_, desktop_workflow_canvas::CanvasMessage> =
            iced::widget::canvas(desktop_workflow_canvas::WorkflowCanvas {
                i18n,
                document: &state.document,
                pan: state.pan,
                zoom: state.zoom,
                selected_node_id: state.selected_node_id.as_deref(),
                selected_edge_id: state.selected_edge_id.as_deref(),
                connection_draft: state.connection_draft.as_ref(),
            })
            .width(Length::Fill)
            .height(Length::Fill).into();
        container(canvas.map(|message| match message {
        desktop_workflow_canvas::CanvasMessage::StartConnection(endpoint, point) => {
            WorkflowMessage::StartConnection(endpoint, point)
        }
        desktop_workflow_canvas::CanvasMessage::NodeDragStart(id) => WorkflowMessage::NodeDragStart(id),
        desktop_workflow_canvas::CanvasMessage::SelectEdge(id) => WorkflowMessage::SelectEdge(id),
        desktop_workflow_canvas::CanvasMessage::ClearSelection => WorkflowMessage::ClearSelection,
        desktop_workflow_canvas::CanvasMessage::OpenCanvasContextMenu(target, anchor, world) => {
            WorkflowMessage::OpenCanvasContextMenu(target, anchor, world)
        }
        desktop_workflow_canvas::CanvasMessage::FinishConnection(endpoint) => WorkflowMessage::FinishConnection(endpoint),
        desktop_workflow_canvas::CanvasMessage::CancelConnection => WorkflowMessage::CancelConnection,
        desktop_workflow_canvas::CanvasMessage::FinishNodeDrag => WorkflowMessage::FinishNodeDrag,
        desktop_workflow_canvas::CanvasMessage::PanBy(delta) => WorkflowMessage::PanBy(delta),
        desktop_workflow_canvas::CanvasMessage::NodeDragged(id, delta) => WorkflowMessage::NodeDragged(id, delta),
        desktop_workflow_canvas::CanvasMessage::UpdateConnectionCursor(point) => WorkflowMessage::UpdateConnectionCursor(point),
    }))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(canvas_panel_style)
        .into()
    };

    let canvas_area = build_canvas_area(i18n, state, canvas_base).map(map_message);

    let mut layers: Vec<Element<'_, M>> = vec![
        container(canvas_area).width(Length::Fill).height(Length::Fill).style(root_style).into(),
    ];

    if let Some(error) = state.error_text(i18n) {
        layers.push(
            container(error_banner(i18n, error).map(map_message))
                .padding(iced::Padding {
                    top: FLOATING_MARGIN,
                    right: FLOATING_MARGIN,
                    bottom: 0.0,
                    left: FLOATING_MARGIN,
                })
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Top)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        );
    }

    if let Some(app_editor) = app_editor {
        layers.push(app_editor);
    }
    if let Some(node_editor) = node_editor {
        layers.push(node_editor);
    }
    if state.variable_panel.is_some() {
        layers.push(build_variable_panel_modal(i18n, state).map(map_message));
    }
    if state.variable_editor.is_some() {
        layers.push(build_variable_editor_modal(i18n, state).map(map_message));
    }
    if state.node_editor.as_ref().and_then(|editor| editor.start_variable_editor.as_ref()).is_some()
    {
        layers.push(start_panel::build_start_variable_editor_modal(i18n, state, map_message, ignored_message));
    }

    if state.video_edit.open {
        layers.push(video_edit::modal(state).map(map_message));
    }
    stack(layers).width(Length::Fill).height(Length::Fill).into()
}

#[cfg(test)]
mod entry_tests;
