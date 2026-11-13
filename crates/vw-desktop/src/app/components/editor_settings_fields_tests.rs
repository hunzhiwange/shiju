use crate::app::message::editor::{self, EditorMessage};
use crate::app::{App, Message};
use iced::advanced::widget::{Id, Operation, operation};
use iced::advanced::{clipboard, renderer::Headless};
use iced::widget::{container, text};
use iced::{Element, Event, Font, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::GatewayClientSystemSettingsConfig;
type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_ECP03_CHILD";
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
        .on_press(Message::Editor(EditorMessage::ClosePopover)),
    )
    .width(Length::Fill)
    .height(Length::Fill);
    crate::app::components::preview_panel::build_settings_overlay(app, base.into())
}

#[tokio::test]
async fn ecp03_settings_surfaces_apply_identical_metrics_and_theme() {
    if !isolated("ecp03_settings_surfaces_apply_identical_metrics_and_theme") {
        return;
    }
    let mut app = fixture();
    app.current_font_size = 16.0;
    app.current_line_height = 26.0;
    app.auto_adjust_line_height = false;
    app.editor_follow_system_theme = true;
    app.app_theme = Theme::Dark;
    app.editor_theme = Theme::Light;
    let mut renderer = renderer().await;
    for local in [true, false] {
        let element =
            if local { preview(&app) } else { super::system_settings_editor::content(&app) };
        let mut ui = scene(element, &mut renderer, Size::new(640, 1200));
        let controls = bounds(&mut ui, &renderer);
        assert_eq!(controls.focus.len(), 1, "manual theme disabled; language remains available");
        let badge = position(&controls.labels, "26.0px");
        let end = Point::new(badge.x - 35.0, badge.center_y());
        let messages = click(&mut ui, &mut renderer, end);
        assert!(
            messages
                .iter()
                .any(|m| matches!(m, Message::Editor(EditorMessage::LineHeightPreview(_))))
        );
        drop(ui);
        for message in messages {
            if let Message::Editor(message) = message {
                let _task = editor::update(&mut app, message);
            }
        }
        assert!(app.current_line_height > 50.0);
        let _task = editor::update(&mut app, EditorMessage::ToggleFollowSystemTheme(false));
        let _task = editor::update(&mut app, EditorMessage::ThemeChanged(Theme::Dark));
        assert_eq!(app.effective_editor_theme(), Theme::Dark);
        app.current_line_height = 26.0;
        app.editor_follow_system_theme = true;
    }
}
#[tokio::test]
async fn ecp03_closing_modal_commits_pending_metrics() {
    if !isolated("ecp03_closing_modal_commits_pending_metrics") {
        return;
    }
    let mut app = fixture();
    let _task = editor::update(&mut app, EditorMessage::ToggleSettings);
    let _task = editor::update(&mut app, EditorMessage::FontSizePreview(22.0));
    let _task = editor::update(&mut app, EditorMessage::LineHeightPreview(60.0));
    let _task = editor::update(&mut app, EditorMessage::ToggleSettings);
    assert!(!app.show_preview_settings);
    assert_eq!(app.current_font_size, 22.0);
    assert_eq!(app.current_line_height, 60.0);
    assert_eq!(app.git_copy_modal_code_editor.font_size(), 22.0);
    assert_eq!(app.git_copy_modal_code_editor.line_height(), 60.0);
}
