//! Real editor/terminal widget dispatch; run with --test-threads=1 (global editor focus).
use crate::app::components::{editor::Editor, terminal_panel};
use crate::app::message::{PreviewMessage, TerminalMessage, ViewMessage};
use crate::app::preview::PreviewTab;
use crate::app::{App, Message};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, layout, widget::Tree};
use iced::{Element, Event, Font, Length, Pixels, Point, Rectangle, Size, Task};
use iced_runtime::futures::futures::StreamExt;

fn view(app: &App) -> Element<'_, Message> {
    iced::widget::column![
        iced::widget::container(
            app.preview_tabs[0]
                .editor
                .content_view(|e| { Message::Preview(PreviewMessage::EditorEvent(e)) })
        )
        .height(Length::Fixed(280.0)),
        iced::widget::container(terminal_panel::view(app)).height(Length::Fixed(220.0)),
    ]
    .into()
}

struct Harness {
    app: App,
    renderer: iced::Renderer,
    tree: Tree,
}
impl Harness {
    async fn new() -> Self {
        let mut app = App::new().0;
        app.terminal = crate::app::TerminalState::blank_with_settings(
            true,
            crate::app::Shell::Bash,
            crate::app::TerminalTheme::System,
            "monospace".into(),
            15.0,
            220.0,
        );
        app.terminal
            .add_terminal_with_command(
                "focus".into(),
                Some(std::env::temp_dir()),
                "exec cat".into(),
            )
            .expect("PTY");
        app.preview_tabs.push(PreviewTab {
            path: "/tmp/terminal-focus.rs".into(),
            title: "focus.rs".into(),
            content: "".into(),
            is_dirty: false,
            truncated: false,
            read_only: false,
            auto_save_revision: 0,
            editor: Editor::new("", "rust"),
            scroll_id: iced::widget::Id::unique(),
            lsp_server_key: None,
            lsp_uri: None,
            lsp_language_id: None,
        });
        let renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("renderer");
        let tree = Tree::new(view(&app).as_widget());
        Self { app, renderer, tree }
    }
    async fn task(&mut self, task: Task<Message>) {
        if let Some(mut stream) = iced_runtime::task::into_stream(task) {
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Widget(mut operation) = action {
                    let mut element = view(&self.app);
                    self.tree.diff(element.as_widget());
                    let node = element.as_widget_mut().layout(
                        &mut self.tree,
                        &self.renderer,
                        &layout::Limits::new(Size::ZERO, Size::new(800.0, 500.0)),
                    );
                    element.as_widget_mut().operate(
                        &mut self.tree,
                        Layout::new(&node),
                        &self.renderer,
                        operation.as_mut(),
                    );
                }
            }
        }
    }
    async fn message(&mut self, message: TerminalMessage) {
        let task = super::update(&mut self.app, message);
        self.task(task).await;
    }
    async fn event(&mut self, event: Event, position: Point) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();
        {
            let mut element = view(&self.app);
            self.tree.diff(element.as_widget());
            let size = Size::new(800.0, 500.0);
            let node = element.as_widget_mut().layout(
                &mut self.tree,
                &self.renderer,
                &layout::Limits::new(Size::ZERO, size),
            );
            element.as_widget_mut().update(
                &mut self.tree,
                &event,
                Layout::new(&node),
                iced::mouse::Cursor::Available(position),
                &self.renderer,
                &mut iced::advanced::clipboard::Null,
                &mut Shell::new(&mut messages),
                &Rectangle::with_size(size),
            );
        }
        let mut writes = Vec::new();
        for message in messages {
            match message {
                Message::Preview(PreviewMessage::EditorEvent(e)) => {
                    let _ = self.app.preview_tabs[0].editor.inner.update(&e);
                }
                Message::Terminal(TerminalMessage::Event(iced_term::Event::BackendCall(
                    _,
                    iced_term::BackendCommand::Write(bytes),
                ))) => {
                    // Inspect the real widget's PTY delivery without executing typed commands.
                    writes.push(bytes);
                }
                Message::Terminal(e) => self.message(e).await,
                _ => {}
            }
        }
        writes
    }
    async fn click(&mut self, point: Point) {
        self.event(
            Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            point,
        )
        .await;
        self.event(
            Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
            point,
        )
        .await;
    }
    async fn key(&mut self) -> Vec<Vec<u8>> {
        let key = iced::keyboard::Key::Character("x".into());
        self.event(
            Event::Keyboard(iced::keyboard::Event::KeyPressed {
                modified_key: key.clone(),
                key,
                physical_key: iced::keyboard::key::Physical::Unidentified(
                    iced::keyboard::key::NativeCode::Unidentified,
                ),
                location: iced::keyboard::Location::Standard,
                modifiers: iced::keyboard::Modifiers::empty(),
                text: Some("x".into()),
                repeat: false,
            }),
            Point::new(120.0, 360.0),
        )
        .await
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        self.app.preview_tabs[0].editor.inner.lose_focus();
        let ids: Vec<_> = self.app.terminal.tabs.iter().map(|tab| tab.id).collect();
        for id in ids {
            self.app.terminal.close_terminal(id);
        }
        self.app.terminal.reap_retired();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_focus_click_delivers_keys_in_both_directions() {
    let mut h = Harness::new().await;
    for turn in 0..3 {
        // Separate clicks spatially so the editor does not classify a rapid
        // round trip as double/triple click (which intentionally selects text).
        h.click(Point::new(120.0 + turn as f32 * 20.0, 80.0)).await;
        let before = h.app.preview_tabs[0].editor.content();
        assert!(h.key().await.is_empty());
        assert_ne!(h.app.preview_tabs[0].editor.content(), before);
        h.click(Point::new(120.0, 360.0)).await;
        let before = h.app.preview_tabs[0].editor.content();
        assert_eq!(h.key().await, vec![b"x".to_vec()]);
        assert_eq!(h.app.preview_tabs[0].editor.content(), before);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_focus_background_events_preserve_editor_input() {
    let mut h = Harness::new().await;
    h.click(Point::new(120.0, 80.0)).await;
    h.app.focus_area = crate::app::FocusArea::Preview;
    let id = h.app.terminal.active_id.unwrap();
    for command in [
        iced_term::BackendCommand::Resize(None, None),
        iced_term::BackendCommand::ProcessAlacrittyEvent(iced_term::AlacrittyEvent::Wakeup),
    ] {
        h.message(TerminalMessage::Event(iced_term::Event::BackendCall(id, command))).await;
        assert_eq!(h.app.focus_area, crate::app::FocusArea::Preview);
        let before = h.app.preview_tabs[0].editor.content();
        assert!(h.key().await.is_empty());
        assert_ne!(h.app.preview_tabs[0].editor.content(), before);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_focus_select_and_reopen_deliver_keys_without_click() {
    let mut h = Harness::new().await;
    let id = h.app.terminal.active_id.unwrap();
    let second = h
        .app
        .terminal
        .add_terminal_with_command("second".into(), Some(std::env::temp_dir()), "exec cat".into())
        .expect("second PTY");
    h.click(Point::new(120.0, 80.0)).await;
    let task = super::tabs::update(&mut h.app, TerminalMessage::Select(id));
    h.task(task).await;
    assert_eq!(h.key().await, vec![b"x".to_vec()]);
    let task = super::tabs::update(&mut h.app, TerminalMessage::Close(id));
    h.task(task).await;
    assert_eq!(h.app.terminal.active_id, Some(second));
    assert_eq!(h.key().await, vec![b"x".to_vec()]);
    assert_eq!(h.app.preview_tabs[0].editor.content(), "");
    h.app.terminal.is_visible = false;
    h.click(Point::new(120.0, 80.0)).await;
    let task =
        crate::app::message::view::panel::update(&mut h.app, ViewMessage::ToggleTerminalPanel);
    h.task(task).await;
    assert_eq!(h.key().await, vec![b"x".to_vec()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_focus_rename_modal_does_not_type_into_terminal_or_editor() {
    let mut h = Harness::new().await;
    h.click(Point::new(120.0, 360.0)).await;
    let id = h.app.terminal.active_id.unwrap();
    let task = super::rename::update(&mut h.app, TerminalMessage::RenameStart(id));
    h.task(task).await;
    h.click(Point::new(350.0, 400.0)).await;
    let before = h.app.preview_tabs[0].editor.content();
    let title_before = h.app.terminal.tabs[0].edit_title.clone();
    assert!(h.key().await.is_empty());
    assert_eq!(h.app.preview_tabs[0].editor.content(), before);
    assert_ne!(h.app.terminal.tabs[0].edit_title, title_before);
}
