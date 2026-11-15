use crate::app::{Message,state::app_providers::endpoints::Form};
use iced::Element;
use super::super::system_settings_common::*;
pub(crate) fn overlay<'a>(form:&'a Form,app:vw_api_types::agent_apps::AppId,base:Element<'a,Message>)->Element<'a,Message>{
let card=desktop_provider_apps::components::endpoints::card(form,app).map(super::edit::map);
settings_modal_overlay(Some(base),crate::app::message::app_providers::endpoints::message(crate::app::message::app_providers::endpoints::Event::Cancel),card)
}
