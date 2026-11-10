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
        iced_term::BackendCommand::ProcessAlacrittyEvent(iced_term::AlacrittyEvent::Exit),
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
            if h.app
                .terminal
                .tabs
                .iter()
                .find(|tab| tab.id == id)
                .unwrap()
                .term
                .text()
                .iter()
                .any(|line| line == expected)
            {
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

async fn click_split_menu(h: &mut Harness, button: Point, item: usize) {
    use iced_runtime::user_interface::{Cache, UserInterface};

    let mut cache = Cache::new();
    for (index, point) in [
        button,
        Point::new(button.x - 10.0, (button.y + 19.0).min(372.0) + 16.0 + item as f32 * 30.0),
    ]
    .into_iter()
    .enumerate()
    {
        let mut ui = UserInterface::build(
            view(&h.app, h.workspace),
            Size::new(800.0, 500.0),
            cache,
            &mut h.renderer,
        );
        let mut messages = Vec::new();
        let _ = ui.update(
            &[
                Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
                Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
            ],
            iced::mouse::Cursor::Available(point),
            &mut h.renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        if index == 1 {
            for theme in [iced::Theme::Light, iced::Theme::Dark] {
                ui.draw(
                    &mut h.renderer,
                    &theme,
                    &iced::advanced::renderer::Style { text_color: theme.palette().text },
                    iced::mouse::Cursor::Available(point),
                );
            }
        }
        cache = ui.into_cache();
        for message in messages {
            let task = h.app.update(message);
            h.task(task).await;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_context_menu_splits_the_clicked_pane() {
    use iced::mouse::{Button, Event as Mouse};
    use iced::widget::pane_grid::Direction;
    use iced_runtime::user_interface::{Cache, UserInterface};

    let mut h = Harness::new().await;
    let left = h.app.terminal.panes.focused;
    h.app.terminal.split_terminal(left, Direction::Right, None).unwrap();
    let right = h.app.terminal.panes.focused;

    h.event(Event::Mouse(Mouse::ButtonPressed(Button::Right)), Point::new(120.0, 360.0)).await;
    assert_eq!(h.app.terminal.panes.focused, left);
    assert!(h.app.terminal.content_menu.is_some());

    let mut messages = Vec::new();
    {
        // Leave room below the click for the full menu. Split Down is the
        // fourth split entry, immediately before the clipboard actions.
        let mut ui = UserInterface::build(
            view(&h.app, h.workspace),
            Size::new(800.0, 800.0),
            Cache::new(),
            &mut h.renderer,
        );
        let _ = ui.update(
            &[
                Event::Mouse(Mouse::ButtonPressed(Button::Left)),
                Event::Mouse(Mouse::ButtonReleased(Button::Left)),
            ],
            iced::mouse::Cursor::Available(Point::new(160.0, 510.0)),
            &mut h.renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
    }
    assert!(matches!(
        messages.as_slice(),
        [Message::Terminal(TerminalMessage::ContentMenuAction(
            _,
            super::ContentAction::Split(Direction::Down)
        ))]
    ));
    for message in messages {
        let task = h.app.update(message);
        h.task(task).await;
    }

    let panes = &h.app.terminal.panes;
    let regions = panes.grid.layout().pane_regions(
        crate::app::terminal::panes::PANE_SPACING,
        0.0,
        Size::new(800.0, 220.0),
    );
    assert_eq!(regions.len(), 3);
    assert_eq!(regions[&right].height, 220.0);
    assert_eq!(regions[&panes.focused].x, regions[&left].x);
    assert!(regions[&panes.focused].y > regions[&left].y);
    assert!(h.app.terminal.content_menu.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_split_shortcut_only_runs_in_the_focused_terminal() {
    use iced::keyboard::{self, Modifiers, key};

    let mut h = Harness::new().await;
    let terminal_point = Point::new(120.0, 360.0);
    let editor_point = Point::new(120.0, 90.0);
    let macos = cfg!(target_os = "macos");
    let shortcut = |repeat| {
        let key = keyboard::Key::Character(if macos { "d" } else { "5" }.into());
        Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: key::Physical::Code(if macos {
                key::Code::KeyD
            } else {
                key::Code::Digit5
            }),
            location: keyboard::Location::Standard,
            modifiers: if macos { Modifiers::LOGO } else { Modifiers::CTRL | Modifiers::SHIFT },
            text: None,
            repeat,
        })
    };

    h.click(editor_point).await;
    h.event(shortcut(false), editor_point).await;
    assert_eq!(h.app.terminal.panes.grid.len(), 1);

    h.click(terminal_point).await;
    assert!(h.event(shortcut(false), terminal_point).await.is_empty());
    assert_eq!(h.app.terminal.panes.grid.len(), 2);

    assert!(h.event(shortcut(true), terminal_point).await.is_empty());
    assert_eq!(h.app.terminal.panes.grid.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_split_toolbar_creates_independent_panes_in_each_direction() {
    use crate::app::terminal::panes::PANE_SPACING;

    let project = tempfile::tempdir().unwrap();
    for item in 0..4 {
        let mut h = Harness::new().await;
        h.app.project_path = Some(project.path().to_string_lossy().into_owned());
        h.app.desktop_locale = vw_config_types::ui::DesktopLocale::TraditionalChinese;
        h.app.terminal.tabs[0].title = "开发环境终端标签".repeat(15);
        let original = h.app.terminal.active_id.unwrap();
        let original_pane = h.app.terminal.panes.focused;
        click_split_menu(&mut h, Point::new(720.0, 297.0), item).await;
        assert_eq!(h.app.terminal.tabs.len(), 2, "split menu item {item} must start a PTY");

        let panes = &h.app.terminal.panes;
        let new = h.app.terminal.active_id.unwrap();
        assert_ne!(new, original);
        assert_eq!(panes.grid.get(original_pane).unwrap().tabs, [original]);
        assert_eq!(panes.grid.get(panes.focused).unwrap().tabs, [new]);
        let regions = panes.grid.layout().pane_regions(PANE_SPACING, 0.0, Size::new(800.0, 220.0));
        let old_region = regions[&original_pane];
        let new_region = regions[&panes.focused];
        match item {
            0 => assert!(new_region.x < old_region.x && new_region.y == old_region.y),
            1 => assert!(new_region.x > old_region.x && new_region.y == old_region.y),
            2 => assert!(new_region.y < old_region.y && new_region.x == old_region.x),
            3 => assert!(new_region.y > old_region.y && new_region.x == old_region.x),
            _ => unreachable!(),
        }
        let point = Point::new(new_region.center_x(), 280.0 + new_region.y + 62.0);
        let (messages, _) = h.dispatch(character_key("x"), point);
        assert!(messages.iter().any(|message| matches!(message,
            Message::Terminal(TerminalMessage::Event(iced_term::Event::BackendCall(id,
                iced_term::BackendCommand::Write(_)))) if *id == new
        )));
    }
}

async fn send_terminal_input(h: &mut Harness, id: u64, event: Event, point: Point, bytes: &[u8]) {
    let (messages, _) = h.dispatch(event, point);
    let mut writes = Vec::new();
    for message in messages {
        if let Message::Terminal(TerminalMessage::Event(iced_term::Event::BackendCall(
            target,
            iced_term::BackendCommand::Write(ref data),
        ))) = message
        {
            writes.push((target, data.clone()));
        }
        let task = h.app.update(message);
        h.task(task).await;
    }
    assert_eq!(writes, [(id, bytes.to_vec())]);
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_split_routes_input_and_closes_empty_panes() {
    use iced::advanced::{input_method::Event as Ime, subscription::into_recipes};

    let project = tempfile::tempdir().unwrap();
    let mut h = Harness::with_command("stty -echo -icanon; printf 'FIRST:'; exec cat").await;
    h.app.project_path = Some(project.path().to_string_lossy().into_owned());
    let first = h.app.terminal.active_id.unwrap();
    let left = h.app.terminal.panes.focused;
    wait_for_line(&mut h, first, "FIRST:").await;
    click_split_menu(&mut h, Point::new(720.0, 297.0), 1).await;
    assert_eq!(h.app.terminal.tabs.len(), 2, "right split needs an independent PTY");
    let second = h.app.terminal.active_id.unwrap();
    let right = h.app.terminal.panes.focused;
    let first_point = Point::new(120.0, 360.0);
    let second_point = Point::new(520.0, 360.0);

    h.click(first_point).await;
    assert_eq!(h.app.terminal.active_id, Some(first));
    send_terminal_input(&mut h, first, character_key("x"), first_point, b"x").await;
    send_terminal_input(
        &mut h,
        first,
        Event::InputMethod(Ime::Commit("中文".into())),
        first_point,
        "中文".as_bytes(),
    )
    .await;
    wait_for_line(&mut h, first, "FIRST:x中文").await;

    h.event(
        Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Right)),
        second_point,
    )
    .await;
    assert_eq!(h.app.terminal.content_menu.map(|(id, _)| id), Some(second));
    h.message(TerminalMessage::ContentMenuClose).await;
    let command = "printf '\nSECOND:%s\n' ready; pwd -P
";
    send_terminal_input(&mut h, second, character_key(command), second_point, command.as_bytes())
        .await;
    wait_for_line(&mut h, second, "SECOND:ready").await;
    let canonical_path = project.path().canonicalize().unwrap();
    wait_for_line(&mut h, second, canonical_path.to_str().unwrap()).await;
    assert!(h.app.terminal.tabs[0].term.text().iter().all(|line| !line.contains("SECOND:")));
    assert!(h.app.terminal.tabs[1].term.text().iter().all(|line| !line.contains("FIRST:")));

    // The per-pane + button targets its pane even when the other pane owns focus.
    h.click(Point::new(350.0, 297.0)).await;
    let left_extra = h.app.terminal.active_id.unwrap();
    assert_eq!(h.app.terminal.panes.grid.get(left).unwrap().tabs, [first, left_extra]);
    h.click(Point::new(688.0, 297.0)).await;
    let right_extra = h.app.terminal.active_id.unwrap();
    assert_eq!(h.app.terminal.panes.grid.get(right).unwrap().tabs, [second, right_extra]);
    for tab in h.app.terminal.tabs.iter_mut().filter(|tab| [second, right_extra].contains(&tab.id))
    {
        tab.title = "右侧终端的长标签".repeat(15);
        tab.default_title = false;
    }
    h.click(Point::new(430.0, 291.0)).await;
    assert_eq!(h.app.terminal.active_id, Some(second));
    assert_eq!(h.app.terminal.panes.grid.get(left).unwrap().active_id, Some(left_extra));
    h.click(Point::new(36.0, 291.0)).await;
    assert_eq!(h.app.terminal.active_id, Some(first));
    assert_eq!(h.app.terminal.panes.grid.get(right).unwrap().active_id, Some(second));

    // Split the left pane again, leaving the right pane intact.
    click_split_menu(&mut h, Point::new(382.0, 297.0), 3).await;
    assert_eq!(h.app.terminal.panes.grid.len(), 3);
    let bottom = h.app.terminal.panes.focused;
    let regions = h.app.terminal.panes.grid.layout().pane_regions(
        crate::app::terminal::panes::PANE_SPACING,
        0.0,
        Size::new(800.0, 220.0),
    );
    assert_eq!(regions[&bottom].x, regions[&left].x);
    assert!(regions[&bottom].y > regions[&left].y);
    assert_eq!(regions[&right].height, 220.0);

    // The native renderer must paint a visible separator across both themes.
    {
        use iced_runtime::user_interface::{Cache, UserInterface};

        let mut ui = UserInterface::build(
            view(&h.app, h.workspace),
            Size::new(800.0, 500.0),
            Cache::new(),
            &mut h.renderer,
        );
        for theme in [iced::Theme::Light, iced::Theme::Dark] {
            ui.draw(
                &mut h.renderer,
                &theme,
                &iced::advanced::renderer::Style { text_color: theme.palette().text },
                iced::mouse::Cursor::Unavailable,
            );
            let pixels =
                h.renderer.screenshot(Size::new(800, 500), 1.0, theme.palette().background);
            let pixel = |x: usize| &pixels[4 * (460 * 800 + x)..][..4];
            assert_ne!(pixel(400), pixel(399), "pane separator in {theme:?}");
            assert_eq!(pixel(399), pixel(401), "single-pixel separator in {theme:?}");
        }
    }

    let bottom_id = h.app.terminal.active_id.unwrap();
    let (messages, _) = h.dispatch(
        Event::Window(iced::window::Event::RedrawRequested(std::time::Instant::now())),
        Point::new(120.0, 460.0),
    );
    for (id, pane) in [(first, left), (bottom_id, bottom)] {
        assert!(messages.iter().any(|message| matches!(
            message,
            Message::Terminal(TerminalMessage::Event(iced_term::Event::BackendCall(
                target, iced_term::BackendCommand::Resize(Some(size), _)
            ))) if *target == id && size.height == regions[&pane].height - 46.0
        )));
    }

    // The short bottom pane can still be split and the new tab can be closed.
    click_split_menu(&mut h, Point::new(382.0, 407.0), 3).await;
    assert_eq!(h.app.terminal.panes.grid.len(), 4);
    let small_pane_id = h.app.terminal.active_id.unwrap();
    assert_ne!(small_pane_id, bottom_id);
    h.message(TerminalMessage::Close(small_pane_id)).await;
    assert_eq!(h.app.terminal.panes.focused, bottom);

    let widgets: Vec<_> =
        h.app.terminal.tabs.iter().map(|tab| tab.term.widget_id().clone()).collect();
    h.workspace = true;
    h.click(Point::new(784.0, 305.0)).await;
    assert!(h.app.terminal.is_fullscreen);
    h.click(Point::new(784.0, 17.0)).await;
    assert!(!h.app.terminal.is_fullscreen);
    assert_eq!(h.app.terminal.panes.grid.len(), 3);
    assert!(h.app.terminal.tabs.iter().map(|tab| tab.term.widget_id()).eq(widgets.iter()));
    h.workspace = false;

    h.message(TerminalMessage::Close(bottom_id)).await;
    assert_eq!(h.app.terminal.panes.grid.len(), 2);
    assert_eq!(h.app.terminal.active_id, Some(first));

    h.message(TerminalMessage::Close(right_extra)).await;
    h.click(second_point).await;
    let subscription =
        h.app.terminal.tabs.iter().find(|tab| tab.id == second).unwrap().term.subscription();
    let mut events = into_recipes(subscription)
        .pop()
        .unwrap()
        .stream(Box::pin(iced::futures::stream::pending()));
    send_terminal_input(
        &mut h,
        second,
        character_key(
            "exit
",
        ),
        second_point,
        b"exit
",
    )
    .await;
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while h.app.terminal.tabs.iter().any(|tab| tab.id == second) {
            h.message(TerminalMessage::Event(events.next().await.expect("PTY event"))).await;
        }
    })
    .await
    .expect("shell exit should close its pane");
    drop(events);
    assert_eq!(h.app.terminal.panes.grid.len(), 1);
    assert_eq!(h.app.terminal.active_id, Some(first));
    assert_eq!(h.app.terminal.panes.grid.get(left).unwrap().tabs, [first, left_extra]);
    send_terminal_input(&mut h, first, character_key("y"), first_point, b"y").await;
    wait_for_line(&mut h, first, "FIRST:x中文y").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn terminal_drag_preserves_pty_identity_and_uses_new_grid_size() {
    use iced::advanced::input_method::Event as Ime;
    use iced::mouse::{Button, Event as Mouse};
    use iced::widget::pane_grid::Direction;

    let mut h = Harness::with_command(concat!(
        "stty -echo; printf 'READY\\n'; ",
        r#"while IFS= read -r line; do printf 'FIRST:%s ' "$line"; stty size; done"#,
    ))
    .await;
    let first = h.app.terminal.active_id.unwrap();
    let pane = h.app.terminal.panes.focused;
    let widget = h.app.terminal.tabs[0].term.widget_id().clone();
    h.app.terminal.split_terminal(pane, Direction::Right, None).unwrap();
    let second = h.app.terminal.active_id.unwrap();
    let first_point = Point::new(120.0, 360.0);
    let redraw = || Event::Window(iced::window::Event::RedrawRequested(std::time::Instant::now()));
    h.click(first_point).await;
    h.event(redraw(), first_point).await;
    wait_for_line(&mut h, first, "READY").await;
    send_terminal_input(&mut h, first, character_key("before\n"), first_point, b"before\n").await;
    async fn pty_size(h: &mut Harness, id: u64, prefix: &str) -> (u16, u16) {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                h.message(TerminalMessage::Event(iced_term::Event::BackendCall(
                    id,
                    iced_term::BackendCommand::Resize(None, None),
                )))
                .await;
                let tab = h.app.terminal.tabs.iter().find(|tab| tab.id == id).unwrap();
                if let Some(size) = tab.term.text().iter().find_map(|line| {
                    let (rows, cols) = line.strip_prefix(prefix)?.split_once(' ')?;
                    Some((rows.parse::<u16>().ok()?, cols.parse::<u16>().ok()?))
                }) {
                    break size;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("stty size from the original PTY")
    }
    async fn drag(h: &mut Harness, from: Point, to: Point) {
        h.event(Event::Mouse(Mouse::ButtonPressed(Button::Left)), from).await;
        h.event(Event::Mouse(Mouse::CursorMoved { position: to }), to).await;
        h.event(Event::Mouse(Mouse::ButtonReleased(Button::Left)), to).await;
    }
    let before = pty_size(&mut h, first, "FIRST:before ").await;
    drag(&mut h, Point::new(400.0, 390.0), Point::new(550.0, 390.0)).await;
    drag(&mut h, Point::new(10.0, 297.0), Point::new(675.0, 390.0)).await;
    let moved = Point::new(675.0, 390.0);
    h.event(redraw(), moved).await;
    assert_eq!(h.app.terminal.active_id, Some(first));
    assert_eq!(h.app.terminal.tabs[0].term.widget_id(), &widget);
    send_terminal_input(
        &mut h,
        first,
        Event::InputMethod(Ime::Commit("中文".into())),
        moved,
        "中文".as_bytes(),
    )
    .await;
    send_terminal_input(&mut h, first, character_key("\n"), moved, b"\n").await;
    let after = pty_size(&mut h, first, "FIRST:中文 ").await;
    eprintln!("terminal {first} {widget:?}: stty size {before:?} -> {after:?}");
    assert_eq!(before.0, after.0);
    assert!(after.1 < before.1);
    assert!(
        h.app
            .terminal
            .tabs
            .iter()
            .find(|tab| tab.id == second)
            .unwrap()
            .term
            .text()
            .iter()
            .all(|line| !line.contains("FIRST:"))
    );
}
