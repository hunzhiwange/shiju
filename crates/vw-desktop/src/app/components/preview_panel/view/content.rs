use crate::app::{App, Message, message::PreviewMessage};
use iced::{Element, Length};
use desktop_editor::components::preview_panel::view::{content::{self, ContentTab}, ContentMessage};
pub fn build_content_base(app: &App) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let tab = app.navigation_tabs.active_preview_path.as_ref().and_then(|path| app.navigation_tabs.preview_tabs.iter().find(|tab| &tab.path == path)).map(|tab| ContentTab {
        path: &tab.path, editor: &tab.editor, scroll_id: tab.scroll_id.clone(), truncated: tab.truncated,
        message: app.preview_messages.get(&tab.path).map(|message| message.render(i18n).expect("preview template")),
    });
    let empty: Element<'_, Message> = if matches!(app.navigation_tabs.screen, crate::app::Screen::Preview) || !app.file_manager.file_manager_show_changes || app.navigation_tabs.active_preview_path.is_some() {
        iced::widget::container(iced::widget::text(i18n.__("未选择文件"))).width(Length::Fill).height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center).align_y(iced::alignment::Vertical::Center).into()
    } else { crate::app::components::git_panel::view(app) };
    content::build_content_base(tab, app.navigation_tabs.active_preview_path.is_some(), i18n, empty.map(ContentMessage::External),
        #[cfg(not(target_arch = "wasm32"))]
        super::super::lsp_overlay::lsp_overlay(app).map(ContentMessage::External),
    ).map(|action| match action {
        ContentMessage::External(message) => message,
        ContentMessage::EditorEvent(event) => Message::Preview(PreviewMessage::EditorEvent(event)),
        #[cfg(not(target_arch = "wasm32"))]
        ContentMessage::MouseEntered => Message::Preview(PreviewMessage::EditorMouseEntered),
        #[cfg(not(target_arch = "wasm32"))]
        ContentMessage::MouseExited => Message::Preview(PreviewMessage::EditorMouseExited),
    })
}
