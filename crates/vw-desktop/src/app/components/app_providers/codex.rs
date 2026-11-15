use super::list::Help;
use crate::app::{Message,state::app_providers::{edit::EditForm,codex::Draft}};
use iced::Element;
pub(super) fn fields<'a>(form:&'a EditForm,draft:&'a Draft,help:Help<'_>)->Element<'a,Message>{
    desktop_provider_apps::components::codex::fields(form,draft,help.provider_help()).map(super::edit::map)
}
