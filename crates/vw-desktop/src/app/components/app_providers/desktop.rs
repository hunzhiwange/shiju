use crate::app::{Message,state::app_providers::desktop::Form};
use iced::Element;
use super::super::system_settings_common::*;
use super::list::Help;
pub(crate) fn overlay<'a>(form:&'a Form,help:Help<'_>,base:Element<'a,Message>)->Element<'a,Message>{
if form.aggregation.is_some(){return crate::app::components::aggregation_desktop::overlay(form,help,base);}
let card=desktop_provider_apps::components::desktop::card(form,help.provider_help()).map(super::edit::map);
settings_modal_overlay(Some(base),crate::app::message::app_providers::desktop::message(crate::app::message::app_providers::desktop::Event::Cancel),card)
}
#[cfg(test)]
#[path = "desktop_ui_refresh_12_tests.rs"]
pub(super) mod ui_refresh_12_tests;

pub(crate) fn mapping_fields<'a>(form:&'a Form,help:Help<'_>)->Vec<Element<'a,Message>>{
    desktop_provider_apps::components::desktop::mapping_fields(form,help.provider_help()).into_iter().map(|element|element.map(super::edit::map)).collect()
}
