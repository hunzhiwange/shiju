//! # Workflow 视图模块
//!
//! 该模块构建 workflow 整体界面，包含头部工具区、画布区、各类弹层编辑器与交互控件。

use self::PageMessage as Message;
use crate::message::WorkflowMessage;
pub(super) use desktop_workflow_state::state;
use desktop_workflow_state::state::{
    WorkflowCodeOutputDraft,
    WorkflowCodeVariableDraft, WorkflowNodeEditorTab, WorkflowNodeRetryDraft, WorkflowNodeVisualDraft,
    WorkflowState,
};
use iced::widget::text_editor;
use iced::Element;

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
use node_editor::*;
use node_visual::*;
use node_visual_integrations::*;
use start_panel::*;

#[cfg(test)]
const FLOATING_MARGIN: f32 = 14.0;
#[cfg(test)]
const ACTION_BAR_BUTTON_SIZE: f32 = 30.0;
#[cfg(test)]
const ACTION_BAR_PADDING: f32 = 6.0;
#[cfg(test)]
const ACTION_BAR_HEIGHT: f32 = ACTION_BAR_PADDING * 2.0 + ACTION_BAR_BUTTON_SIZE;
#[cfg(test)]
const ACTION_MENU_GAP: f32 = 8.0;
#[cfg(test)]
const TOOLBAR_WIDTH: f32 = 248.0;
#[cfg(test)]
const TOOLBAR_HEIGHT: f32 = 40.0;
#[cfg(test)]
const QUICK_INSERT_GAP: f32 = 8.0;
#[cfg(test)]
const ZOOM_CONTROL_WIDTH: f32 = 156.0;
#[cfg(test)]
const ZOOM_CONTROL_HEIGHT: f32 = 32.0;
#[cfg(test)]
const ZOOM_MENU_GAP: f32 = 8.0;

#[cfg(test)]
fn build_app_switcher(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Element<'static, Message> {
    crate::view::build_app_switcher(i18n, state).map(Message::WorkflowTool)
}

pub fn view(i18n: desktop_i18n::I18n, state: &WorkflowState) -> Element<'_, Message> {
    crate::view::view(
        i18n,
        state,
        state.app_editor.as_ref().map(|_| build_app_editor_modal(i18n, state)),
        state
            .node_editor
            .as_ref()
            .filter(|_| state.active_app_id.is_some())
            .map(|_| build_node_editor_modal(i18n, state)),
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

#[derive(Debug, Clone)]
pub enum ViewMessage {
    OpenUrlExternal(String),
}
#[derive(Debug, Clone)]
pub enum PageMessage {
    WorkflowTool(WorkflowMessage),
    None,
    CopyCode(String),
    TextContext(desktop_ui_controls::text_editor_context_menu::TextContextEffect),
    View(ViewMessage),
}
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for PageMessage {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        match effect {
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::OpenUrl(url) => {
                Self::View(ViewMessage::OpenUrlExternal(url))
            }
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::Copy(text) => {
                Self::CopyCode(text)
            }
            effect => Self::TextContext(effect),
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}

#[cfg(test)]
use desktop_workflow_state::state::*;
#[cfg(test)]
use desktop_workflow_model::*;
#[cfg(test)]
use iced::{Alignment, Background, Border, Color, Length, Point, Shadow, Theme, Vector};
#[cfg(test)]
use iced::widget::{Space, button, checkbox, container, image, mouse_area, scrollable, slider, stack, svg, text, text_input, scrollable::{Direction, Scrollbar}, tooltip::{Position as TooltipPosition, Tooltip}};
#[cfg(test)]
use desktop_assets::assets::{self, Icon};
#[cfg(test)]
use desktop_ui_controls::overlays::PointBelowOverlay;
#[cfg(test)]
use desktop_ui_controls::system_settings_common::*;
#[cfg(test)]
use app_list::*;
#[cfg(test)]
use canvas_shell::*;
#[cfg(test)]
use data_nodes::*;
#[cfg(test)]
use http_request::*;
#[cfg(test)]
use if_else::*;
#[cfg(test)]
use menu::*;
#[cfg(test)]
use next_step::*;
#[cfg(test)]
use run_preview::*;
#[cfg(test)]
use start_inputs::*;
#[cfg(test)]
use start_meta::*;
#[cfg(test)]
use styles::*;
#[cfg(test)]
use toolbar::*;
#[cfg(test)]
use variables::*;
#[cfg(test)]
use crate::view::{editor_style, is_dark_theme, workflow_text_input, WorkflowAppSwitchOption, WorkflowAppSwitchSource, workflow_app_display_name, workflow_app_switch_options};
