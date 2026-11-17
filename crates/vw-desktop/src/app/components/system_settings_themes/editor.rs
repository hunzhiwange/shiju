//! Theme editor host adapter.
#[cfg(test)]
pub(crate) fn existing_custom_ids(app: &App, exclude: Option<&str>) -> Vec<String> { domain::existing_custom_ids(&super::inputs(app), exclude) }
#[cfg(test)]
pub(crate) fn view(app: &App) -> Element<'_, Message> { domain::view(&super::inputs(app)) }
#[cfg(test)]
#[path = "editor_tests.rs"]
mod editor_tests;

#[cfg(test)]
use crate::app::App;
#[cfg(test)]
use crate::app::Message;
#[cfg(test)]
use desktop_settings_appearance::system_settings_themes::editor as domain;
#[cfg(test)]
use iced::Element;
#[cfg(test)]
pub(crate) use domain::allocate_unique_theme_id;
#[cfg(test)]
pub(crate) use domain::draft_from_builtin;
#[cfg(test)]
pub(crate) use domain::draft_from_definition;
#[cfg(test)]
pub(crate) use domain::new_theme_draft;
#[cfg(test)]
pub(crate) use domain::slugify_theme_id;
#[cfg(test)]
pub(crate) use domain::try_definition_from_draft;
