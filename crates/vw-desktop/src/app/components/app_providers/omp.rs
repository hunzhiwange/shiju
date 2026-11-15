use super::list::Help;
use crate::app::{Message,state::app_providers::{edit::EditForm,omp::Draft}};
use iced::Element;
pub(super) fn fields<'a>(form:&'a EditForm,draft:&'a Draft,help:Help<'_>)->Element<'a,Message>{
    desktop_provider_apps::components::omp::fields(form,draft,help.provider_help()).map(super::edit::map)
}
