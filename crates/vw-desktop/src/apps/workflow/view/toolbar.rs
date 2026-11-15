//! Desktop workflow message mapping.

use super::*;

pub(super) fn build_center_toolbar(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
) -> Element<'static, Message> {
    desktop_workflow_ui::view::toolbar::build_center_toolbar(i18n, state).map(Message::WorkflowTool)
}

pub(super) fn build_zoom_control(state: &WorkflowState) -> Element<'static, Message> {
    desktop_workflow_ui::view::toolbar::build_zoom_control(state).map(Message::WorkflowTool)
}

pub(super) fn build_zoom_menu_overlay(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
    zoom_presets: &[u32],
) -> Element<'static, Message> {
    desktop_workflow_ui::view::toolbar::build_zoom_menu_overlay(i18n, state, zoom_presets).map(Message::WorkflowTool)
}

pub(super) fn build_app_switcher_dock(
    i18n: crate::app::i18n::I18n,
    state: &WorkflowState,
) -> Element<'static, Message> {
    desktop_workflow_ui::view::toolbar::build_app_switcher_dock(i18n, state).map(Message::WorkflowTool)
}

pub(super) use desktop_workflow_ui::view::toolbar::mix_color;

pub(super) use desktop_workflow_ui::view::toolbar::toolbar_icon_style;

pub(super) use desktop_workflow_ui::view::toolbar::toolbar_button_style;

pub(super) use desktop_workflow_ui::view::toolbar::zoom_button_style;

pub(super) use desktop_workflow_ui::view::toolbar::zoom_menu_button_style;
