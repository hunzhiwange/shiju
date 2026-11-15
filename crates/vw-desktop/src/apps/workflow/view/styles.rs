//! Desktop workflow message mapping.

use super::*;

pub(super) fn canvas_panel_style(theme: &Theme) -> iced::widget::container::Style {
    desktop_workflow_ui::view::styles::canvas_panel_style(theme)
}

pub(super) fn inspector_style(theme: &Theme) -> iced::widget::container::Style {
    desktop_workflow_ui::view::styles::inspector_style(theme)
}

pub(super) fn modal_backdrop_style(theme: &Theme) -> iced::widget::container::Style {
    desktop_workflow_ui::view::styles::modal_backdrop_style(theme)
}

pub(super) use desktop_workflow_ui::view::styles::floating_panel_style;

pub(super) use desktop_workflow_ui::view::styles::root_style;

pub(super) use desktop_workflow_ui::view::styles::value_card_style;

pub(super) use desktop_workflow_ui::view::styles::modal_card_style;

pub(super) use desktop_workflow_ui::view::node_editor::node_editor_drawer_style;

pub(super) use desktop_workflow_ui::view::node_editor::validation_summary_style;
