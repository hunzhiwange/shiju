//! Host configuration and project lifecycle contract (no rendering fixture).
use crate::app::message::{TerminalMessage, ViewMessage};
use crate::app::{App, Message};
struct Harness {
    app: App,
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
        Self { app }
    }
    async fn message(&mut self, message: TerminalMessage) {
        let _ = super::update(&mut self.app, message);
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        let ids: Vec<_> = self.app.terminal.tabs.iter().map(|tab| tab.id).collect();
        for id in ids {
            self.app.terminal.close_terminal(id);
        }
        self.app.terminal.reap_retired();
    }
}
use iced::Point;
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

/// Keep the real workspace assembly boundary, while widget algorithms live in the library.
#[tokio::test(flavor = "multi_thread")]
async fn workspace_terminal_toolbar_routes_fullscreen_and_preserves_widget() {
    use iced::advanced::renderer::Headless;
    use iced::{Font, Pixels, Size};
    use iced_runtime::user_interface::{Cache, UserInterface};
    let mut h = Harness::new().await;
    let widget = h.app.terminal.tabs[0].term.widget_id().clone();
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("renderer");
    let mut cache = Cache::new();
    for (fullscreen, y) in [(true, 305.0), (false, 17.0)] {
        let mut messages = Vec::new();
        {
            let mut ui = UserInterface::build(
                crate::app::views::project::view(&h.app),
                Size::new(800.0, 500.0),
                cache,
                &mut renderer,
            );
            let _ = ui.update(
                &[
                    iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                        iced::mouse::Button::Left,
                    )),
                    iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                        iced::mouse::Button::Left,
                    )),
                ],
                iced::mouse::Cursor::Available(Point::new(784.0, y)),
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
            cache = ui.into_cache();
        }
        assert!(messages.iter().any(|message| matches!(
            message,
            Message::Terminal(TerminalMessage::ToggleFullscreen)
        )));
        for message in messages {
            let _ = h.app.update(message);
        }
        assert_eq!(h.app.terminal.is_fullscreen, fullscreen);
        assert_eq!(h.app.terminal.tabs[0].term.widget_id(), &widget);
    }
}

#[test]
fn terminal_chat_append_effect_preserves_existing_draft() {
    let mut app = App::new().0;
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("现有草稿");
    let selection = "终端输出 READY";
    let _ = crate::app::message::chat::update(
        &mut app,
        crate::app::message::ChatMessage::AppendText(selection.into()),
    );
    assert_eq!(app.current_session_runtime().input_editor.text(), format!("现有草稿\n{selection}"));
}
