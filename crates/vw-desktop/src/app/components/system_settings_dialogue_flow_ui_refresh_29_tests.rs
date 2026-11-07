//! Render the dialogue and quick message forms and exercise their real controls.
#![cfg(not(target_arch = "wasm32"))]

use super::super::{system_settings_agents as agents, system_settings_quick_messages as quick};
use super::*;
use crate::app::message::SettingsMessage;
use crate::app::{i18n::I18n, message::settings::QuickMessagesMessage, state::AgentConfigTab};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::keyboard::key::Named;
use iced::{Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::GatewayClientSystemSettingsConfig;
use vw_config_types::ui::{DesktopLocale, QuickMessageConfig, QuickMessageGroup, QuickMessageKind};

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_29_CHILD";
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

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

fn scene<'a>(view: Element<'a, Message>, renderer: &mut Renderer, size: Size<u32>) -> Ui<'a> {
    rebuild(view, renderer, size, Cache::new())
}

fn rebuild<'a>(
    view: Element<'a, Message>,
    renderer: &mut Renderer,
    size: Size<u32>,
    cache: Cache,
) -> Ui<'a> {
    UserInterface::build(view, Size::new(size.width as f32, size.height as f32), cache, renderer)
}

fn send(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    cursor: mouse::Cursor,
    events: &[Event],
) -> Vec<Message> {
    let mut messages = Vec::new();
    ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
    messages.retain(|message| !matches!(message, Message::None));
    messages
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_29_SCREENSHOTS") {
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

fn key(name: iced::keyboard::key::Named) -> Event {
    use iced::keyboard::{self, Key};
    let key = Key::Named(name);
    Event::Keyboard(keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    })
}

#[derive(Default)]
struct Geometry {
    labels: Vec<(String, iced::Rectangle)>,
    inputs: Vec<(String, iced::Rectangle)>,
    focusables: Vec<iced::Rectangle>,
    scroll: Option<(iced::Rectangle, iced::Vector)>,
}

impl iced::advanced::widget::Operation for Geometry {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        children(self);
    }

    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        value: &str,
    ) {
        self.labels.push((value.into(), bounds));
    }

    fn text_input(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        state: &mut dyn iced::advanced::widget::operation::TextInput,
    ) {
        self.inputs.push((state.text().into(), bounds));
    }
    fn focusable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        _: &mut dyn iced::advanced::widget::operation::Focusable,
    ) {
        self.focusables.push(bounds);
    }

    fn scrollable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        _: iced::Rectangle,
        translation: iced::Vector,
        _: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        self.scroll = Some((bounds, translation));
    }
}

impl Geometry {
    fn label_at(&self, value: &str, index: usize) -> iced::Rectangle {
        self.labels
            .iter()
            .filter(|(label, _)| label == value)
            .nth(index)
            .unwrap_or_else(|| panic!("missing label {value}: {:?}", self.labels))
            .1
    }

    fn label(&self, value: &str) -> Rectangle {
        self.label_at(value, 0)
    }

    fn input(&self, value: &str) -> iced::Rectangle {
        self.inputs
            .iter()
            .find(|(input, _)| input == value)
            .unwrap_or_else(|| panic!("missing fixture input {value}"))
            .1
    }
}

fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
    let mut geometry = Geometry::default();
    ui.operate(renderer, &mut geometry);
    geometry
}

fn click(ui: &mut Ui<'_>, renderer: &mut Renderer, point: iced::Point) -> Vec<Message> {
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

fn at(point: Point) -> mouse::Cursor {
    mouse::Cursor::Available(point)
}

fn key_at(ui: &mut Ui<'_>, renderer: &mut Renderer, name: Named) -> Vec<Message> {
    send(ui, renderer, mouse::Cursor::Unavailable, &[key(name)])
}

fn assert_fits(bounds: Rectangle, size: Size<u32>) {
    assert!(bounds.width > 0.0 && bounds.height > 0.0, "{bounds:?}");
    assert!(bounds.x >= 0.0 && bounds.y >= 0.0, "{bounds:?}");
    assert!(bounds.x + bounds.width <= size.width as f32 + 0.1, "{bounds:?}");
    assert!(bounds.y + bounds.height <= size.height as f32 + 0.1, "{bounds:?}");
}

const TEMPLATE: &str = "檢查設定 /fixture/abcdefghijklmnopqrstuvwxyz/abcdefghijklmnopqrstuvwxyz/config.toml\r\n保留 `git status` 的結果。";
const FAILURE: &str = "Fixture request failed / 範例請求失敗";

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    App::new().0
}

fn templates() -> Vec<QuickMessageConfig> {
    vec![
        QuickMessageConfig {
            text: TEMPLATE.into(),
            group: QuickMessageGroup::General,
            kind: QuickMessageKind::FillInput,
        },
        QuickMessageConfig {
            text: "Create a new review session".into(),
            group: QuickMessageGroup::Git,
            kind: QuickMessageKind::CreateNewSession,
        },
    ]
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_29_layout_and_interaction() {
    if !isolated("ui_refresh_29_layout_and_interaction") {
        return;
    }
    let mut app = fixture();
    app.dialogue_flow_show_reasoning_summary = true;
    app.dialogue_flow_expand_shell_tool_section = false;
    app.dialogue_flow_expand_edit_tool_section = true;
    app.quick_messages = templates();
    let mut renderer = renderer().await;
    let (first_line, second_line) = TEMPLATE.split_once("\r\n").unwrap();
    for (locale, width, theme, language) in [
        (DesktopLocale::English, 640, Theme::Light, "en"),
        (DesktopLocale::SimplifiedChinese, 960, Theme::Dark, "zh-cn"),
        (DesktopLocale::TraditionalChinese, 420, Theme::Dark, "zh-tw"),
    ] {
        app.desktop_locale = locale;
        app.dialogue_flow_settings_save_message = match locale {
            DesktopLocale::English => None,
            DesktopLocale::SimplifiedChinese => Some("已保存对话流配置".into()),
            _ => Some(FAILURE.into()),
        };
        let i18n = I18n::new(locale);
        let size = Size::new(width, 620);
        let mut ui = scene(view(&app), &mut renderer, size);
        let positions = geometry(&mut ui, &renderer);
        assert_fits(positions.label(i18n.__("对话流")), size);
        if let Some(status) = &app.dialogue_flow_settings_save_message {
            assert_fits(positions.label(status), size);
        }
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            &format!("dialogue-{language}-{width}"),
            mouse::Cursor::Unavailable,
        );

        for (index, expected_label, description) in [
            (0, "显示推理摘要", "在时间线中显示模型推理摘要。"),
            (1, "展开 shell 工具", "默认在时间线中展开 shell 工具部分。"),
            (2, "展开编辑工具", "默认在时间线中展开 edit、write、patch 工具部分。"),
        ] {
            let checkbox = positions.label_at(i18n.__("启用"), index);
            assert_fits(checkbox, size);
            let label = positions.label(i18n.__(expected_label));
            let hint = positions.label(i18n.__(description));
            assert_fits(label, size);
            assert_fits(hint, size);
            assert!((checkbox.center_y() - (label.y + hint.y + hint.height) / 2.0).abs() < 0.5);
            let messages = click(&mut ui, &mut renderer, checkbox.center());
            assert!(matches!(
                (index, messages.as_slice()),
                (
                    0,
                    [Message::Settings(SettingsMessage::DialogueFlowShowReasoningSummaryToggled(
                        false
                    ))]
                ) | (
                    1,
                    [Message::Settings(
                        SettingsMessage::DialogueFlowExpandShellToolSectionToggled(true)
                    )]
                ) | (
                    2,
                    [Message::Settings(SettingsMessage::DialogueFlowExpandEditToolSectionToggled(
                        false
                    ))]
                )
            ));
        }
        let mut ui = scene(quick::view(&app), &mut renderer, size);
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            &format!("quick-{language}-{width}"),
            mouse::Cursor::Unavailable,
        );
        let positions = geometry(&mut ui, &renderer);
        let first = positions.input(first_line);
        let second = positions.input(second_line);
        assert_fits(first, size);
        assert_fits(second, size);
        assert!(first.y + first.height <= second.y, "both template lines must be visible");
        assert_fits(positions.label(i18n.__("保存快捷消息")), size);
    }

    app.desktop_locale = DesktopLocale::TraditionalChinese;
    let size = Size::new(420, 450);
    let mut ui = scene(quick::view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let [group, kind, input, ..] = positions.focusables.as_slice() else {
        panic!("expected two selectors and a text input: {:?}", positions.focusables);
    };
    assert_eq!(*input, positions.input(first_line));
    assert_fits(*input, size);
    for control in [group, kind] {
        assert_fits(*control, size);
        assert!((control.height - 38.0).abs() < 0.5, "{control:?}");
    }
    assert!((group.center_y() - kind.center_y()).abs() < 0.5);
    assert!(click(&mut ui, &mut renderer, group.center()).is_empty());
    let menu = geometry(&mut ui, &renderer).scroll.unwrap().0;
    let git = Point::new(menu.center_x(), menu.y + 56.0);
    assert!(matches!(
        click(&mut ui, &mut renderer, git).as_slice(),
        [Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::GroupChanged(
            0,
            QuickMessageGroup::Git
        )))]
    ));

    assert!(click(&mut ui, &mut renderer, group.center()).is_empty());
    let delete = positions.label(I18n::new(app.desktop_locale).__("删除"));
    assert!(click(&mut ui, &mut renderer, delete.center()).is_empty());
    assert!(geometry(&mut ui, &renderer).scroll.is_none());

    assert!(click(&mut ui, &mut renderer, kind.center()).is_empty());
    let menu = geometry(&mut ui, &renderer).scroll.unwrap().0;
    assert_fits(menu, size);
    let mut screenshots = Vec::new();
    for (offset, name) in [(18.0, "kind-hover-first"), (56.0, "kind-hover-second")] {
        let point = Point::new(menu.center_x(), menu.y + offset);
        assert!(
            send(
                &mut ui,
                &mut renderer,
                at(point),
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })]
            )
            .is_empty()
        );
        screenshots.push(capture(&mut ui, &mut renderer, &Theme::Dark, size, name, at(point)));
    }
    assert_ne!(screenshots[0], screenshots[1]);
    let point = Point::new(menu.center_x(), menu.y + 72.0);
    assert!(
        send(
            &mut ui,
            &mut renderer,
            at(point),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -8.0 },
            })]
        )
        .is_empty()
    );
    assert!(geometry(&mut ui, &renderer).scroll.unwrap().1.y > 0.0);
    assert!(
        send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Unavailable,
            &[Event::Mouse(mouse::Event::CursorLeft)]
        )
        .is_empty()
    );
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "kind-scrolled-no-hover",
        mouse::Cursor::Unavailable,
    );
    assert!(key_at(&mut ui, &mut renderer, Named::Escape).is_empty());
    assert!(geometry(&mut ui, &renderer).scroll.is_none());
    assert!(key_at(&mut ui, &mut renderer, Named::Enter).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::ArrowDown).is_empty());
    assert!(matches!(
        key_at(&mut ui, &mut renderer, Named::Enter).as_slice(),
        [Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::KindChanged(
            0,
            QuickMessageKind::Message
        )))]
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_refresh_29_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_29_preserves_values_and_disabled_states") {
        return;
    }
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    let i18n = I18n::new(app.desktop_locale);
    app.quick_messages = templates();
    let mut renderer = renderer().await;
    let size = Size::new(640, 580);
    let mut ui = scene(quick::view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    assert!(
        click(
            &mut ui,
            &mut renderer,
            positions.input(TEMPLATE.split_once("\r\n").unwrap().1).center()
        )
        .is_empty()
    );
    assert!(key_at(&mut ui, &mut renderer, Named::End).is_empty());
    let messages = key_at(&mut ui, &mut renderer, Named::Backspace);
    assert!(matches!(messages.as_slice(),
        [Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::TextChanged(0, value)))]
        if value == TEMPLATE.strip_suffix('。').unwrap()));
    let cache = ui.into_cache();
    let [Message::Settings(update)] = messages.as_slice() else { unreachable!() };
    let _ = crate::app::message::settings::update(&mut app, update.clone());
    let mut ui = rebuild(quick::view(&app), &mut renderer, size, cache);
    let messages = key_at(&mut ui, &mut renderer, Named::Backspace);
    assert!(matches!(messages.as_slice(),
        [Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::TextChanged(0, value)))]
        if value == TEMPLATE.strip_suffix("果。").unwrap()));

    for (label, index) in
        [("上移", 1), ("下移", 0), ("删除", 1), ("添加快捷项", 0), ("保存快捷消息", 0)]
    {
        let messages =
            click(&mut ui, &mut renderer, positions.label_at(i18n.__(label), index).center());
        assert!(matches!(
            (label, messages.as_slice()),
            (
                "上移",
                [Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::MoveUp(
                    1
                )))]
            ) | (
                "下移",
                [Message::Settings(SettingsMessage::QuickMessages(
                    QuickMessagesMessage::MoveDown(0)
                ))]
            ) | (
                "删除",
                [Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::Remove(
                    1
                )))]
            ) | (
                "添加快捷项",
                [Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::Add))]
            ) | (
                "保存快捷消息",
                [Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::Save))]
            )
        ));
    }
    let (first_line, second_line) = TEMPLATE.split_once("\r\n").unwrap();
    assert!(click(&mut ui, &mut renderer, positions.input(first_line).center()).is_empty());
    assert!(key_at(&mut ui, &mut renderer, Named::End).is_empty());
    assert!(matches!(key_at(&mut ui, &mut renderer, Named::Backspace).as_slice(),
        [Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::TextChanged(0, value)))]
        if value == &format!("{}\r\n{}", first_line.strip_suffix('l').unwrap(), second_line.strip_suffix("果。").unwrap())));
    assert!(key_at(&mut ui, &mut renderer, Named::Enter).is_empty());

    for (label, index) in [("上移", 0), ("下移", 1)] {
        let messages =
            click(&mut ui, &mut renderer, positions.label_at(i18n.__(label), index).center());
        assert!(messages.is_empty(), "a move beyond the list boundary must be disabled");
    }
    drop(ui);

    app.agents_settings.active_config_tab = AgentConfigTab::QuickMessages;
    app.quick_messages = (0..4).flat_map(|_| templates()).collect();
    let size = Size::new(640, 420);
    let mut ui = scene(agents::view(&app), &mut renderer, size);
    let positions = geometry(&mut ui, &renderer);
    let (page, before) = positions.scroll.unwrap();
    assert!(
        send(
            &mut ui,
            &mut renderer,
            at(page.center()),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -8.0 }
            })]
        )
        .is_empty()
    );
    assert!(geometry(&mut ui, &renderer).scroll.unwrap().1.y > before.y);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "agents-quick-scrolled-en-640",
        at(page.center()),
    );
    drop(ui);

    app.quick_messages.clear();
    let mut ui = scene(quick::view(&app), &mut renderer, size);
    assert_fits(
        geometry(&mut ui, &renderer).label(i18n.__("暂无快捷消息；添加后会显示在聊天输入框上方。")),
        size,
    );
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "quick-empty-en-640",
        mouse::Cursor::Unavailable,
    );
}
