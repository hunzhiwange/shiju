// 宿主契约：代理策略页面依赖技能领域；保留真实跨页策略事件、整形和帮助交互。
#![cfg(not(target_arch = "wasm32"))]

use super::*;
use crate::app::state::SkillsDirectoryScope;
use config_ui::{DesktopLocale, GatewayClientSystemSettingsConfig};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
};
use iced::{Event, Font, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_19_CHILD";
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_19_SCREENSHOTS") {
        image::save_buffer(
            std::path::PathBuf::from(directory).join(format!("{name}.png")),
            &pixels,
            size.width,
            size.height,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
    pixels
}

const PATH: &str = "/虚构專案/fixture-project";
fn fixture() -> App {
    App::new().0
}
#[derive(Default)]
struct Geometry {
    labels: Vec<(String, iced::Rectangle)>,
    inputs: Vec<(String, iced::Rectangle)>,
    focusable: Vec<iced::Rectangle>,
    scroll_offset: iced::Vector,
}

impl iced::advanced::widget::Operation for Geometry {
    fn scrollable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        _: iced::Rectangle,
        _: iced::Rectangle,
        translation: iced::Vector,
        _: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        self.scroll_offset = translation;
    }

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
        self.focusable.push(bounds);
    }
}

impl Geometry {
    fn label(&self, value: &str) -> iced::Rectangle {
        self.labels
            .iter()
            .find(|(label, _)| label == value)
            .unwrap_or_else(|| panic!("missing label {value}: {:?}", self.labels))
            .1
    }
}

fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
    let mut positions = Geometry::default();
    ui.operate(renderer, &mut positions);
    positions
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

fn press_label(ui: &mut Ui<'_>, renderer: &mut Renderer, value: &str) -> Vec<Message> {
    let point = geometry(ui, renderer).label(value).center();
    click(ui, renderer, point)
}

fn assert_contextual_help(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    (size, theme, locale): (Size<u32>, &Theme, DesktopLocale),
    title: &'static str,
    description: &'static str,
) -> iced::Point {
    let positions = geometry(ui, renderer);
    let label = positions
        .labels
        .iter()
        .rev()
        .find(|(label, _)| label == tr(locale, title))
        .unwrap_or_else(|| panic!("missing title {title}"))
        .1;
    let icon = iced::Point::new(label.x + label.width + 15.0, label.center_y());
    let frame = |ui: &mut Ui<'_>, renderer: &mut Renderer, suffix: &str, cursor| {
        capture(ui, renderer, theme, size, &format!("help-10-{locale:?}-{title}-{suffix}"), cursor)
    };
    let idle = frame(ui, renderer, "idle", mouse::Cursor::Unavailable);
    assert!(
        !positions.labels.iter().any(|(label, _)| label == tr(locale, description)),
        "{title}: explanation must be absent from the idle view"
    );
    let cursor = mouse::Cursor::Available(icon);
    assert!(
        send(ui, renderer, cursor, &[Event::Mouse(mouse::Event::CursorMoved { position: icon })])
            .is_empty()
    );
    assert!(idle != frame(ui, renderer, "hover", cursor), "{title}: expected a hover preview");
    assert!(click(ui, renderer, icon).is_empty());
    assert!(
        idle != frame(ui, renderer, "open", mouse::Cursor::Unavailable),
        "{title}: expected a pinned help card"
    );
    assert!(
        send(ui, renderer, mouse::Cursor::Unavailable, &[key(iced::keyboard::key::Named::Escape)])
            .is_empty()
    );
    assert!(
        idle == frame(ui, renderer, "closed", mouse::Cursor::Unavailable),
        "{title}: Escape must close only the help card"
    );
    icon
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_10_agent_policy_and_scope() {
    if !isolated("help_rollout_10_agent_policy_and_scope") {
        return;
    }
    use crate::app::components::system_settings_agents;
    use crate::app::message::settings::{AgentsMessage, SettingsMessage};
    use crate::app::state::{AGENT_DETAIL_SKILLS_TAB, AgentConfigTab, DelegateAgentSettingsEntry};

    let mut app = fixture();
    let mut renderer = renderer().await;
    app.agents_settings.config_ready = true;
    app.agents_settings.active_config_tab = AgentConfigTab::Agents;
    app.agents_settings.selected_agent = "fixture-agent".into();
    app.agents_settings.profile_modal =
        Some(DelegateAgentSettingsEntry::from_config("fixture-agent", None));
    app.agents_settings.active_detail_tab = AGENT_DETAIL_SKILLS_TAB.into();
    app.agents_settings.skill_directory = Some(PATH.into());
    app.agents_settings.skill_directory_scope = SkillsDirectoryScope::All;
    for (locale, theme, width) in [
        (DesktopLocale::SimplifiedChinese, Theme::Dark, 420),
        (DesktopLocale::English, Theme::Light, 640),
    ] {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        let size = Size::new(width, 800);
        let mut ui = scene(system_settings_agents::view(&app), &mut renderer, size);
        for (title, description) in [
            (
                "默认技能",
                "继承预览原有允许技能；固定空列表不预选技能。默认选择不会扩大严格访问范围。",
            ),
            ("技能访问策略", "严格限制保留原有允许范围；不限制允许会话临时选择。"),
            ("技能目录", "按加载顺序汇总全部技能来源。"),
        ] {
            assert_contextual_help(
                &mut ui,
                &mut renderer,
                (size, &theme, locale),
                title,
                description,
            );
        }
        assert!(geometry(&mut ui, &renderer).labels.iter().any(|(label, _)| label.contains(PATH)));
        assert!(matches!(
            press_label(&mut ui, &mut renderer, tr(locale, "严格限制")).as_slice(),
            [Message::Settings(SettingsMessage::Agents(AgentsMessage::SkillPolicyChanged(..)))]
        ));
        assert!(matches!(
            press_label(&mut ui, &mut renderer, tr(locale, "保存配置")).as_slice(),
            [Message::Settings(SettingsMessage::Agents(AgentsMessage::ProfileSave))]
        ));
    }
}
