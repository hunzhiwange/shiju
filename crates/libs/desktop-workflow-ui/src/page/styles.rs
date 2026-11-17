//! Desktop workflow message mapping.

#[cfg(test)]
pub(super) fn canvas_panel_style(theme: &Theme) -> iced::widget::container::Style {
    crate::view::styles::canvas_panel_style(theme)
}

#[cfg(test)]
pub(super) fn inspector_style(theme: &Theme) -> iced::widget::container::Style {
    crate::view::styles::inspector_style(theme)
}

#[cfg(test)]
pub(super) fn modal_backdrop_style(theme: &Theme) -> iced::widget::container::Style {
    crate::view::styles::modal_backdrop_style(theme)
}

#[cfg(test)]
use super::*;
