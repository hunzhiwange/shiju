//! Cross-domain screen sharing dispatch.
use super::{Event, end_conversation};
use crate::app::{App, Message};
#[cfg(test)]
use crate::app::voice::screen_context::{self, Ticket};
use iced::Task;
use desktop_voice_ui::message::voice_screen::{self as domain, Outcome};
pub use domain::{Owner, ScreenEvent};
#[cfg(test)]
use vw_api_voice::VoiceCounter;
fn owner(app: &App) -> Option<Owner> {
    if app.voice.local_conversation.is_some() {
        super::local_chat::screen_call(app).ok().map(Owner::Local)
    } else {
        app.voice.conversation.as_ref()?.snapshot().generation.map(Owner::Cloud)
    }
}

fn dispatch(app: &mut App, outcome: Outcome) -> Task<Message> {
    let wrap = |event| Message::Voice(Event::Screen(event));
    match outcome {
        Outcome::None => Task::none(),
        Outcome::Error(error) => app.show_error_toast(error),
        Outcome::Task(task) => task.map(wrap),
        Outcome::Offer(task) => Task::batch([task.map(wrap), app.show_info_toast("请在三秒内切换到要分享的应用，然后返回这里确认窗口。此时只选择目标，不采集屏幕。")]),
        Outcome::Permission => {
            if app.voice.local_conversation.is_some() && let Err(error) = super::local_chat::screen_call(app) { return app.show_error_toast(error); }
            use crate::app::voice::permissions::{Permission, SystemPermissions, VoicePermissions};
            if let Err(error) = VoicePermissions::new(SystemPermissions).request_from_user_action(Permission::ScreenRecording) { return app.show_error_toast(error.to_string()); }
            Task::none()
        }
        Outcome::AttachmentChat => { end_conversation(app); app.show_info_toast("已返回当前聊天。请使用附件按钮选择要分享的图像，并确认发送。") }
        Outcome::Share(ticket, frame_owner, payload) => match frame_owner {
            Owner::Local(call) => super::local_chat::share_screen(app, call, payload),
            Owner::Cloud(generation) => {
                if let Some(chat) = &app.voice.conversation {
                    if let Err(error) = chat.share(ticket, generation, payload) { return app.show_error_toast(error); }
                    return app.show_info_toast("单帧已交给当前语音通道；没有持续采集。远端可能保留已分享内容。");
                }
                Task::none()
            }
        }
    }
}
pub(super) fn update(app: &mut App, event: ScreenEvent) -> Task<Message> {
    let active = app.voice.chat_active();
    let allowed = app.voice.local_conversation.is_some() || app.voice.conversation.as_ref().is_some_and(|c| matches!(c.snapshot().phase, crate::app::voice::conversation::Phase::Listening | crate::app::voice::conversation::Phase::Speaking | crate::app::voice::conversation::Phase::Muted));
    let current = owner(app);
    let outcome = domain::update(&mut app.voice.screen, event, active, allowed, current);
    dispatch(app, outcome)
}
#[cfg(test)]
#[path = "voice_screen_tests.rs"]
mod tests;

#[cfg(test)]
pub(super) fn receive(app: &mut App, ticket: Ticket, frame_owner: Owner, result: Result<Vec<u8>, String>, permitted: bool) -> Task<Message> {
    let current = owner(app);
    let outcome = domain::receive(&mut app.voice.screen, current, ticket, frame_owner, result, permitted);
    dispatch(app, outcome)
}
