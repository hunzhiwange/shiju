//! Explicit native acceptance window using the production preview and message handler.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
mod apps;
mod desktop_update_helper;
mod fonts;
use crate::app::components::{editor::Editor, preview_panel};
use crate::app::message::editor::{self, EditorPopover};
use crate::app::{App, Message, PreviewTab};
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
        let content = format!(
            "fn 中文配置验收() {{\n\tlet color = \"#ff8800\";\n    let nested = ([1, 2, 3]);\n    // {}\n{}\n}}",
            "中文长行 Tab 与软换行 ".repeat(25),
            (0..65)
                .map(|i| format!("    let row_{i} = ({i}, \"#22aa88\");"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let path = "/tmp/editor-config-acceptance.rs".to_owned();
        app.preview_tabs = vec![PreviewTab {
            path: path.clone(),
            title: "editor-config-acceptance.rs".into(),
            content: content.clone(),
            is_dirty: false,
            truncated: false,
            read_only: false,
            auto_save_revision: 0,
            editor: Editor::new(&content, "rust"),
            scroll_id: iced::widget::Id::unique(),
            lsp_server_key: None,
            lsp_uri: None,
            lsp_language_id: None,
        }];
        app.active_preview_path = Some(path);
        app.app_theme = Theme::Light;
        app.editor_theme = Theme::Light;
        for tab in &mut app.preview_tabs {
            tab.editor.set_theme(Theme::Light);
        }
        app.file_manager_show_changes = false;
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
                for tab in &mut self.app.preview_tabs {
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
