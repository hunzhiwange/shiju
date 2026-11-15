//! Conversation overlay uses the shared modal layout and theme palette.
use crate::app::{
    App, Message,
    message::voice::{Event, ScreenEvent},
    voice::conversation::Snapshot,
};
use iced::Element;
#[cfg(test)]
use crate::app::voice::conversation::Phase;
#[cfg(test)]
use iced::{Length, widget::{container, text}};

fn message(event: Event) -> Message {
    Message::Voice(event)
}
pub(crate) fn with_voice_chat<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    if app.voice.local_conversation.is_some() {
        return base;
    }
    let Some(chat) = &app.voice.conversation else {
        return base;
    };
    overlay_screen(base, chat.snapshot(), Some(&app.voice.screen))
}
pub(crate) fn overlay_screen<'a>(base: Element<'a, Message>, state: Snapshot, screen: Option<&'a crate::app::voice::screen_context::State>) -> Element<'a, Message> {
    desktop_voice_ui::components::voice_chat::overlay_screen(base, map, state, screen)
}
fn map(event: desktop_voice_ui::components::voice_chat::Event) -> Message {
    use desktop_voice_ui::components::voice_chat::{Event as Ui, LocalEvent as Local, ScreenEvent as Screen};
    message(match event {
        Ui::ChatEnd => Event::ChatEnd,
        Ui::ChatMute => Event::ChatMute,
        Ui::ChatInterrupt => Event::ChatInterrupt,
        Ui::Local(event) => Event::Local(match event {
            Local::Begin => crate::app::message::voice::local_chat::Event::Begin,
            Local::Finish => crate::app::message::voice::local_chat::Event::Finish,
        }),
        Ui::Screen(event) => Event::Screen(match event {
            Screen::Enable(value) => ScreenEvent::Enable(value),
            Screen::Permission => ScreenEvent::Permission,
            Screen::Offer => ScreenEvent::Offer,
            Screen::Cancel => ScreenEvent::Cancel,
            Screen::Confirm => ScreenEvent::Confirm,
            Screen::AttachmentChat => ScreenEvent::AttachmentChat,
        }),
    })
}
pub(crate) fn local_inline(app: &App) -> Option<Element<'_, Message>> {
    let chat = app.voice.local_conversation.as_ref()?;
    Some(desktop_voice_ui::components::voice_chat::local_inline(
        desktop_voice_ui::components::voice_chat::LocalView {
            stage: chat.stage, muted: chat.muted, notice: chat.notice.as_deref(), captions: &chat.captions,
        }, &app.voice.screen,
    ).map(map))
}
#[cfg(test)]
#[path = "voice_chat_tests.rs"]
mod tests;
