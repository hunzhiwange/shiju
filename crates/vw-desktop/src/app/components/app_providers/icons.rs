use crate::app::{Message,state::app_providers::icons::Form};
use iced::Element;
use super::super::system_settings_common::*;
pub(crate) fn overlay<'a>(form:&'a Form,base:Element<'a,Message>)->Element<'a,Message>{
let card=desktop_provider_apps::components::icons::card(form).map(super::edit::map);
settings_modal_overlay(Some(base),crate::app::message::app_providers::icons::message(crate::app::message::app_providers::icons::Event::Close),card)
}
