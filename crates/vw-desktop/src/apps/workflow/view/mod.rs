//! # Workflow 视图模块
//!
//! 该模块构建 workflow 整体界面，包含头部工具区、画布区、各类弹层编辑器与交互控件。

use super::canvas::WorkflowCanvas;
use super::message::WorkflowMessage;
use super::model::{
    WorkflowEdge, WorkflowNode, WorkflowNodeIconDescriptor, WorkflowNodeTypeDescriptor,
    pretty_block_type, supported_node_types, workflow_node_accent_color, workflow_node_icon,
    workflow_system_variables,
};
pub(super) use super::state;
use super::state::{
    WorkflowAppEditorMode, WorkflowCanvasContextMenuTarget, WorkflowCodeOutputDraft,
    WorkflowCodeVariableDraft, WorkflowDataNodeEdit, WorkflowHttpRequestEdit,
    WorkflowNodeEditorMode, WorkflowNodeEditorTab, WorkflowNodeRetryDraft, WorkflowNodeVisualDraft,
    WorkflowState, WorkflowVariablePanelKind,
};
use crate::app::Message;
use crate::app::assets::{self, Icon};
use crate::app::components::overlays::PointBelowOverlay;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, danger_action_btn_style,
    primary_action_btn_style, round_icon_btn_style, rounded_action_btn_style,
    settings_close_button, settings_modal_backdrop_style, settings_modal_card_style,
    settings_modal_header, settings_modal_layout, settings_modal_overlay,
    settings_muted_text_style, settings_panel_style, settings_segment_button_style,
    settings_select, settings_text_editor_style, settings_text_input_style, settings_value_badge,
};
use iced::widget::{
    Space, button, checkbox, container, image, mouse_area, scrollable,
    scrollable::{Direction, Scrollbar},
    slider, stack, svg, text, text_editor, text_input,
    tooltip::{Position as TooltipPosition, Tooltip},
};
use iced::{Alignment, Background, Border, Color, Element, Length, Point, Shadow, Theme, Vector};

mod app_editor;
mod app_list;
mod canvas_shell;
mod data_nodes;
mod http_request;
mod if_else;
mod menu;
mod next_step;
mod node_editor;
#[cfg_attr(not(test), allow(dead_code))]
mod node_editor_fields;
mod node_visual;
mod node_visual_integrations;
mod run_preview;
mod start_inputs;
mod start_meta;
mod start_panel;
mod styles;
mod toolbar;
mod variables;
mod video_edit;
#[cfg(test)]
mod video_edit_tests;

use app_editor::*;
use app_list::*;
use canvas_shell::*;
use data_nodes::*;
use http_request::*;
use if_else::*;
use menu::*;
use next_step::*;
use node_editor::*;
use node_visual::*;
use node_visual_integrations::*;
use run_preview::*;
use start_inputs::*;
use start_meta::*;
use start_panel::*;
use styles::*;
use toolbar::*;
use variables::*;

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

pub(super) use desktop_workflow_ui::view::editor_style;

pub(super) use desktop_workflow_ui::view::is_dark_theme;

pub(super) use desktop_workflow_ui::view::workflow_text_input;

use desktop_workflow_ui::view::next_step::next_step_jump_card_style;

use desktop_workflow_ui::view::{WorkflowAppSwitchOption, WorkflowAppSwitchSource};

pub(super) use desktop_workflow_ui::view::workflow_app_display_name;

pub(super) use desktop_workflow_ui::view::workflow_app_switch_options;

fn build_app_switcher(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
) -> Element<'static, Message> {
    desktop_workflow_ui::view::build_app_switcher(i18n, state).map(Message::WorkflowTool)
}

pub fn view(i18n: crate::app::i18n::I18n, state: &WorkflowState) -> Element<'_, Message> {
    if state.active_app_id.is_none() {
        let mut layers: Vec<Element<'_, Message>> = vec![
            container(build_saved_apps_view(i18n, state))
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        ];

        if let Some(error) = state.error_text(i18n) {
            layers.push(
                container(error_banner(i18n, error))
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

        if state.app_editor.is_some() {
            layers.push(build_app_editor_modal(i18n, state));
        }
        if let Some(confirm_dialog) = build_saved_app_delete_confirm_dialog(i18n, state) {
            layers.push(confirm_dialog);
        }
        if let Some(run_prompt) = build_saved_app_run_prompt_dialog(i18n, state) {
            layers.push(run_prompt);
        }
        if let Some(preview) = &state.saved_app_run_preview {
            layers.push(build_saved_app_run_preview_layer(i18n, preview, state.run_error_copied));
        }
        if let Some(detail) = &state.saved_app_run_detail {
            layers.push(build_saved_app_run_detail_layer(i18n, detail));
        }

        if state.video_edit.open {
            layers.push(video_edit::modal(state));
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
            iced::widget::canvas(WorkflowCanvas {
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

    let canvas_area = build_canvas_area(i18n, state, canvas_base);

    let mut layers: Vec<Element<'_, Message>> = vec![
        container(canvas_area).width(Length::Fill).height(Length::Fill).style(root_style).into(),
    ];

    if let Some(error) = state.error_text(i18n) {
        layers.push(
            container(error_banner(i18n, error))
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

    if state.app_editor.is_some() {
        layers.push(build_app_editor_modal(i18n, state));
    }
    if state.node_editor.is_some() {
        layers.push(build_node_editor_modal(i18n, state));
    }
    if state.variable_panel.is_some() {
        layers.push(build_variable_panel_modal(i18n, state));
    }
    if state.variable_editor.is_some() {
        layers.push(build_variable_editor_modal(i18n, state));
    }
    if state.node_editor.as_ref().and_then(|editor| editor.start_variable_editor.as_ref()).is_some()
    {
        layers.push(build_start_variable_editor_modal(i18n, state));
    }

    if state.video_edit.open {
        layers.push(video_edit::modal(state));
    }
    stack(layers).width(Length::Fill).height(Length::Fill).into()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "mod_ui_refresh_36_tests.rs"]
mod ui_refresh_36_tests;
