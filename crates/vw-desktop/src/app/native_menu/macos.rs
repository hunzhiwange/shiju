//! App 消息与原生平台事件之间的窄适配。
use super::actions::NativeMenuSnapshot;
use crate::app::{
    Message,
    message::{GeneralSettingsMessage, SettingsMessage, view::ViewMessage},
};
use desktop_platform::native_menu as platform;

pub(crate) fn install_or_sync(snapshot: NativeMenuSnapshot) {
    #[cfg(test)]
    if unsafe { libc::pthread_main_np() } == 0 {
        return;
    }
    platform::install_or_sync(desktop_navigation::native_menu::platform_snapshot(snapshot));
}
pub(crate) fn subscription() -> iced::Subscription<Message> {
    platform::subscription().map(|event| match event {
        platform::Event::Quit => {
            Message::Settings(SettingsMessage::General(GeneralSettingsMessage::TrayQuit))
        }
        platform::Event::TabSelected(id) => Message::View(ViewMessage::TabSelected(id)),
        platform::Event::Action(action) => Message::NativeMenu(action),
    })
}
