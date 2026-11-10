//! Real editor/terminal widget dispatch; run with --test-threads=1 (global editor focus).
use crate::app::components::{editor::Editor, terminal_panel};
use crate::app::message::{PreviewMessage, TerminalMessage, ViewMessage};
use crate::app::preview::PreviewTab;
use crate::app::{App, Message};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, layout, widget::Tree};
use iced::{Element, Event, Font, Length, Pixels, Point, Rectangle, Size, Task};
use iced_runtime::futures::futures::StreamExt;

fn view(app: &App, workspace: bool) -> Element<'_, Message> {
    if workspace {
        return crate::app::views::project::view(app);
    }
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
    workspace: bool,
}
impl Harness {
    async fn new() -> Self {
        Self::with_command("exec cat").await
    }

    async fn with_command(command: &str) -> Self {
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
            .add_terminal_with_command("focus".into(), Some(std::env::temp_dir()), command.into())
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
        let tree = Tree::new(view(&app, false).as_widget());
        Self { app, renderer, tree, workspace: false }
    }
    async fn task(&mut self, task: Task<Message>) {
        if let Some(mut stream) = iced_runtime::task::into_stream(task) {
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Widget(mut operation) = action {
                    let mut element = view(&self.app, self.workspace);
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
    fn dispatch(
        &mut self,
        event: Event,
        position: Point,
    ) -> (Vec<Message>, iced::advanced::input_method::InputMethod) {
        let mut messages = Vec::new();
        let mut element = view(&self.app, self.workspace);
        self.tree.diff(element.as_widget());
        let size = Size::new(800.0, 500.0);
        let node = element.as_widget_mut().layout(
            &mut self.tree,
            &self.renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        let mut shell = Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut self.tree,
            &event,
            Layout::new(&node),
            iced::mouse::Cursor::Available(position),
            &self.renderer,
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &Rectangle::with_size(size),
        );
        let input_method = shell.input_method().clone();
        (messages, input_method)
    }

    async fn event(&mut self, event: Event, position: Point) -> Vec<Vec<u8>> {
        let (messages, _) = self.dispatch(event, position);
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
        self.event(character_key("x"), Point::new(120.0, 360.0)).await
    }
}

fn character_key(text: &'static str) -> Event {
    let key = iced::keyboard::Key::Character(text.into());
    Event::Keyboard(iced::keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: iced::keyboard::key::Physical::Unidentified(
            iced::keyboard::key::NativeCode::Unidentified,
        ),
        location: iced::keyboard::Location::Standard,
        modifiers: iced::keyboard::Modifiers::empty(),
        text: Some(text.into()),
        repeat: false,
    })
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
async fn terminal_context_menu_preserves_selection_and_appends_to_chat_draft() {
    use super::ContentAction;
    use iced_runtime::user_interface::{Cache, UserInterface};

    let mut h = Harness::with_command("printf '终端输出 READY\\n'; exec cat").await;
    let point = Point::new(120.0, 360.0);
    let id = h.app.terminal.active_id.unwrap();
    let right_click = Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Right));
    h.click(point).await;
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !h.app.terminal.tabs[0].term.text().iter().any(|line| line.contains("READY")) {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("terminal output");

    h.event(right_click.clone(), point).await;
    assert!(h.app.terminal.content_menu.is_some());
    h.message(TerminalMessage::ContentMenuAction(id, ContentAction::SelectAll)).await;
    let selection = h.app.terminal.tabs[0].term.selected_text();
    assert!(selection.contains("终端输出 READY"));

    h.event(right_click.clone(), point).await;
    assert_eq!(h.app.terminal.tabs[0].term.selected_text(), selection);
    {
        let mut ui = UserInterface::build(
            view(&h.app, h.workspace),
            Size::new(800.0, 500.0),
            Cache::new(),
            &mut h.renderer,
        );
        let cursor = iced::mouse::Cursor::Available(point);
        let key = iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape);
        let mut messages = Vec::new();
        let (_, statuses) = ui.update(
            &[Event::Keyboard(iced::keyboard::Event::KeyPressed {
                modified_key: key.clone(),
                key,
                physical_key: iced::keyboard::key::Physical::Unidentified(
                    iced::keyboard::key::NativeCode::Unidentified,
                ),
                location: iced::keyboard::Location::Standard,
                modifiers: iced::keyboard::Modifiers::empty(),
                text: None,
                repeat: false,
            })],
            cursor,
            &mut h.renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(statuses, [iced::event::Status::Captured]);
        assert!(matches!(
            messages.as_slice(),
            [Message::Terminal(TerminalMessage::ContentMenuClose)]
        ));
        for theme in [iced::Theme::Light, iced::Theme::Dark] {
            ui.draw(
                &mut h.renderer,
                &theme,
                &iced::advanced::renderer::Style { text_color: theme.palette().text },
                cursor,
            );
        }
    }
    h.message(TerminalMessage::ContentMenuClose).await;

    h.app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("现有草稿");
    h.event(right_click, point).await;
    h.message(TerminalMessage::ContentMenuAction(id, ContentAction::InsertInChat)).await;
    assert!(h.app.terminal.content_menu.is_none());
    assert_eq!(
        h.app.current_session_runtime().input_editor.text(),
        format!("现有草稿\n{selection}")
    );
    assert!(h.key().await.is_empty());
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

#[tokio::test(flavor = "multi_thread")]
async fn terminal_focus_repeated_clicks_and_keys_preserve_tab_and_pty_identity() {
    let mut h = Harness::new().await;
    let widget_id = h.app.terminal.tabs[0].term.widget_id().clone();
    let id = h.app.terminal.tabs[0].id;
    for _ in 0..20 {
        h.click(Point::new(120.0, 360.0)).await;
        assert_eq!(h.key().await, vec![b"x".to_vec()]);
        assert_eq!(h.app.terminal.tabs.len(), 1);
        assert_eq!(h.app.terminal.active_id, Some(id));
        assert_eq!(h.app.terminal.tabs[0].term.widget_id(), &widget_id);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_ime_requests_native_composition_only_while_focused() {
    use iced::advanced::input_method::{Event as Ime, InputMethod, Purpose};

    let mut h = Harness::new().await;
    let point = Point::new(120.0, 360.0);
    let redraw = || Event::Window(iced::window::Event::RedrawRequested(std::time::Instant::now()));
    assert!(!h.dispatch(redraw(), point).1.is_enabled());
    h.click(point).await;

    let InputMethod::Enabled { cursor, purpose, preedit } = h.dispatch(redraw(), point).1 else {
        panic!("focused terminal must request native IME on redraw");
    };
    assert_eq!(purpose, Purpose::Terminal);
    assert!(cursor.y >= 314.0 && cursor.y < 500.0);
    assert!(preedit.is_none());

    assert!(
        h.event(Event::InputMethod(Ime::Preedit("zhong".into(), Some(5..5))), point)
            .await
            .is_empty()
    );
    let InputMethod::Enabled { preedit: Some(preedit), .. } = h.dispatch(redraw(), point).1 else {
        panic!("IME preedit should be presented at the terminal caret");
    };
    assert_eq!(preedit.content, "zhong");
    assert_eq!(preedit.selection, Some(5..5));

    h.event(Event::Window(iced::window::Event::Unfocused), point).await;
    assert!(!h.dispatch(redraw(), point).1.is_enabled());
    assert!(h.event(Event::InputMethod(Ime::Commit("中".into())), point).await.is_empty());

    h.event(Event::Window(iced::window::Event::Focused), point).await;
    assert!(matches!(h.dispatch(redraw(), point).1, InputMethod::Enabled { preedit: None, .. }));

    h.click(Point::new(120.0, 100.0)).await;
    assert!(!matches!(
        h.dispatch(redraw(), point).1,
        InputMethod::Enabled { purpose: Purpose::Terminal, .. }
    ));
    assert!(h.event(Event::InputMethod(Ime::Commit("文".into())), point).await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_ime_writes_committed_utf8_once_to_the_pty() {
    use iced::advanced::input_method::Event as Ime;

    let mut h = Harness::with_command(concat!(
        "stty -echo; printf 'IME-READY\\n'; ",
        r#"IFS= read -r line; printf 'IME:%s\n' "$line"; exec cat"#,
    ))
    .await;
    let point = Point::new(120.0, 360.0);
    h.click(point).await;
    let id = h.app.terminal.active_id.unwrap();

    wait_for_line(&mut h, id, "IME-READY").await;
    for event in [
        Ime::Opened,
        Ime::Preedit("zhongwen".into(), Some(8..8)),
        Ime::Preedit(String::new(), None),
    ] {
        assert!(h.event(Event::InputMethod(event), point).await.is_empty());
    }
    let writes = h.event(Event::InputMethod(Ime::Commit("中文".into())), point).await;
    assert_eq!(writes, vec!["中文".as_bytes().to_vec()]);
    assert!(h.app.preview_tabs[0].editor.content().is_empty());
    assert_eq!(h.app.terminal.tabs.len(), 1);
    for mut bytes in writes {
        bytes.push(b'\n');
        h.message(TerminalMessage::Event(iced_term::Event::BackendCall(
            id,
            iced_term::BackendCommand::Write(bytes),
        )))
        .await;
    }
    wait_for_line(&mut h, id, "IME:中文").await;
}

async fn wait_for_line(h: &mut Harness, id: u64, expected: &str) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            h.message(TerminalMessage::Event(iced_term::Event::BackendCall(
                id,
                iced_term::BackendCommand::Resize(None, None),
            )))
            .await;
            if h.app.terminal.tabs[0].term.text().iter().any(|line| line == expected) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("PTY should read the committed input");
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_fullscreen_preserves_pty_focus_and_output() {
    let mut h = Harness::with_command("stty -echo -icanon; printf 'READY:'; exec cat").await;
    h.workspace = true;
    let id = h.app.terminal.active_id.unwrap();
    let widget_id = h.app.terminal.tabs[0].term.widget_id().clone();
    wait_for_line(&mut h, id, "READY:").await;

    for (fullscreen, toolbar_y, key, output) in
        [(true, 305.0, "x", "READY:x"), (false, 17.0, "y", "READY:xy")]
    {
        let point = Point::new(784.0, toolbar_y);
        let (messages, _) = h.dispatch(
            Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            point,
        );
        for message in messages {
            let _ = h.app.update(message);
        }
        let (messages, _) = h.dispatch(
            Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
            point,
        );
        assert!(matches!(
            messages.as_slice(),
            [Message::Terminal(TerminalMessage::ToggleFullscreen)]
        ));
        for message in messages {
            let task = h.app.update(message);
            h.task(task).await;
        }

        assert_eq!(h.app.terminal.is_fullscreen, fullscreen);
        assert_eq!(h.app.terminal.active_id, Some(id));
        assert_eq!(h.app.terminal.tabs[0].term.widget_id(), &widget_id);
        let (messages, _) = h.dispatch(character_key(key), Point::new(120.0, 360.0));
        assert!(messages.iter().any(|message| matches!(
            message,
            Message::Terminal(TerminalMessage::Event(iced_term::Event::BackendCall(
                event_id,
                iced_term::BackendCommand::Write(bytes)
            ))) if *event_id == id && bytes == key.as_bytes()
        )));
        for message in messages {
            let _ = h.app.update(message);
        }
        wait_for_line(&mut h, id, output).await;
        eprintln!("terminal {id}, widget {widget_id:?}, PTY output {output}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_fullscreen_clears_on_hide_project_switch_and_pty_exit() {
    let mut h = Harness::new().await;
    let id = h.app.terminal.active_id.unwrap();
    let widget_id = h.app.terminal.tabs[0].term.widget_id().clone();

    h.message(TerminalMessage::DragStarted).await;
    h.app.terminal.tab_context_menu_id = Some(id);
    h.app.terminal.tab_context_menu_pos = Some((20.0, 30.0));
    h.app.terminal.content_menu = Some((id, Point::new(50.0, 60.0)));
    h.message(TerminalMessage::ToggleFullscreen).await;
    assert!(h.app.terminal.is_fullscreen);
    assert!(h.app.terminal.tab_context_menu_id.is_none());
    assert!(h.app.terminal.tab_context_menu_pos.is_none());
    assert!(h.app.terminal.content_menu.is_none());
    h.message(TerminalMessage::DragStarted).await;
    assert!(!h.app.terminal.is_dragging);
    assert!(h.app.terminal.drag_anchor_y.is_none());

    let _ = h.app.update(Message::BootstrapAppConfig(Ok(serde_json::json!({
        "show_terminal": false,
    }))));
    assert!(!h.app.terminal.is_visible);
    assert!(!h.app.terminal.is_fullscreen);
    let _ = h.app.update(Message::BootstrapAppConfig(Ok(serde_json::json!({
        "show_terminal": true,
    }))));
    assert!(h.app.terminal.is_visible);
    assert!(!h.app.terminal.is_fullscreen);

    h.message(TerminalMessage::ToggleFullscreen).await;
    // Check synchronous visibility changes without writing desktop preferences.
    let _ = h.app.update(Message::View(ViewMessage::ToggleTerminalPanel));
    assert!(!h.app.terminal.is_visible);
    assert!(!h.app.terminal.is_fullscreen);
    h.message(TerminalMessage::ToggleFullscreen).await;
    assert!(!h.app.terminal.is_fullscreen);
    let _ = h.app.update(Message::View(ViewMessage::ToggleTerminalPanel));
    assert!(h.app.terminal.is_visible);
    assert!(!h.app.terminal.is_fullscreen);
    assert_eq!(h.app.terminal.height, 220.0);

    h.message(TerminalMessage::ToggleFullscreen).await;
    let project = std::env::temp_dir().to_string_lossy().into_owned();
    h.app.switch_project_terminal(Some(project.clone()), None);
    assert!(!h.app.terminal.is_fullscreen);
    h.app.switch_project_terminal(None, Some(project));
    assert!(h.app.terminal.is_visible);
    assert!(!h.app.terminal.is_fullscreen);
    assert_eq!(h.app.terminal.active_id, Some(id));
    assert_eq!(h.app.terminal.tabs[0].term.widget_id(), &widget_id);

    h.message(TerminalMessage::ToggleFullscreen).await;
    h.message(TerminalMessage::Event(iced_term::Event::BackendCall(
        id,
        iced_term::BackendCommand::ProcessAlacrittyEvent(iced_term::AlacrittyEvent::Exit),
    )))
    .await;
    assert!(h.app.terminal.tabs.is_empty());
    assert!(!h.app.terminal.is_visible);
    assert!(!h.app.terminal.is_fullscreen);
}
