//! Exercise the session dialogs through Iced's headless layout, event and draw paths.
#![cfg(not(target_arch = "wasm32"))]

use super::{App, ForkSessionTarget, Message, with_chat_fork_dialog};
use crate::app::i18n::I18n;
use crate::app::impls::app_view_modals::{
    with_chat_reset_dialog, with_file_tree_rename, with_session_rename,
};
use crate::app::message::{ChatMessage, ProjectMessage};
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
    app.chat = (0..2)
        .map(|_| TranscriptMessage {
            role: ChatRole::User,
            content: "虚构消息・測試訊息・Fixture".into(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        })
        .collect();
    app.chat_fork_dialog_idx = Some(1);
    app.chat_reset_menu_idx = Some(1);
}

const DRAFT: &str = "新草稿・新檔案・draft";
const VIEW_SIZE: Size<u32> = Size::new(420, 480);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_32_layout_and_interaction() {
    if !isolated("ui_refresh_32_layout_and_interaction") {
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
            let local = label(&bounds, i18n.__("打开一个新会话，并复制到当前消息为止")).center();
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
                    msg_idx: 1,
                    target: ForkSessionTarget::Local
                })]
            ));
            assert!(matches!(
                click(&mut fork, &mut renderer, worktree).as_slice(),
                [Message::Chat(ChatMessage::ForkSessionAt {
                    msg_idx: 1,
                    target: ForkSessionTarget::NewWorktree
                })]
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
            for (title, revert) in [("仅重置会话历史", false), ("回滚代码并重置", true)]
            {
                let point = label(&bounds, i18n.__(title)).center();
                assert!(matches!(click(&mut reset, &mut renderer, point).as_slice(), [
                    Message::Chat(ChatMessage::ResetSessionToMessage { msg_idx: 1, revert_code })
                ] if *revert_code == revert));
            }
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
    app.chat_fork_dialog_idx = Some(1);
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
