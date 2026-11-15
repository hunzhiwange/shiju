//! # Workflow 视图模块
//!
//! 该模块构建 workflow 整体界面，包含头部工具区、画布区、各类弹层编辑器与交互控件。

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
    desktop_workflow_ui::view::view(
        i18n,
        state,
        state.app_editor.as_ref().map(|_| build_app_editor_modal(i18n, state)),
        state.node_editor.as_ref().filter(|_| state.active_app_id.is_some()).map(|_| build_node_editor_modal(i18n, state)),
        Message::WorkflowTool,
        Message::None,
    )
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "mod_ui_refresh_36_tests.rs"]
mod ui_refresh_36_tests;
