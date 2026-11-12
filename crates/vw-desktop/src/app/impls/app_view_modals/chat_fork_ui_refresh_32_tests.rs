//! Exercise the session dialogs through Iced's headless layout, event and draw paths.
#![cfg(not(target_arch = "wasm32"))]

use super::{App, ForkSessionTarget, Message, with_chat_fork_dialog};
use crate::app::i18n::I18n;
use crate::app::impls::app_view_modals::{
    with_chat_reset_dialog, with_file_tree_delete_confirm, with_file_tree_rename,
    with_session_rename,
};
use crate::app::message::{ChatMessage, ProjectMessage, chat::ForkSessionAnchor};
use crate::app::models::{ChatMessage as TranscriptMessage, ChatRole};
use iced::advanced::{
    Layout, clipboard, layout,
    renderer::{Headless, Style},
    widget::{Id, Operation, Tree, operation},
};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::{button, container};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

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

fn assert_fits(geometry: &Geometry, size: Size<u32>) {
    for (value, bounds) in geometry.labels.iter().chain(&geometry.inputs) {
        assert!(bounds.width > 0.0 && bounds.height > 0.0, "{value}: {bounds:?}");
        assert!(bounds.x >= 0.0 && bounds.y >= 0.0, "{value}: {bounds:?}");
        assert!(bounds.x + bounds.width <= size.width as f32 + 0.1, "{value}: {bounds:?}");
        assert!(bounds.y + bounds.height <= size.height as f32 + 0.1, "{value}: {bounds:?}");
    }
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

fn assert_control_alignment(
    mut view: Element<'_, Message>,
    renderer: &Renderer,
    size: Size<u32>,
    input: Option<Rectangle>,
) -> Point {
    let mut tree = Tree::new(view.as_widget());
    let limits = layout::Limits::new(Size::ZERO, Size::new(size.width as f32, size.height as f32));
    let node = view.as_widget_mut().layout(&mut tree, renderer, &limits);
    let layout = Layout::new(&node);
    let close = find_layout(layout, |bounds| bounds.width == 28.0 && bounds.height == 28.0)
        .expect("close button");
    let icon = find_layout(close, |bounds| bounds.width == 12.0 && bounds.height == 12.0)
        .expect("close icon");
    assert!((close.bounds().center_y() - icon.bounds().center_y()).abs() < 0.5);
    if let Some(input) = input {
        let field = find_layout(layout, |bounds| bounds == input).expect("input field");
        let glyphs = field.children().next().expect("input text");
        assert!((glyphs.bounds().center_y() - input.center_y()).abs() < 0.5);
    }
    close.bounds().center()
}

fn prepare_chat(app: &mut App) {
    app.active_session_id = Some("source".into());
    app.project_path = Some("/fixture/source".into());
    app.sessions = vec![session_info("source", "/fixture/source")];
    app.chat = [ChatRole::User, ChatRole::Tool, ChatRole::User]
        .into_iter()
        .map(|role| TranscriptMessage {
            role,
            content: "虚构消息・測試訊息・Fixture".into(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        })
        .collect();
    app.chat_message_ids =
        vec![Some("first-turn".into()), Some("tool-row".into()), Some("second-turn".into())];
    let anchor = ForkSessionAnchor::from_user_row(app, 2);
    let _ = crate::app::message::chat::update(app, ChatMessage::OpenForkSessionDialog(anchor));
    app.chat_reset_menu_idx = Some(1);
}

fn session_info(id: &str, directory: &str) -> vw_shared::session::info::Info {
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
        let message = &app.chat[row];
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
    let i18n = I18n::new(app.desktop_locale);
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
    let [user, assistant] = app.chat.as_slice() else {
        panic!("expected only the retained user and assistant");
    };
    assert_eq!(user.role, ChatRole::User);
    assert_eq!(user.content, FORK_USER);
    assert_eq!(assistant.role, ChatRole::Assistant);
    assert_eq!(assistant.content, fork_tool_block());
    assert_eq!(
        app.chat_message_ids,
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
        app.chat_fork_dialog = None;
        app.current_session_runtime_mut().input_editor = iced::widget::text_editor::Content::new();
        app.chat = [
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
        app.chat_message_ids = [
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
        app.chat_message_ids[2] = None;
        let open = click_user_fork(&mut app, &mut renderer, 2);
        let _ = chat::update(&mut app, open);
        assert!(app.chat_fork_dialog.is_none());
        assert!(requests.try_recv().is_err());
        app.chat_message_ids[2] = Some("second-turn".into());

        let (row, turn_id, prompt) = if id == "first" {
            (0, "retained-user", FORK_USER)
        } else {
            (2, "second-turn", FORK_SECOND)
        };
        let open = click_user_fork(&mut app, &mut renderer, row);
        let _ = chat::update(&mut app, open);
        assert_eq!(app.chat_fork_dialog.as_ref().unwrap().anchor.user_message_id, turn_id);

        // Paging can change the rows while the modal is open.
        app.chat.remove(0);
        app.chat_message_ids.remove(0);
        let confirm = click_fork_target(&app, &mut renderer, target);
        let task = chat::update(&mut app, confirm.clone());
        assert_eq!(app.active_session_id.as_deref(), Some("source"));
        assert!(iced_runtime::task::into_stream(chat::update(&mut app, confirm)).is_none());
        if id == "first" {
            let mut progress = scene(with_chat_fork_dialog(&app, root()), &mut renderer, VIEW_SIZE);
            let bounds = geometry(&mut progress, &renderer);
            let i18n = I18n::new(app.desktop_locale);
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
            app.active_session_id = Some("other".into());
        }
        let _ = chat::update(&mut app, finished);
        assert!(app.chat_fork_dialog.is_none());

        if switch_while_pending {
            assert_eq!(app.active_session_id.as_deref(), Some("other"));
        } else if reject {
            assert_eq!(app.active_session_id.as_deref(), Some("source"));
            assert_eq!(app.chat_message_ids[1].as_deref(), Some("second-turn"));
            assert_eq!(
                app.active_toast.as_ref().unwrap().kind,
                crate::app::state::ToastKind::Error
            );
        } else {
            assert_eq!(app.active_session_id.as_deref(), Some(id));
            assert_eq!(app.project_path.as_deref(), Some("/fixture/source"));
            assert_eq!(app.known_session_directory(id).as_deref(), Some(directory));
            assert!(app.sessions.iter().any(|info| info.id == id));
            assert!(app.project_sessions["/fixture/source"].iter().any(|info| info.id == id));
            if target == ForkSessionTarget::NewWorktree {
                assert!(!app.recent_projects.iter().any(|path| path == directory));
                assert!(!app.project_sessions.contains_key(directory));
            }
            assert_eq!(app.current_session_runtime().input_editor.text(), prompt);
            if id == "first" {
                assert!(app.chat.is_empty());
            } else {
                assert_projected_fork(&app);
            }
            assert_eq!(
                app.active_toast.as_ref().unwrap().kind,
                crate::app::state::ToastKind::Success
            );

            app.active_session_id = None;
            app.chat.clear();
            app.session_view_cache.remove(id);
            app.session_chat_cache.remove(id);
            app.session_chat_message_id_cache.remove(id);
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
            assert_eq!(app.project_path.as_deref(), Some("/fixture/source"));
            if id == "first" {
                assert!(app.chat.is_empty());
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

const DRAFT: &str = "新草稿・新檔案・draft";
const VIEW_SIZE: Size<u32> = Size::new(420, 480);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fork_desktop_layout_and_interaction() {
    if !isolated("fork_desktop_layout_and_interaction") {
        return;
    }
    let mut app = fixture();
    prepare_chat(&mut app);
    let mut renderer = renderer().await;

    for (locale, size, language) in [
        (DesktopLocale::SimplifiedChinese, VIEW_SIZE, "zh-cn"),
        (DesktopLocale::TraditionalChinese, Size::new(640, 540), "zh-tw"),
        (DesktopLocale::English, Size::new(960, 600), "en"),
    ] {
        app.desktop_locale = locale;
        let i18n = I18n::new(locale);
        for (theme, mode) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
            let mut fork = scene(with_chat_fork_dialog(&app, root()), &mut renderer, size);
            let bounds = geometry(&mut fork, &renderer);
            assert_fits(&bounds, size);
            let local = label(&bounds, i18n.__("在当前目录中打开新会话")).center();
            let worktree = label(&bounds, i18n.__("分叉到新工作树")).center();
            let idle = capture(
                &mut fork,
                &mut renderer,
                &theme,
                size,
                &format!("{language}-{mode}-fork"),
                mouse::Cursor::Unavailable,
            );
            for (point, target) in [(local, "local"), (worktree, "worktree")] {
                let cursor = mouse::Cursor::Available(point);
                assert!(
                    send(
                        &mut fork,
                        &mut renderer,
                        cursor,
                        &[
                            Event::Mouse(mouse::Event::CursorMoved { position: point }),
                            Event::Mouse(mouse::Event::WheelScrolled {
                                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 },
                            }),
                        ]
                    )
                    .is_empty()
                );
                assert!(
                    capture(
                        &mut fork,
                        &mut renderer,
                        &theme,
                        size,
                        &format!("{language}-{mode}-fork-{target}"),
                        cursor,
                    ) != idle
                );
            }
            assert!(
                send(
                    &mut fork,
                    &mut renderer,
                    mouse::Cursor::Unavailable,
                    &[Event::Mouse(mouse::Event::CursorLeft),]
                )
                .is_empty()
            );
            assert!(
                capture(
                    &mut fork,
                    &mut renderer,
                    &theme,
                    size,
                    &format!("{language}-{mode}-fork-idle"),
                    mouse::Cursor::Unavailable,
                ) == idle
            );
            assert!(click(&mut fork, &mut renderer, Point::new(5.0, 5.0)).is_empty());
            assert!(key(&mut fork, &mut renderer, Key::Named(Named::Enter), None).is_empty());
            assert!(matches!(
                click(&mut fork, &mut renderer, local).as_slice(),
                [Message::Chat(ChatMessage::ForkSessionAt {
                    anchor,
                    target: ForkSessionTarget::Local
                })] if anchor.user_message_id == "second-turn"
            ));
            assert!(matches!(
                click(&mut fork, &mut renderer, worktree).as_slice(),
                [Message::Chat(ChatMessage::ForkSessionAt {
                    anchor,
                    target: ForkSessionTarget::NewWorktree
                })] if anchor.user_message_id == "second-turn"
            ));
            let cancel = label(&bounds, i18n.__("取消")).center();
            assert!(matches!(
                click(&mut fork, &mut renderer, cancel).as_slice(),
                [Message::Chat(ChatMessage::CloseForkSessionDialog)]
            ));

            let close = assert_control_alignment(
                with_chat_fork_dialog(&app, root()),
                &renderer,
                size,
                None,
            );
            assert!(matches!(
                click(&mut fork, &mut renderer, close).as_slice(),
                [Message::Chat(ChatMessage::CloseForkSessionDialog)]
            ));

            let mut reset = scene(with_chat_reset_dialog(&app, root()), &mut renderer, size);
            let bounds = geometry(&mut reset, &renderer);
            assert_fits(&bounds, size);
            capture(
                &mut reset,
                &mut renderer,
                &theme,
                size,
                &format!("{language}-{mode}-reset"),
                mouse::Cursor::Unavailable,
            );
            assert!(key(&mut reset, &mut renderer, Key::Named(Named::Enter), None).is_empty());
            assert!(click(&mut reset, &mut renderer, Point::new(5.0, 5.0)).is_empty());
            let confirm = label(&bounds, i18n.__("确认")).center();
            assert!(matches!(
                click(&mut reset, &mut renderer, confirm).as_slice(),
                [Message::Chat(ChatMessage::ResetSessionToMessage { msg_idx: 1 })]
            ));
            let cancel = label(&bounds, i18n.__("取消")).center();
            assert!(matches!(
                click(&mut reset, &mut renderer, cancel).as_slice(),
                [Message::Chat(ChatMessage::CloseResetMenu)]
            ));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_32_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_32_preserves_values_and_disabled_states") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let long_name = format!("{}.txt", "长檔案名稱example".repeat(10));
    let long_title = "长會話名稱example".repeat(28);
    let project_dir = tempfile::tempdir().unwrap();
    app.file_tree_rename_path = Some(project_dir.path().join(&long_name).display().to_string());
    app.file_tree_rename_value = DRAFT.into();
    app.session_rename_id = Some("session-fixture".into());
    app.session_rename_value = DRAFT.into();
    let session = serde_json::from_value(serde_json::json!({
        "id": "session-fixture", "slug": "fixture", "projectID": "fixture",
        "directory": project_dir.path(), "title": long_title, "version": "1",
        "time": {"created": 1, "updated": 2}
    }))
    .unwrap();
    app.sessions = vec![session];

    for (locale, size, language) in [
        (DesktopLocale::SimplifiedChinese, VIEW_SIZE, "zh-cn"),
        (DesktopLocale::TraditionalChinese, Size::new(640, 540), "zh-tw"),
        (DesktopLocale::English, Size::new(960, 600), "en"),
    ] {
        app.desktop_locale = locale;
        let i18n = I18n::new(locale);
        for (theme, mode) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
            for (view, kind) in [
                (with_file_tree_rename(&app, root()), "file"),
                (with_session_rename(&app, root()), "session"),
            ] {
                let mut ui = scene(view, &mut renderer, size);
                let bounds = geometry(&mut ui, &renderer);
                assert_fits(&bounds, size);
                let [(value, input)] = bounds.inputs.as_slice() else {
                    panic!("expected one input")
                };
                assert_eq!(value, DRAFT);
                let view = if kind == "file" {
                    with_file_tree_rename(&app, root())
                } else {
                    with_session_rename(&app, root())
                };
                assert_control_alignment(view, &renderer, size, Some(*input));
                if kind == "session" {
                    let [(viewport, translation)] = bounds.scroll.as_slice() else {
                        panic!("expected a scrollable session title");
                    };
                    assert_eq!(translation.y, 0.0);
                    assert!(
                        send(
                            &mut ui,
                            &mut renderer,
                            mouse::Cursor::Available(viewport.center()),
                            &[Event::Mouse(mouse::Event::WheelScrolled {
                                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 },
                            }),]
                        )
                        .is_empty()
                    );
                    let after = geometry(&mut ui, &renderer);
                    assert!(after.scroll[0].1.y > 0.0);
                    assert_eq!(after.inputs, bounds.inputs);
                    assert_eq!(
                        label(&after, i18n.__("会话重命名")),
                        label(&bounds, i18n.__("会话重命名"))
                    );
                    assert_eq!(label(&after, i18n.__("确定")), label(&bounds, i18n.__("确定")));
                    capture(
                        &mut ui,
                        &mut renderer,
                        &theme,
                        size,
                        &format!("{language}-{mode}-session-scrolled"),
                        mouse::Cursor::Unavailable,
                    );
                    assert!(
                        send(
                            &mut ui,
                            &mut renderer,
                            mouse::Cursor::Available(viewport.center()),
                            &[Event::Mouse(mouse::Event::WheelScrolled {
                                delta: mouse::ScrollDelta::Lines { x: 0.0, y: 8.0 },
                            }),]
                        )
                        .is_empty()
                    );
                }
                assert!(key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).is_empty());
                assert!(click(&mut ui, &mut renderer, Point::new(5.0, 5.0)).is_empty());
                assert!(
                    click(
                        &mut ui,
                        &mut renderer,
                        Point::new(input.x + input.width - 8.0, input.center_y())
                    )
                    .is_empty()
                );
                capture(
                    &mut ui,
                    &mut renderer,
                    &theme,
                    size,
                    &format!("{language}-{mode}-{kind}"),
                    mouse::Cursor::Unavailable,
                );
                let changed = key(&mut ui, &mut renderer, Key::Character("x".into()), Some("x"));
                assert!(matches!((kind, changed.as_slice()),
                    ("file", [Message::Project(ProjectMessage::FileTreeRenameChanged(value))])
                    | ("session", [Message::Project(ProjectMessage::SessionRenameChanged(value))])
                        if value == &format!("{DRAFT}x")));
                let messages = key(&mut ui, &mut renderer, Key::Named(Named::Enter), None);
                assert!(matches!(
                    (kind, messages.as_slice()),
                    ("file", [Message::Project(ProjectMessage::FileTreeRenameSave)])
                        | ("session", [Message::Project(ProjectMessage::SessionRenameSave)])
                ));
                let save = label(&bounds, i18n.__("确定")).center();
                let messages = click(&mut ui, &mut renderer, save);
                assert!(matches!(
                    (kind, messages.as_slice()),
                    ("file", [Message::Project(ProjectMessage::FileTreeRenameSave)])
                        | ("session", [Message::Project(ProjectMessage::SessionRenameSave)])
                ));
                let cancel = label(&bounds, i18n.__("取消")).center();
                let messages = click(&mut ui, &mut renderer, cancel);
                assert!(matches!(
                    (kind, messages.as_slice()),
                    ("file", [Message::Project(ProjectMessage::FileTreeRenameCancel)])
                        | ("session", [Message::Project(ProjectMessage::SessionRenameCancel)])
                ));
            }
        }
    }

    app.chat.clear();
    app.active_session_id = None;
    app.chat_reset_menu_idx = Some(1);
    app.file_tree_rename_path = None;
    app.session_rename_id = None;
    for view in [
        with_chat_fork_dialog(&app, root()),
        with_chat_reset_dialog(&app, root()),
        with_file_tree_rename(&app, root()),
        with_session_rename(&app, root()),
    ] {
        let mut ui = scene(view, &mut renderer, VIEW_SIZE);
        let bounds = geometry(&mut ui, &renderer);
        assert!(bounds.inputs.is_empty());
        assert_eq!(bounds.labels.len(), 1);
        let underlay = label(&bounds, "underlay").center();
        assert!(matches!(
            click(&mut ui, &mut renderer, underlay).as_slice(),
            [Message::CloseError]
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_32_escape_cancels_only_the_visible_modal() {
    if !isolated("ui_refresh_32_escape_cancels_only_the_visible_modal") {
        return;
    }
    let mut app = fixture();
    prepare_chat(&mut app);
    app.file_tree_rename_path = Some("/fixture/original.txt".into());
    app.file_tree_rename_value = DRAFT.into();
    app.session_rename_id = Some("session-fixture".into());
    app.session_rename_value = DRAFT.into();
    let mut renderer = renderer().await;

    for (view, kind) in [
        (with_chat_fork_dialog(&app, with_chat_reset_dialog(&app, root())), "fork"),
        (with_chat_reset_dialog(&app, root()), "reset"),
        (with_file_tree_rename(&app, root()), "file"),
        (with_session_rename(&app, root()), "session"),
    ] {
        let mut ui = scene(view, &mut renderer, VIEW_SIZE);
        if let Some((_, input)) = geometry(&mut ui, &renderer).inputs.first() {
            assert!(click(&mut ui, &mut renderer, input.center()).is_empty());
        }
        let messages = key(&mut ui, &mut renderer, Key::Named(Named::Escape), None);
        assert!(
            matches!(
                (kind, messages.as_slice()),
                ("fork", [Message::Chat(ChatMessage::CloseForkSessionDialog)])
                    | ("reset", [Message::Chat(ChatMessage::CloseResetMenu)])
                    | ("file", [Message::Project(ProjectMessage::FileTreeRenameCancel)])
                    | ("session", [Message::Project(ProjectMessage::SessionRenameCancel)])
            ),
            "Escape did not cancel {kind}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn file_tree_delete_confirmation_draws_and_routes_actions() {
    if !isolated("file_tree_delete_confirmation_draws_and_routes_actions") {
        return;
    }
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::SimplifiedChinese;
    let path = "/fixture/项目/待删除的文件.txt";
    app.file_tree_pending_delete = Some(("/fixture/项目".into(), path.into()));
    let i18n = I18n::new(app.desktop_locale);
    let mut renderer = renderer().await;

    for theme in [Theme::Light, Theme::Dark] {
        let mut ui = scene(with_file_tree_delete_confirm(&app, root()), &mut renderer, VIEW_SIZE);
        let bounds = geometry(&mut ui, &renderer);
        assert_fits(&bounds, VIEW_SIZE);
        label(&bounds, path);
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            VIEW_SIZE,
            "file-tree-delete",
            mouse::Cursor::Unavailable,
        );
        assert!(key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).is_empty());
        assert!(matches!(
            click(&mut ui, &mut renderer, label(&bounds, i18n.__("删除")).center()).as_slice(),
            [Message::Project(ProjectMessage::FileTreeDeleteConfirmed)]
        ));
        assert!(matches!(
            click(&mut ui, &mut renderer, label(&bounds, i18n.__("取消")).center()).as_slice(),
            [Message::Project(ProjectMessage::FileTreeDeleteCanceled)]
        ));
        assert!(matches!(
            key(&mut ui, &mut renderer, Key::Named(Named::Escape), None).as_slice(),
            [Message::Project(ProjectMessage::FileTreeDeleteCanceled)]
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_32_invalid_file_name_feedback_stays_above_dialog() {
    if !isolated("ui_refresh_32_invalid_file_name_feedback_stays_above_dialog") {
        return;
    }
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    app.file_tree_rename_path = Some("/fixture/original.txt".into());
    app.file_tree_rename_value = "../wrong".into();
    let mut renderer = renderer().await;
    let mut ui = scene(with_file_tree_rename(&app, root()), &mut renderer, VIEW_SIZE);
    let input = geometry(&mut ui, &renderer).inputs[0].1;
    assert!(click(&mut ui, &mut renderer, input.center()).is_empty());
    let messages = key(&mut ui, &mut renderer, Key::Named(Named::Enter), None);
    drop(ui);
    let [message @ Message::Project(ProjectMessage::FileTreeRenameSave)] = messages.as_slice()
    else {
        panic!("expected exactly one save message");
    };
    let _ = app.update(message.clone());
    assert_eq!(app.file_tree_rename_path.as_deref(), Some("/fixture/original.txt"));
    assert_eq!(app.file_tree_rename_value, "../wrong");
    let toast = app.active_toast.as_ref().expect("invalid file name must be visible above dialog");
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
    assert!(label(&geometry(&mut ui, &renderer), "Invalid file name").y > input.center_y());
}
