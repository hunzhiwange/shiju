//! Explicit native acceptance window using the production preview and message handler.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
mod apps;
mod desktop_update_helper;
mod fonts;
use crate::app::components::preview_panel;
use crate::app::message::editor;
use crate::app::{App, Message};
use desktop_editor::components::editor::Editor;
use desktop_editor::message::editor::EditorPopover;
use desktop_editor::preview::PreviewTab;
use iced::widget::{button, column};
use iced::{Element, Task, Theme};

struct Acceptance {
    app: App,
}
#[derive(Debug, Clone)]
enum Action {
    Ui(Message),
    Theme,
}
impl Acceptance {
    fn new() -> Self {
        let (mut app, _) = App::new();
        let tab = desktop_editor::acceptance::tab();
        let path = tab.path.clone();
        app.navigation_tabs.preview_tabs = vec![tab];
        app.navigation_tabs.active_preview_path = Some(path);
        app.app_theme = Theme::Light;
        app.editor_theme = Theme::Light;
        for tab in &mut app.navigation_tabs.preview_tabs {
            tab.editor.set_theme(Theme::Light);
        }
        app.file_manager.file_manager_show_changes = false;
        app.editor_popover = Some(EditorPopover::Settings);
        Self { app }
    }
    fn update(&mut self, action: Action) -> Task<Action> {
        match action {
            Action::Theme => {
                let theme =
                    if self.app.app_theme == Theme::Light { Theme::Dark } else { Theme::Light };
                self.app.app_theme = theme.clone();
                self.app.editor_theme = theme.clone();
                for tab in &mut self.app.navigation_tabs.preview_tabs {
                    tab.editor.set_theme(theme.clone());
                }
            }
            Action::Ui(Message::Editor(message)) => {
                return editor::update(&mut self.app, message).map(Action::Ui);
            }
            Action::Ui(Message::Preview(message)) => {
                return app::message::preview::update(&mut self.app, message).map(Action::Ui);
            }
            Action::Ui(_) => {}
        }
        Task::none()
    }
    fn view(&self) -> Element<'_, Action> {
        column![
            button("Light / Dark").on_press(Action::Theme),
            preview_panel::view(&self.app).map(Action::Ui)
        ]
        .into()
    }
}
fn main() -> iced::Result {
    iced::application(Acceptance::new, Acceptance::update, Acceptance::view)
        .title("Editor configuration native acceptance")
        .theme(|s: &Acceptance| s.app.app_theme.clone())
        .settings(iced::Settings {
            fonts: fonts::load_all(),
            default_font: iced::Font::with_name("PingFang SC"),
            ..Default::default()
        })
        .run()
}
