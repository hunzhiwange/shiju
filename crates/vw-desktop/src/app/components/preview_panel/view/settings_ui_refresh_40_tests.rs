//! Exercise the rendered preview settings and terminal rename controls.

use super::*;
#[cfg(unix)]
use crate::app::components::terminal_panel;
use crate::app::i18n::I18n;
#[cfg(unix)]
use crate::app::message::terminal;
use crate::app::message::terminal::TerminalMessage;
#[cfg(unix)]
use crate::app::terminal::{Shell, TerminalState, TerminalTheme};
use iced::advanced::widget::{Id, Operation, operation};
use iced::advanced::{clipboard, renderer::Headless};
use iced::keyboard::{self, Key, key::Named};
use iced::{Event, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_40_CHILD";
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
    let test = format!("{}::{test}", module_path!().split_once("::").unwrap().1);
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &test])
        .env(CHILD, "1")
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

fn scene<'a>(view: Element<'a, Message>, renderer: &mut Renderer, size: Size<u32>) -> Ui<'a> {
    UserInterface::build(
        view,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    )
}

fn send(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point, events: &[Event]) -> Vec<Message> {
    let mut messages = Vec::new();
    ui.update(
        events,
        mouse::Cursor::Available(point),
        renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    messages
}

fn click(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Vec<Message> {
    send(
        ui,
        renderer,
        point,
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
        Point::ORIGIN,
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
    point: Point,
) -> Vec<u8> {
    let redraw = send(
        ui,
        renderer,
        point,
        &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
    );
    assert!(
        redraw
            .iter()
            .all(|message| matches!(message, Message::Terminal(TerminalMessage::Event(_)))),
        "{redraw:?}"
    );
    // UserInterface rebuilds the active overlay after update before drawing it.
    ui.draw(
        renderer,
        theme,
        &iced::advanced::renderer::Style { text_color: theme.palette().text },
        mouse::Cursor::Available(point),
    );
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_40_SCREENSHOTS") {
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
struct Bounds {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    focus: Vec<Rectangle>,
    body: Option<(Rectangle, Vector)>,
    menu: Option<Rectangle>,
}

impl Operation for Bounds {
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
        input: &mut dyn operation::TextInput,
    ) {
        self.inputs.push((input.text().into(), bounds));
    }

    fn focusable(&mut self, _: Option<&Id>, bounds: Rectangle, _: &mut dyn operation::Focusable) {
        self.focus.push(bounds);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        offset: Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        if id == Some(&Id::from("settings-select-menu")) {
            self.menu = Some(bounds);
        } else {
            self.body = Some((bounds, offset));
        }
    }
}

fn bounds(ui: &mut Ui<'_>, renderer: &Renderer) -> Bounds {
    let mut result = Bounds::default();
    ui.operate(renderer, &mut result);
    result
}

fn position(controls: &[(String, Rectangle)], value: &str) -> Rectangle {
    controls
        .iter()
        .find(|(text, _)| text == value)
        .unwrap_or_else(|| panic!("missing {value:?} in {controls:?}"))
        .1
}

fn preview(app: &App) -> Element<'_, Message> {
    let base = container(
        iced::widget::mouse_area(
            container(text("Preview fixture")).width(Length::Fill).height(Length::Fill),
        )
        .on_press(Message::Terminal(TerminalMessage::Add)),
    )
    .width(Length::Fill)
    .height(Length::Fill);
    build_settings_overlay(app, base.into())
}

#[cfg(unix)]
fn assert_centered_text(root: iced::advanced::Layout<'_>, input: Rectangle) {
    fn find<'a>(
        layout: iced::advanced::Layout<'a>,
        bounds: Rectangle,
    ) -> Option<iced::advanced::Layout<'a>> {
        if layout.bounds() == bounds {
            Some(layout)
        } else {
            layout.children().find_map(|child| find(child, bounds))
        }
    }

    let layout = find(root, input).expect("terminal rename input layout");
    let text = layout.children().next().expect("terminal rename text layout").bounds();
    assert!((input.center_y() - text.center_y()).abs() < 0.5);
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_40_terminal_rename_isolates_card_and_preserves_messages() {
    if !isolated("ui_refresh_40_terminal_rename_isolates_card_and_preserves_messages") {
        return;
    }
    let mut app = fixture();
    let home = tempfile::tempdir().unwrap();
    app.terminal = TerminalState::blank_with_settings(
        true,
        Shell::Bash,
        TerminalTheme::System,
        "JetBrains Mono".into(),
        13.0,
        200.0,
    );
    let title = "  終端名称 Terminal / 测试  ";
    let id = app
        .terminal
        .add_terminal_with_command(
            title.into(),
            Some(home.path().into()),
            "read -r line; printf 'PTY_READY\\n'; read -r finish".into(),
        )
        .expect("isolated PTY");
    let _task = terminal::update(&mut app, TerminalMessage::RenameStart(id));
    let mut renderer = renderer().await;

    for (width, height, locale, theme, name) in [
        (420, 112, DesktopLocale::English, Theme::Light, "terminal-420-compact"),
        (420, 192, DesktopLocale::TraditionalChinese, Theme::Dark, "terminal-420-dark"),
        (640, 260, DesktopLocale::SimplifiedChinese, Theme::Light, "terminal-640-light"),
        (960, 260, DesktopLocale::English, Theme::Dark, "terminal-960-dark"),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        // The project layout reserves 8px for the terminal resize handle.
        app.terminal.height = height as f32 + 8.0;
        let draft = if width == 420 { "終端名称 / 测试 / ".repeat(18) } else { title.into() };
        app.terminal.tabs[0].edit_title = Some(draft.clone());
        let size = Size::new(width, height);
        let mut ui = scene(terminal_panel::view(&app), &mut renderer, size);
        capture(&mut ui, &mut renderer, &theme, size, name, Point::ORIGIN);
        let controls = bounds(&mut ui, &renderer);
        let input = position(&controls.inputs, &draft);
        inside(input, size);
        let mut element = terminal_panel::view(&app);
        let mut tree = iced::advanced::widget::Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &iced::advanced::layout::Limits::new(
                Size::ZERO,
                Size::new(width as f32, height as f32),
            ),
        );
        assert_centered_text(iced::advanced::Layout::new(&node), input);
        let i18n = I18n::new(locale);
        for label in ["重命名终端", "取消", "确定"] {
            inside(position(&controls.labels, i18n.__(label)), size);
        }
    }
    app.terminal.tabs[0].edit_title = Some(title.into());

    let i18n = I18n::new(app.desktop_locale);
    let mut ui = scene(terminal_panel::view(&app), &mut renderer, Size::new(640, 260));
    let controls = bounds(&mut ui, &renderer);
    let heading = position(&controls.labels, i18n.__("重命名终端"));
    let heading_click = click(&mut ui, &mut renderer, heading.center());
    assert!(heading_click.is_empty(), "click inside rename card sent {heading_click:?}");
    let input = position(&controls.inputs, title);
    assert!(click(&mut ui, &mut renderer, input.center()).is_empty());
    let changed = key(&mut ui, &mut renderer, Key::Character("界".into()), Some("界"));
    assert!(matches!(
        changed.as_slice(),
        [Message::Terminal(TerminalMessage::RenameChanged(tab, value))]
            if *tab == id && value.replace('界', "") == title
    ));
    assert!(matches!(
        key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).as_slice(),
        [Message::Terminal(TerminalMessage::RenameSave(tab))] if *tab == id
    ));
    let confirm = position(&controls.labels, i18n.__("确定"));
    assert!(matches!(
        click(&mut ui, &mut renderer, confirm.center()).as_slice(),
        [Message::Terminal(TerminalMessage::RenameSave(tab))] if *tab == id
    ));
    let cancel = position(&controls.labels, i18n.__("取消"));
    let cancelled = click(&mut ui, &mut renderer, cancel.center());
    assert!(matches!(
        cancelled.as_slice(),
        [Message::Terminal(TerminalMessage::RenameCancel(tab))] if *tab == id
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, Point::new(5.0, 5.0)).as_slice(),
        [Message::Terminal(TerminalMessage::RenameCancel(tab))] if *tab == id
    ));
    drop(ui);
    let [Message::Terminal(changed)]: [Message; 1] = changed.try_into().unwrap() else {
        unreachable!()
    };
    let _task = terminal::update(&mut app, changed);
    assert!(app.terminal.tabs[0].edit_title.as_deref().unwrap().contains('界'));
    let [Message::Terminal(cancelled)]: [Message; 1] = cancelled.try_into().unwrap() else {
        unreachable!()
    };
    let _task = terminal::update(&mut app, cancelled);
    assert_eq!(app.terminal.active_id, Some(id));
    assert_eq!(app.terminal.tabs.len(), 1);
    assert_eq!(app.terminal.tabs[0].title, title);
    assert_eq!(app.terminal.tabs[0].edit_title, None);

    let term = &mut app.terminal.tabs[0].term;
    let resize = || {
        iced_term::Command::ProxyToBackend(iced_term::BackendCommand::Resize(
            Some(Size::new(420.0, 196.0)),
            None,
        ))
    };
    term.handle(resize());
    term.handle(iced_term::Command::ProxyToBackend(iced_term::BackendCommand::Write(
        b"ping\n".to_vec(),
    )));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        term.handle(resize());
        if term.text().iter().any(|line| line.contains("PTY_READY")) {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "PTY must still respond after cancellation");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    app.terminal.close_terminal(id);
    app.terminal.reap_retired();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_40_preview_rendering() {
    if !isolated("ui_refresh_40_preview_rendering") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    app.editor_follow_system_theme = false;
    app.editor_theme =
        Theme::custom("長い編輯器主题 Long Editor Theme ".repeat(6), Theme::Dark.palette());
    for (width, height, locale, theme, name) in [
        (420, 420, DesktopLocale::TraditionalChinese, Theme::Dark, "preview-420-dark"),
        (420, 420, DesktopLocale::English, Theme::Light, "preview-420-light"),
        (640, 720, DesktopLocale::SimplifiedChinese, Theme::Light, "preview-640-light"),
        (960, 720, DesktopLocale::English, Theme::Dark, "preview-960-dark"),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        let size = Size::new(width, height);
        let mut ui = scene(preview(&app), &mut renderer, size);
        capture(&mut ui, &mut renderer, &theme, size, name, Point::ORIGIN);
        let controls = bounds(&mut ui, &renderer);
        let (body, _) = controls.body.unwrap();
        assert!((body.center_x() - width as f32 * 0.5).abs() < 0.5);
        if width == 420 {
            assert!(scroll(&mut ui, &mut renderer, body.center(), -800.0).is_empty());
            capture(
                &mut ui,
                &mut renderer,
                &theme,
                size,
                &format!("{name}-scrolled"),
                Point::ORIGIN,
            );
        }
    }
}

fn keyboard(ui: &mut Ui<'_>, renderer: &mut Renderer, name: Named) -> Vec<Message> {
    key(ui, renderer, Key::Named(name), None)
}

fn hover(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Vec<Message> {
    send(ui, renderer, point, &[Event::Mouse(mouse::Event::CursorMoved { position: point })])
}

fn scroll(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point, y: f32) -> Vec<Message> {
    send(
        ui,
        renderer,
        point,
        &[Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y },
        })],
    )
}

fn inside(rect: Rectangle, size: Size<u32>) {
    assert!(
        rect.width > 0.0
            && rect.height > 0.0
            && rect.x >= 0.0
            && rect.y >= 0.0
            && rect.x + rect.width <= size.width as f32
            && rect.y + rect.height <= size.height as f32,
        "{rect:?} outside {size:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_40_layout_and_interaction() {
    if !isolated("ui_refresh_40_layout_and_interaction") {
        return;
    }
    use iced_code_editor::i18n::Language;

    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    app.current_language = Language::English;
    let mut renderer = renderer().await;
    let i18n = I18n::new(app.desktop_locale);
    let size = Size::new(420, 420);
    let mut ui = scene(preview(&app), &mut renderer, size);
    let before = bounds(&mut ui, &renderer);
    let (body, offset) = before.body.expect("modal body");
    assert_eq!(offset.y, 0.0);
    let title = position(&before.labels, i18n.__("编辑器设置"));
    let done = position(&before.labels, i18n.__("完成"));
    assert!(title.y + title.height < body.y);
    assert!(done.y > body.y + body.height);
    inside(title, size);
    inside(done, size);
    assert!(click(&mut ui, &mut renderer, Point::new(2.0, 2.0)).is_empty());

    // Scroll only the card body until the language trigger is fully visible.
    let scroll_distance = before.focus[0].center_y() - body.y - body.height * 0.45;
    assert!(scroll_distance > 0.0);
    assert!(scroll(&mut ui, &mut renderer, body.center(), -scroll_distance).is_empty());
    let after = bounds(&mut ui, &renderer);
    assert!(after.body.unwrap().1.y > 0.0);
    assert_eq!(position(&after.labels, i18n.__("编辑器设置")), title);
    assert_eq!(position(&after.labels, i18n.__("完成")), done);
    let [language, theme] = after.focus.as_slice() else { panic!("two select triggers") };
    for trigger in [language, theme] {
        assert_eq!(trigger.height, 38.0);
        assert!(trigger.x >= body.x + 7.0);
        assert!(trigger.x + trigger.width <= body.x + body.width - 7.0);
    }
    // Iced reports content-space bounds during widget operations.
    let language_point = language.center() - after.body.unwrap().1;
    assert!(body.contains(language_point));
    assert!(click(&mut ui, &mut renderer, language_point).is_empty());
    let menu = bounds(&mut ui, &renderer).menu.expect("language menu");
    inside(menu, size);

    let first = Point::new(menu.center_x(), menu.y + 24.0);
    let second = Point::new(menu.center_x(), menu.y + 60.0);
    assert!(hover(&mut ui, &mut renderer, first).is_empty());
    let first_frame =
        capture(&mut ui, &mut renderer, &Theme::Light, size, "language-hover-first", first);
    assert!(hover(&mut ui, &mut renderer, second).is_empty());
    let second_frame =
        capture(&mut ui, &mut renderer, &Theme::Light, size, "language-hover-second", second);
    assert_ne!(first_frame, second_frame);
    assert!(hover(&mut ui, &mut renderer, Point::ORIGIN).is_empty());
    let outside =
        capture(&mut ui, &mut renderer, &Theme::Light, size, "language-hover-exit", Point::ORIGIN);
    assert_ne!(outside, second_frame);
    assert!(scroll(&mut ui, &mut renderer, menu.center(), -80.0).is_empty());
    assert!(keyboard(&mut ui, &mut renderer, Named::Escape).is_empty());
    assert!(bounds(&mut ui, &renderer).menu.is_none());

    // The focused trigger reopens from the keyboard after cancelling the menu.
    for name in [Named::Enter, Named::Home, Named::ArrowDown] {
        assert!(keyboard(&mut ui, &mut renderer, name).is_empty());
    }
    assert!(matches!(
        keyboard(&mut ui, &mut renderer, Named::Enter).as_slice(),
        [Message::Editor(EditorMessage::LanguageChanged(Language::ChineseSimplified))]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, done.center()).as_slice(),
        [Message::Editor(EditorMessage::ToggleSettings)]
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_40_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_40_preserves_values_and_disabled_states") {
        return;
    }
    use crate::app::message::editor;

    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    app.current_font_size = 16.0;
    app.current_line_height = 26.0;
    app.auto_adjust_line_height = false;
    app.app_theme = Theme::Dark;
    app.editor_theme = Theme::Light;
    app.editor_follow_system_theme = true;
    let size = Size::new(640, 720);
    let mut renderer = renderer().await;
    let mut ui = scene(preview(&app), &mut renderer, size);
    let controls = bounds(&mut ui, &renderer);
    let [language, theme] = controls.focus.as_slice() else { panic!("two select triggers") };
    let (body, offset) = controls.body.unwrap();
    assert_eq!(offset, Vector::ZERO);

    let enabled = I18n::new(app.desktop_locale).__("启用");
    let [auto_line_height, follow] = controls
        .labels
        .iter()
        .filter_map(|(label, bounds)| (label == enabled).then_some(*bounds))
        .collect::<Vec<_>>()[..]
    else {
        panic!("two editor checkboxes");
    };
    assert!(matches!(
        click(&mut ui, &mut renderer, auto_line_height.center()).as_slice(),
        [Message::Editor(EditorMessage::ToggleAutoLineHeight(true))]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, follow.center()).as_slice(),
        [Message::Editor(EditorMessage::ToggleFollowSystemTheme(false))]
    ));

    let mut edits = Vec::new();
    for (badge, is_font) in [("16 pt", true), ("26.0", false)] {
        let label = position(&controls.labels, badge);
        let start = Point::new(language.x + 1.0, label.center_y());
        let end = Point::new(label.x + label.width, start.y);
        let pressed = send(
            &mut ui,
            &mut renderer,
            start,
            &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))],
        );
        let dragged = hover(&mut ui, &mut renderer, end);
        if is_font {
            assert!(matches!(
                pressed.as_slice(),
                [Message::Editor(EditorMessage::FontSizePreview(10.0))]
            ));
            assert!(matches!(
                dragged.as_slice(),
                [Message::Editor(EditorMessage::FontSizePreview(30.0))]
            ));
        } else {
            assert!(matches!(
                pressed.as_slice(),
                [Message::Editor(EditorMessage::LineHeightPreview(10.0))]
            ));
            assert!(matches!(
                dragged.as_slice(),
                [Message::Editor(EditorMessage::LineHeightPreview(50.0))]
            ));
        }
        let released = send(
            &mut ui,
            &mut renderer,
            end,
            &[Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))],
        );
        assert!(matches!(released.as_slice(), [Message::Editor(EditorMessage::CommitMetrics)]));
        edits.extend(dragged.into_iter().chain(released));
    }

    // Choosing a manual theme from the preview has always exited system following.
    assert!(scroll(&mut ui, &mut renderer, body.center(), -800.0).is_empty());
    let theme_point = theme.center() - bounds(&mut ui, &renderer).body.unwrap().1;
    assert!(body.contains(theme_point));
    assert!(click(&mut ui, &mut renderer, theme_point).is_empty());
    let menu = bounds(&mut ui, &renderer).menu.expect("theme menu");
    inside(menu, size);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "preview-system-theme-menu", Point::ORIGIN);
    assert!(keyboard(&mut ui, &mut renderer, Named::Home).is_empty());
    let selected = keyboard(&mut ui, &mut renderer, Named::Enter);
    assert!(matches!(
        selected.as_slice(),
        [Message::Editor(EditorMessage::ThemeChanged(Theme::Light))]
    ));
    drop(ui);

    for (i, message) in edits.into_iter().enumerate() {
        let Message::Editor(message) = message else { unreachable!() };
        let _task = editor::update(&mut app, message);
        match i {
            0 => assert_eq!(app.current_font_size, 16.0),
            1 => assert_eq!(app.current_font_size, 30.0),
            2 => assert_eq!(app.current_line_height, 26.0),
            3 => assert_eq!(app.current_line_height, 50.0),
            _ => unreachable!(),
        }
    }
    let [Message::Editor(selected)]: [Message; 1] = selected.try_into().unwrap() else {
        unreachable!()
    };
    let _task = editor::update(&mut app, selected);
    assert_eq!(app.editor_theme, Theme::Light);
    assert!(!app.editor_follow_system_theme);
}
