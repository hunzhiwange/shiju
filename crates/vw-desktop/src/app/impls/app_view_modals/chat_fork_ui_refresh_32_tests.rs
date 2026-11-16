//! Exercise the session dialogs through Iced's headless layout, event and draw paths.
#![cfg(not(target_arch = "wasm32"))]

use super::{App, ForkSessionTarget, Message, with_chat_fork_dialog};
use crate::app::i18n::I18n;
use crate::app::message::{ChatMessage, ProjectMessage};
use crate::app::models::{ChatMessage as TranscriptMessage, ChatRole};
use config_ui::{DesktopLocale, GatewayClientSystemSettingsConfig};
use iced::advanced::{
    Layout, clipboard, layout,
    renderer::{Headless, Style},
    widget::{Id, Operation, Tree, operation},
};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::{button, container};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_32_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let home = tempfile::tempdir().unwrap();
    let config = GatewayClientSystemSettingsConfig {
        host: "[".into(),
        servers: Vec::new(),
        ..Default::default()
    };
    std::fs::write(
        home.path().join("gateway-client-bootstrap.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("{}::{test}", module_path!().split_once("::").unwrap().1)])
        .env(CHILD, "1")
        .env("HOME", home.path())
        .env("VIBEWINDOW_CONFIG_DIR", home.path())
        .env("VIBEWINDOW_WORKSPACE", home.path())
        .current_dir(home.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed; 0 failed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    false
}

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    App::new().0
}

async fn renderer() -> Renderer {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .unwrap()
}

fn root() -> Element<'static, Message> {
    container(button("underlay").on_press(Message::CloseError))
        .width(iced::Fill)
        .height(iced::Fill)
        .into()
}

fn scene<'a>(view: Element<'a, Message>, renderer: &mut Renderer, size: Size<u32>) -> Ui<'a> {
    UserInterface::build(
        view,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    )
}

fn send(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    cursor: mouse::Cursor,
    events: &[Event],
) -> Vec<Message> {
    let mut messages = Vec::new();
    ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
    messages
}

fn click(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Vec<Message> {
    send(
        ui,
        renderer,
        mouse::Cursor::Available(point),
        &[
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
    )
}

fn key(ui: &mut Ui<'_>, renderer: &mut Renderer, key: Key, text: Option<&str>) -> Vec<Message> {
    send(
        ui,
        renderer,
        mouse::Cursor::Unavailable,
        &[Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: text.map(Into::into),
            repeat: false,
        })],
    )
}

fn capture(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
    cursor: mouse::Cursor,
) -> Vec<u8> {
    assert!(
        send(
            ui,
            renderer,
            cursor,
            &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
        )
        .is_empty()
    );
    ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_32_SCREENSHOTS") {
        ::image::save_buffer(
            std::path::PathBuf::from(directory).join(format!("{name}.png")),
            &pixels,
            size.width,
            size.height,
            ::image::ColorType::Rgba8,
        )
        .unwrap();
    }
    pixels
}

#[derive(Default)]
struct Geometry {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    scroll: Vec<(Rectangle, iced::Vector)>,
}

impl Operation for Geometry {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, value: &str) {
        self.labels.push((value.into(), bounds));
    }

    fn text_input(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        state: &mut dyn operation::TextInput,
    ) {
        self.inputs.push((state.text().into(), bounds));
    }

    fn scrollable(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        translation: iced::Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        self.scroll.push((bounds, translation));
    }
}

fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
    let mut geometry = Geometry::default();
    ui.operate(renderer, &mut geometry);
    geometry
}

fn label(geometry: &Geometry, value: &str) -> Rectangle {
    geometry
        .labels
        .iter()
        .find(|(label, _)| label == value)
        .unwrap_or_else(|| panic!("missing {value}: {:?}", geometry.labels))
        .1
}

fn find_layout<'a>(
    layout: Layout<'a>,
    matches: impl Fn(Rectangle) -> bool + Copy,
) -> Option<Layout<'a>> {
    if matches(layout.bounds()) {
        return Some(layout);
    }
    layout.children().find_map(|child| find_layout(child, matches))
}

fn prepare_chat(app: &mut App) {
    app.session_state.active_session_id = Some("source".into());
    app.navigation_tabs.project_path = Some("/fixture/source".into());
    app.workspace_state.project_is_git_repo = true;
    app.session_state.sessions = vec![session_info("source", "/fixture/source")];
    app.chat_view.chat = [ChatRole::User, ChatRole::Tool, ChatRole::User]
        .into_iter()
        .map(|role| TranscriptMessage {
            role,
            content: "虚构消息・測試訊息・Fixture".into(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        })
        .collect();
    app.chat_view.chat_message_ids =
        vec![Some("first-turn".into()), Some("tool-row".into()), Some("second-turn".into())];
    let anchor = crate::app::message::chat::fork_session_anchor_from_user_row(app, 2);
    let _ = crate::app::message::chat::update(app, ChatMessage::OpenForkSessionDialog(anchor));
    app.session_state.chat_reset_menu_idx = Some(1);
}

fn session_info(id: &str, directory: &str) -> session_info::Info {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "slug": id,
        "projectID": "project-fixture",
        "directory": directory,
        "title": id,
        "version": "0.0.0",
        "time": {"created": 1, "updated": 2},
    }))
    .unwrap()
}

fn click_user_fork(app: &mut App, renderer: &mut Renderer, row: usize) -> ChatMessage {
    use crate::app::components::chat_panel::message_view::message_view;
    use crate::app::models::ChatRenderCacheEntry;

    let cache = ChatRenderCacheEntry::default();
    let build = || {
        let message = &app.chat_view.chat[row];
        message_view(app, row, message.role, &message.content, &[], None, None, &cache, false)
    };
    let mut element = build();
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, Size::new(420.0, 480.0));
    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    // The fork action is the first of the three compact 13px footer buttons.
    let point =
        find_layout(Layout::new(&node), |bounds| bounds.width == 13.0 && bounds.height == 13.0)
            .expect("fork action in the user message footer")
            .bounds()
            .center();
    let mut ui = scene(build(), renderer, VIEW_SIZE);
    let mut messages = click(&mut ui, renderer, point);
    assert_eq!(messages.len(), 1, "{messages:?}");
    let Message::Chat(message @ ChatMessage::OpenForkSessionDialog(_)) = messages.remove(0) else {
        panic!("user row did not open the fork dialog");
    };
    message
}

fn click_fork_target(app: &App, renderer: &mut Renderer, target: ForkSessionTarget) -> ChatMessage {
    let mut ui = scene(with_chat_fork_dialog(app, root()), renderer, VIEW_SIZE);
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let title = match target {
        ForkSessionTarget::Local => "在当前目录中打开新会话",
        ForkSessionTarget::NewWorktree => "分叉到新工作树",
    };
    let point = label(&geometry(&mut ui, renderer), i18n.__(title)).center();
    let mut messages = click(&mut ui, renderer, point);
    assert_eq!(messages.len(), 1);
    let Message::Chat(message @ ChatMessage::ForkSessionAt { .. }) = messages.remove(0) else {
        panic!("fork dialog did not emit a confirmation");
    };
    message
}

async fn next_output(task: iced::Task<Message>, matches: impl Fn(&Message) -> bool) -> Message {
    use iced_runtime::futures::futures::StreamExt;

    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let mut stream = iced_runtime::task::into_stream(task).expect("expected a task");
        while let Some(action) = stream.next().await {
            if let iced_runtime::Action::Output(message) = action
                && matches(&message)
            {
                return message;
            }
        }
        panic!("task finished without producing the expected message");
    })
    .await
    .expect("gateway fixture timed out")
}

// Same first turn and tool block as the gateway fork acceptance fixture.
const FORK_USER: &str = "fork-retained-user";
const FORK_ANSWER: &str = "fork-retained-answer";
const FORK_SECOND: &str = "fork-discarded-second";
const FORK_THIRD: &str = "fork-discarded-third";

fn fork_tool_block() -> String {
    let tool = serde_json::json!({
        "tool_call_id": "fork-retained-call",
        "status": "completed",
        "input": r#"{"path":"tracked.txt"}"#,
        "output": "fork-retained-result",
    });
    format!("tool file_read\n{tool}\n{FORK_ANSWER}")
}

fn assert_projected_fork(app: &App) {
    let [user, assistant] = app.chat_view.chat.as_slice() else {
        panic!("expected only the retained user and assistant");
    };
    assert_eq!(user.role, ChatRole::User);
    assert_eq!(user.content, FORK_USER);
    assert_eq!(assistant.role, ChatRole::Assistant);
    assert_eq!(assistant.content, fork_tool_block());
    assert_eq!(
        app.chat_view.chat_message_ids,
        [Some("retained-user".into()), Some("retained-assistant".into())]
    );
}

#[derive(Debug)]
struct FixtureRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

async fn serve_fork_gateway(
    listener: tokio::net::TcpListener,
    branch_id: &'static str,
    branch_directory: &'static str,
    reject: bool,
    requests: tokio::sync::mpsc::UnboundedSender<FixtureRequest>,
) {
    use serde_json::json;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut reader = BufReader::new(&mut socket);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        let mut request_line = line.split_whitespace();
        let method = request_line.next().unwrap().to_owned();
        let path = urlencoding::decode(request_line.next().unwrap()).unwrap().into_owned();
        let mut length = 0;
        loop {
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                length = value.trim().parse().unwrap();
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).await.unwrap();
        let route = path.split('?').next().unwrap();
        let (status, response) = match (method.as_str(), route) {
            ("POST", "/v1/session/source/fork") if reject => {
                ("400 Bad Request", json!({"error": "anchor was removed"}))
            }
            ("POST", "/v1/session/source/fork") => {
                ("200 OK", serde_json::to_value(session_info(branch_id, branch_directory)).unwrap())
            }
            ("GET", route) if route == format!("/v1/session/{branch_id}") => {
                let mut info = session_info(branch_id, branch_directory);
                info.fork_execution_started = Some(false);
                ("200 OK", serde_json::to_value(info).unwrap())
            }
            ("GET", "/v1/projects/project-fixture") => (
                "200 OK",
                json!({"project": {
                    "id": "project-fixture", "name": "Source", "directory": "/fixture/source",
                    "display_path": "/fixture/source", "status": "ready", "created_at_ms": 1,
                    "updated_at_ms": 2, "git": {"is_repo": true, "has_uncommitted_changes": false}
                }}),
            ),
            ("GET", "/v1/session/first/ui/summary") => (
                "200 OK",
                json!({"id": "first", "title": "Fork", "updated_ms": 2, "message_count": 0, "call_count": 0}),
            ),
            ("GET", "/v1/session/first/ui/messages") => {
                ("200 OK", json!({"items": [], "total_count": 0}))
            }
            ("GET", route) if route == format!("/v1/session/{branch_id}/ui/summary") => (
                "200 OK",
                json!({
                    "id": branch_id, "title": "Fork", "updated_ms": 2,
                    "message_count": 2, "call_count": 0
                }),
            ),
            ("GET", route) if route == format!("/v1/session/{branch_id}/ui/messages") => (
                "200 OK",
                json!({
                    "items": [
                        {"seq_no": 1, "message_id": "retained-user", "message": {
                            "role": "User", "content": FORK_USER
                        }},
                        {"seq_no": 2, "message_id": "retained-assistant", "message": {
                            "role": "Assistant", "content": fork_tool_block()
                        }}
                    ],
                    "total_count": 2
                }),
            ),
            _ => ("404 Not Found", json!({"error": "unexpected fixture request"})),
        };
        // Normal worktree opening also reads optional project preferences.
        if method != "GET" || route != "/v1/desktop/project-preferences" {
            requests.send(FixtureRequest { method, path, body }).unwrap();
        }
        let response = response.to_string();
        socket
            .write_all(
                format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
                    response.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fork_desktop_clicks_send_frozen_turn_and_open_gateway_history() {
    use crate::app::message::{chat, project};
    use serde_json::json;

    if !isolated("fork_desktop_clicks_send_frozen_turn_and_open_gateway_history") {
        return;
    }

    let mut app = fixture();
    let mut renderer = renderer().await;
    for (target, id, directory, target_body, reject, switch_while_pending) in [
        (
            ForkSessionTarget::Local,
            "first",
            "/fixture/source",
            json!({"type": "sameDirectory"}),
            false,
            false,
        ),
        (
            ForkSessionTarget::Local,
            "local",
            "/fixture/source",
            json!({"type": "sameDirectory"}),
            false,
            false,
        ),
        (
            ForkSessionTarget::NewWorktree,
            "worktree",
            "/fixture/worktree",
            json!({"type": "worktree"}),
            false,
            false,
        ),
        (
            ForkSessionTarget::Local,
            "rejected",
            "/fixture/source",
            json!({"type": "sameDirectory"}),
            true,
            false,
        ),
        (
            ForkSessionTarget::Local,
            "late",
            "/fixture/source",
            json!({"type": "sameDirectory"}),
            false,
            true,
        ),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = GatewayClientSystemSettingsConfig {
            host: "127.0.0.1".into(),
            port: listener.local_addr().unwrap().port(),
            servers: Vec::new(),
            ..Default::default()
        };
        std::fs::write("gateway-client-bootstrap.json", serde_json::to_vec(&config).unwrap())
            .unwrap();
        let (tx, mut requests) = tokio::sync::mpsc::unbounded_channel();
        let server = tokio::spawn(serve_fork_gateway(listener, id, directory, reject, tx));
        prepare_chat(&mut app);
        app.session_state.chat_fork_dialog = None;
        app.current_session_runtime_mut().input_editor = iced::widget::text_editor::Content::new();
        app.chat_view.chat = [
            (ChatRole::User, FORK_USER.to_owned()),
            (ChatRole::Assistant, fork_tool_block()),
            (ChatRole::User, FORK_SECOND.to_owned()),
            (ChatRole::Assistant, format!("{FORK_SECOND}-answer")),
            (ChatRole::User, FORK_THIRD.to_owned()),
            (ChatRole::Assistant, format!("{FORK_THIRD}-answer")),
        ]
        .into_iter()
        .map(|(role, content)| TranscriptMessage {
            role,
            content,
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        })
        .collect();
        app.chat_view.chat_message_ids = [
            "retained-user",
            "retained-assistant",
            "second-turn",
            "second-assistant",
            "third-turn",
            "third-assistant",
        ]
        .map(|id| Some(id.into()))
        .to_vec();

        // A missing user ID must not trigger any writes or borrow the tool row's ID.
        app.chat_view.chat_message_ids[2] = None;
        let open = click_user_fork(&mut app, &mut renderer, 2);
        let _ = chat::update(&mut app, open);
        assert!(app.session_state.chat_fork_dialog.is_none());
        assert!(requests.try_recv().is_err());
        app.chat_view.chat_message_ids[2] = Some("second-turn".into());

        let (row, turn_id, prompt) = if id == "first" {
            (0, "retained-user", FORK_USER)
        } else {
            (2, "second-turn", FORK_SECOND)
        };
        let open = click_user_fork(&mut app, &mut renderer, row);
        let _ = chat::update(&mut app, open);
        assert_eq!(app.session_state.chat_fork_dialog.as_ref().unwrap().anchor.user_message_id, turn_id);

        // Paging can change the rows while the modal is open.
        app.chat_view.chat.remove(0);
        app.chat_view.chat_message_ids.remove(0);
        let confirm = click_fork_target(&app, &mut renderer, target);
        let task = chat::update(&mut app, confirm.clone());
        assert_eq!(app.session_state.active_session_id.as_deref(), Some("source"));
        assert!(iced_runtime::task::into_stream(chat::update(&mut app, confirm)).is_none());
        if id == "first" {
            let mut progress = scene(with_chat_fork_dialog(&app, root()), &mut renderer, VIEW_SIZE);
            let bounds = geometry(&mut progress, &renderer);
            let i18n = I18n::new(app.general_settings.desktop_locale);
            label(&bounds, i18n.__("正在分叉会话…"));
            assert!(key(&mut progress, &mut renderer, Key::Named(Named::Escape), None).is_empty());
            for (theme, mode) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
                capture(
                    &mut progress,
                    &mut renderer,
                    &theme,
                    VIEW_SIZE,
                    &format!("fork-pending-{mode}"),
                    mouse::Cursor::Unavailable,
                );
            }
        }
        let Message::Chat(finished @ ChatMessage::ForkSessionFinished { .. }) =
            next_output(task, |_| true).await
        else {
            panic!("expected fork completion");
        };
        let post = requests.try_recv().unwrap();
        assert_eq!(post.method, "POST");
        assert_eq!(post.path, "/v1/session/source/fork?directory=/fixture/source");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&post.body).unwrap(),
            json!({"beforeTurnId": turn_id, "target": target_body})
        );

        if !reject {
            let project = requests.try_recv().unwrap();
            assert_eq!(project.method, "GET");
            assert_eq!(project.path, "/v1/projects/project-fixture");
            assert_fork_readback(&mut requests, id, directory);
        }
        assert!(requests.try_recv().is_err());
        if switch_while_pending {
            app.session_state.active_session_id = Some("other".into());
        }
        let _ = chat::update(&mut app, finished);
        assert!(app.session_state.chat_fork_dialog.is_none());

        if switch_while_pending {
            assert_eq!(app.session_state.active_session_id.as_deref(), Some("other"));
        } else if reject {
            assert_eq!(app.session_state.active_session_id.as_deref(), Some("source"));
            assert_eq!(app.chat_view.chat_message_ids[1].as_deref(), Some("second-turn"));
            assert_eq!(
                app.toast_state.active_toast.as_ref().unwrap().kind,
                crate::app::state::ToastKind::Error
            );
        } else {
            assert_eq!(app.session_state.active_session_id.as_deref(), Some(id));
            assert_eq!(app.navigation_tabs.project_path.as_deref(), Some("/fixture/source"));
            assert_eq!(app.known_session_directory(id).as_deref(), Some(directory));
            assert!(app.session_state.sessions.iter().any(|info| info.id == id));
            assert!(app.workspace_state.project_sessions["/fixture/source"].iter().any(|info| info.id == id));
            if target == ForkSessionTarget::NewWorktree {
                assert!(!app.workspace_state.recent_projects.iter().any(|path| path == directory));
                assert!(!app.workspace_state.project_sessions.contains_key(directory));
            }
            assert_eq!(app.current_session_runtime().input_editor.text(), prompt);
            if id == "first" {
                assert!(app.chat_view.chat.is_empty());
            } else {
                assert_projected_fork(&app);
            }
            assert_eq!(
                app.toast_state.active_toast.as_ref().unwrap().kind,
                crate::app::state::ToastKind::Success
            );

            app.session_state.active_session_id = None;
            app.chat_view.chat.clear();
            app.session_state.session_view_cache.remove(id);
            app.session_state.session_chat_cache.remove(id);
            app.session_state.session_chat_message_id_cache.remove(id);
            let task = project::update(
                &mut app,
                ProjectMessage::OpenProjectSessionPressed(directory.into(), id.into()),
            );
            let Message::Project(loaded) = next_output(task, |message| {
                matches!(message, Message::Project(ProjectMessage::SessionOpenPrepared { .. }))
            })
            .await
            else {
                panic!("expected normal session readback");
            };
            let _ = project::update(&mut app, loaded);
            assert_fork_readback(&mut requests, id, directory);
            assert_eq!(app.navigation_tabs.project_path.as_deref(), Some("/fixture/source"));
            if id == "first" {
                assert!(app.chat_view.chat.is_empty());
            } else {
                assert_projected_fork(&app);
            }
        }
        server.abort();
        let _ = server.await;
    }
}

fn assert_fork_readback(
    requests: &mut tokio::sync::mpsc::UnboundedReceiver<FixtureRequest>,
    id: &str,
    directory: &str,
) {
    let mut endpoints = Vec::new();
    for _ in 0..2 {
        let request = requests.try_recv().unwrap();
        assert_eq!(request.method, "GET");
        assert!(request.path.contains(&format!("directory={directory}")), "{request:?}");
        endpoints.push(request.path.split('?').next().unwrap().to_owned());
    }
    endpoints.sort_unstable();
    assert_eq!(
        endpoints,
        [format!("/v1/session/{id}/ui/messages"), format!("/v1/session/{id}/ui/summary")]
    );
}

const VIEW_SIZE: Size<u32> = Size::new(420, 480);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_32_invalid_file_name_feedback_keeps_inline_editor() {
    if !isolated("ui_refresh_32_invalid_file_name_feedback_keeps_inline_editor") {
        return;
    }
    let mut app = fixture();
    app.general_settings.desktop_locale = DesktopLocale::English;
    app.file_tree_interaction.file_tree_rename_path = Some("/fixture/original.txt".into());
    app.file_tree_interaction.file_tree_rename_value = "../wrong".into();
    let mut renderer = renderer().await;
    app.navigation_tabs.project_path = Some("/fixture".into());
    app.workspace_state.file_index_cache.insert("/fixture".into(), vec!["/fixture/original.txt".into()]);
    let mut ui = scene(crate::app::components::file_tree::view(&app), &mut renderer, VIEW_SIZE);
    let input = geometry(&mut ui, &renderer).inputs[0].1;
    assert!(click(&mut ui, &mut renderer, input.center()).is_empty());
    let messages = key(&mut ui, &mut renderer, Key::Named(Named::Enter), None);
    drop(ui);
    let [message @ Message::Project(ProjectMessage::FileTreeRenameSave)] = messages.as_slice()
    else {
        panic!("expected exactly one save message");
    };
    let _ = app.update(message.clone());
    assert_eq!(
        app.file_tree_interaction.file_tree_rename_path.as_deref(),
        Some("/fixture/original.txt")
    );
    assert_eq!(app.file_tree_interaction.file_tree_rename_value, "../wrong");
    let toast = app.toast_state.active_toast.as_ref().expect("invalid file name must be visible");
    assert!(matches!(toast.kind, crate::app::state::ToastKind::Error));
    let theme = Theme::Dark;
    let mut ui = scene(app.view(), &mut renderer, VIEW_SIZE);
    capture(
        &mut ui,
        &mut renderer,
        &theme,
        VIEW_SIZE,
        "en-dark-file-error",
        mouse::Cursor::Unavailable,
    );
    assert!(
        geometry(&mut ui, &renderer)
            .labels
            .iter()
            .any(|(value, _)| value.contains("Invalid file name"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn fork_selection_reopen_gateway_reads_worktree_authority() {
    use crate::app::message::project;
    if !isolated("fork_selection_reopen_gateway_reads_worktree_authority") {
        return;
    }
    let mut app = fixture();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = GatewayClientSystemSettingsConfig {
        host: "127.0.0.1".into(),
        port: listener.local_addr().unwrap().port(),
        servers: Vec::new(),
        ..Default::default()
    };
    std::fs::write("gateway-client-bootstrap.json", serde_json::to_vec(&config).unwrap()).unwrap();
    let (tx, mut requests) = tokio::sync::mpsc::unbounded_channel();
    let server =
        tokio::spawn(serve_fork_gateway(listener, "reopen", "/fixture/worktree", false, tx));
    app.navigation_tabs.project_path = Some("/fixture/source".into());
    app.session_state.active_session_id = Some("source".into());
    let mut cached = session_info("reopen", "/fixture/worktree");
    cached.fork_execution_started = Some(false);
    app.session_state.sessions = vec![cached];
    app.get_session_runtime_mut("reopen").input_editor =
        iced::widget::text_editor::Content::with_text("未发送草稿");
    let task = project::update(
        &mut app,
        ProjectMessage::OpenProjectSessionPressed("/fixture/source".into(), "reopen".into()),
    );
    let resolved = next_output(task, |message| {
        matches!(message, Message::Project(ProjectMessage::SessionNavigationResolved { .. }))
    })
    .await;
    let request = requests.try_recv().unwrap();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/session/reopen?directory=/fixture/worktree");
    let Message::Project(resolved) = resolved else { unreachable!() };
    let _ = project::update(&mut app, resolved);
    assert_eq!(app.session_state.active_session_id.as_deref(), Some("reopen"));
    assert!(!app.current_session_has_started());
    assert_eq!(app.current_session_runtime().input_editor.text(), "未发送草稿");
    server.abort();
}
