//! App 消息与原生平台事件之间的窄适配。
use super::actions::{NativeMenuSnapshot, menu_sections, help_section};
use crate::app::{Message, message::{SettingsMessage, GeneralSettingsMessage, view::ViewMessage}};
use crate::app::components::top_bar::menus::MenuEntry;
use desktop_platform::native_menu as platform;

pub(crate) fn install_or_sync(snapshot: NativeMenuSnapshot) {
    #[cfg(test)]
    if unsafe { libc::pthread_main_np() } == 0 { return; }
    fn entries(items: Vec<MenuEntry>) -> Vec<platform::MenuEntry> {
        items.into_iter().map(|item| match item {
            MenuEntry::Separator => platform::MenuEntry::Separator,
            MenuEntry::Item { action, label, shortcut, .. } => platform::MenuEntry::Item { action, label, shortcut },
        }).collect()
    }
    let sections = menu_sections(&snapshot).map(|(title, items)| (title, entries(items)));
    let (title, items) = help_section();
    platform::install_or_sync(platform::NativeMenuSnapshot {
        locale: snapshot.locale, sections, help: (title, entries(items)),
        open_window_tabs: snapshot.open_window_tabs,
    });
}
pub(crate) fn subscription() -> iced::Subscription<Message> {
    platform::subscription().map(|event| match event {
        platform::Event::Quit => Message::Settings(SettingsMessage::General(GeneralSettingsMessage::TrayQuit)),
        platform::Event::TabSelected(id) => Message::View(ViewMessage::TabSelected(id)),
        platform::Event::Action(action) => Message::NativeMenu(action),
    })
}
