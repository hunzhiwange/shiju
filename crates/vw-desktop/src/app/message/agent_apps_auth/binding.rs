use super::message;
use crate::app::{Message, state::app_providers::edit::EditForm};
use vw_api_app_identity::AppId;
pub(crate) fn entry(form: &EditForm, app: AppId, generation: u64) -> Option<Message> {
    desktop_apps_ui::agent_apps_auth::binding::entry(form, app, generation).map(message)
}
