//! Theme editor host adapter.
use crate::app::{App, Message};
use iced::Element;
use desktop_settings_appearance::system_settings_themes::editor as domain;
pub(crate) use domain::{slugify_theme_id, allocate_unique_theme_id, new_theme_draft, draft_from_definition, draft_from_builtin, try_definition_from_draft};
pub(crate) fn existing_custom_ids(app: &App, exclude: Option<&str>) -> Vec<String> { domain::existing_custom_ids(&super::inputs(app), exclude) }
pub(crate) fn view(app: &App) -> Element<'_, Message> { domain::view(&super::inputs(app)) }
#[cfg(test)] mod editor_tests;
