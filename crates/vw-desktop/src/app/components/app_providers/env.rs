use crate::app::{Message,state::app_providers::env::Form};
use iced::Element;
use super::super::system_settings_common::*;
use super::list::Help;
pub(crate) fn overlay<'a>(form:&'a Form,help:Help<'_>,base:Element<'a,Message>)->Element<'a,Message>{
let card=desktop_provider_apps::components::env::card(form,help.provider_help()).map(super::edit::map);
settings_modal_overlay(Some(base),if form.writing(){Message::None}else{crate::app::message::app_providers::env::message(crate::app::message::app_providers::env::Event::Close)},card)
}
