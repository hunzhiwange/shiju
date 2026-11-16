use crate::app::{App, Message};
use crate::app::message::settings::{ChannelsMessage, SettingsMessage};
use iced::Task;
use desktop_settings_channels::update::{Effect, Services};

pub(super) fn services(app: &App) -> Services {
 Services { config: crate::app::config::config_access(), client: crate::app::config::gateway_client(), project_choices: super::projects::project_choices(app) }
}
pub(super) fn map_effect(effect: Effect) -> Message {
 match effect {
  Effect::Channels(message) => Message::Settings(SettingsMessage::Channels(message)),
  effect => Message::Settings(SettingsMessage::Channels(ChannelsMessage::Effect(Box::new(effect)))),
 }
}
pub(super) fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
 let SettingsMessage::Channels(message) = message else { return Task::none(); };
 if let ChannelsMessage::Effect(effect) = message {
  return match *effect {
   Effect::Channels(message) => update(app, SettingsMessage::Channels(message)),
   Effect::Saved { tag, result } => Task::done(Message::Settings(SettingsMessage::AgentConfigSaved { tag, result })),
   Effect::WeComDelivery(message) => super::wecom_delivery::update(app, message),
   Effect::After(delay, message) => crate::app::message::after(delay, Message::Settings(SettingsMessage::Channels(message))),
   Effect::Error(error) => app.show_error_toast(error),
   Effect::Toast(kind, text, args) => {
    use desktop_settings_channels::update::ToastKind;
    let kind = match kind { ToastKind::Error => crate::app::state::ToastKind::Error, ToastKind::Warning => crate::app::state::ToastKind::Warning, ToastKind::Success => crate::app::state::ToastKind::Success };
    app.show_localized_toast(kind, &text, args.into_iter().map(Into::into).collect())
   }
  };
 }
 // Delivery updates must mutate the draft before loading execution candidates.
 let delivery = desktop_settings_channels::update::updates::delivery_event(&app.channels_settings, &message);
 if let Some((delivery, reload)) = delivery {
  let task = super::wecom_delivery::update(app, delivery);
  return if reload { Task::batch([task, super::execution::load_active(app)]) } else { task };
 }
 let services = services(app);
 desktop_settings_channels::update::updates::update(&mut app.channels_settings, &services, message).map(map_effect)
}
#[cfg(test)]
#[path = "updates_tests.rs"]
mod updates_tests;
