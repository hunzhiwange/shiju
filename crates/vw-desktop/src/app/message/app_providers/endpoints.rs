pub use desktop_provider_apps::message::endpoints::Event;
use crate::app::{App,Message};
pub(crate) fn message(event:Event)->Message { super::list::message(super::list::ListMessage::Endpoints(event)) }
pub(crate) fn update(app:&mut App,event:Event)->iced::Task<Message>{
 let current=app.app_providers.scope.as_ref().and_then(|s|super::list::current(app,s.app).ok());
 let reload = matches!(event, Event::Cancel)
     && !app.app_providers.endpoints.as_ref().is_some_and(|f| f.busy);
 let task = super::provider_update::endpoints(app,current,event);
 if reload {
     if let Some(speed) = &mut app.app_providers.endpoint_speed {
         speed.generation = app.app_providers.generation;
         let diagnostics = crate::app::message::agent_apps_diagnostics::endpoints::update(
             app, crate::app::message::agent_apps_diagnostics::endpoints::Event::Reload,
         );
         return iced::Task::batch([task, diagnostics]);
     }
 }
 task
}
