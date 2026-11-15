use super::{
    edit::{self, EditMessage},
    list::{self, ListMessage},
};
use crate::app::{
    App, Message,
    message::{SettingsMessage, ViewMessage},
};
use iced::{Element, Task};

pub use desktop_provider_apps::message::leave::Choice;
fn busy(app: &App) -> bool {
    app.app_providers.form.as_ref().is_some_and(|f| f.saving)
        || app.app_providers.editor.as_ref().is_some_and(|f| f.saving)
}

/// Runs before dispatch so cancelling never mutates the current page or host.
pub(crate) fn intercept(app: &mut App, message: &Message) -> bool {
    use EditMessage as E;
    use ListMessage as L;
    use SettingsMessage as S;
    let edits = matches!(
        message,
        Message::Settings(S::AppProviderEdit(
            E::Id(_)
                | E::Name(_)
                | E::Category(_)
                | E::Notes(_)
                | E::Website(_)
                | E::Key(_)
                | E::Replace(_)
                | E::Clear(_)
                | E::Claude(_)
                | E::Codex(_)
                | E::Gemini(_)
                | E::Grok(_)
                | E::Hermes(_)
                | E::Pi(_)
                | E::Kimi(_)
                | E::Mcode(_)
                | E::Dsh(_)
                | E::Omp(_)
                | E::OpenClaw(_)
                | E::OpenCode(_)
                | E::Agy(_)
        )) | Message::Settings(S::AppProviders(L::Sort(_) | L::Category(_) | L::Notes(_)))
    );
    let leaving = matches!(
        message,
        Message::Settings(S::AppProviders(L::Close | L::Open(_) | L::Cancel))
            | Message::Settings(S::AppProviderEdit(E::Cancel | E::Create | E::Open { .. }))
            | Message::Settings(S::TabSelected(_) | S::SystemTabSelected(_))
            | Message::Settings(S::AppsCatalog(
                crate::app::message::settings::AppsCatalogMessage::HostInput(_)
            ))
            | Message::View(
                ViewMessage::ToggleSystemSettings
                    | ViewMessage::OpenSystemSettingsTab(_)
                    | ViewMessage::OpenSystemSettingsModelDetail(_, _)
            )
    );
    let busy=busy(app);
   let draft=app.app_providers.form.is_some()||app.app_providers.editor.is_some();
   desktop_provider_apps::message::leave::intercept(&mut app.app_providers.leave,message,edits,leaving,busy,draft)
}


pub(crate) fn choose(app:&mut App,choice:Choice)->Task<Message>{
use desktop_provider_apps::message::leave::{self,Action};
let state=&mut app.app_providers;
let action=leave::choose(&mut state.leave,&mut state.form,&mut state.editor,choice);
let task=match action{Action::None=>Task::none(),Action::Resume(m)=>Task::done(m),Action::SaveEdit=>edit::update(app,EditMessage::Save),Action::SaveList=>list::update(app,ListMessage::Save)};
if !busy(app){app.app_providers.leave.saving=false;} task
}
pub(crate) fn resume(app:&mut App)->Task<Message>{
let busy=busy(app);let state=&mut app.app_providers;
desktop_provider_apps::message::leave::resume(&mut state.leave,state.form.is_some()||state.editor.is_some(),busy).map_or_else(Task::none,Task::done)
}
pub(crate) fn overlay<'a>(app:&'a App,base:Element<'a,Message>)->Element<'a,Message>{
desktop_provider_apps::message::leave::overlay(app.app_providers.leave.pending.is_some(),busy(app),base,|choice|list::message(ListMessage::Leave(choice)))
}
