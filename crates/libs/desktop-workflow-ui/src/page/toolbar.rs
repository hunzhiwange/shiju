//! Desktop workflow message mapping.

use super::*;

pub(super) fn build_center_toolbar(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Element<'static, Message> {
    crate::view::toolbar::build_center_toolbar(i18n, state).map(Message::WorkflowTool)
}

pub(super) fn build_zoom_control(state: &WorkflowState) -> Element<'static, Message> {
    crate::view::toolbar::build_zoom_control(state).map(Message::WorkflowTool)
}

pub(super) fn build_zoom_menu_overlay(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
    zoom_presets: &[u32],
) -> Element<'static, Message> {
    crate::view::toolbar::build_zoom_menu_overlay(i18n, state, zoom_presets)
        .map(Message::WorkflowTool)
}

pub(super) fn build_app_switcher_dock(
    i18n: desktop_i18n::I18n,
    state: &WorkflowState,
) -> Element<'static, Message> {
    crate::view::toolbar::build_app_switcher_dock(i18n, state).map(Message::WorkflowTool)
}

pub(super) use crate::view::toolbar::mix_color;

pub(super) use crate::view::toolbar::toolbar_icon_style;

pub(super) use crate::view::toolbar::toolbar_button_style;

pub(super) use crate::view::toolbar::zoom_button_style;

pub(super) use crate::view::toolbar::zoom_menu_button_style;
