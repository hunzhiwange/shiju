// Task 20 ownership: Host contracts: import_errors checks App update/toast/notification projection; layout_and_interaction composes host modal slots; preserves_values checks host editors and selector injection. Native rendering, locales and screenshot resources remain here.
//! Exercise task board forms through the actual Iced layout, event, and draw paths.
#![cfg(not(target_arch = "wasm32"))]

use crate::app::i18n::I18n;
use crate::app::message::TaskBoardMessage;
use crate::app::state::ToastKind;
use crate::app::{App, Message, Screen};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
    widget::{Id, Operation, operation},
};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use config_ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_34_CHILD";
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_34_SCREENSHOTS") {
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
struct Labels {
    text: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    scroll: Vec<(Rectangle, iced::Vector)>,
    selects: Vec<Rectangle>,
    select_menu: Option<Rectangle>,
}

impl Operation for Labels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, value: &str) {
        self.text.push((value.into(), bounds));
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
        id: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        translation: iced::Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        if id == Some(&Id::from("settings-select-menu")) {
            self.select_menu = Some(bounds);
        } else {
            self.scroll.push((bounds, translation));
        }
    }

    fn focusable(&mut self, _: Option<&Id>, bounds: Rectangle, _: &mut dyn operation::Focusable) {
        if bounds.height == 38.0 {
            self.selects.push(bounds);
        }
    }
}

fn labels(ui: &mut Ui<'_>, renderer: &Renderer) -> Labels {
    let mut labels = Labels::default();
    ui.operate(renderer, &mut labels);
    labels
}

impl Labels {
    fn bounds(&self, label: &str) -> Rectangle {
        self.text
            .iter()
            .find(|(value, _)| value == label)
            .unwrap_or_else(|| panic!("missing {label}: {:?}", self.text))
            .1
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_34_import_errors_are_visible() {
    if !isolated("ui_refresh_34_import_errors_are_visible") {
        return;
    }
    let mut app = fixture();
    app.navigation_tabs.screen = Screen::TaskBoard;
    app.desktop_locale = DesktopLocale::English;
    app.task_board_create_modal_open = true;
    app.task_board_is_import_mode = true;
    let mut renderer = renderer().await;
    let i18n = I18n::new(app.desktop_locale);
    let size = Size::new(960, 780);

    for (kind, content) in [("json", "{}"), ("csv", "unknown,header\n1,2")] {
        app.task_board_import_editor = iced::widget::text_editor::Content::with_text(content);
        let mut ui = scene(app.view(), &mut renderer, size);
        let submit = labels(&mut ui, &renderer).bounds(i18n.__("导入任务")).center();
        let messages = click(&mut ui, &mut renderer, submit);
        drop(ui);
        let [message @ Message::TaskBoard(TaskBoardMessage::ImportTasksSubmitted)] =
            messages.as_slice()
        else {
            panic!("expected one import submission");
        };
        let _ = app.update(message.clone());
        assert!(app.task_board_create_modal_open);
        assert_eq!(app.task_board_import_editor.text().trim_end(), content);
        let toast = app.active_toast.as_ref().expect("import error should appear immediately");
        assert!(matches!(toast.kind, ToastKind::Error));
        assert_eq!(app.notifications.last().unwrap().message, toast.message);
        let mut ui = scene(app.view(), &mut renderer, size);
        let feedback = labels(&mut ui, &renderer).bounds(&toast.message);
        assert!(feedback.x > size.width as f32 / 2.0);
        assert!(feedback.height > 0.0 && feedback.y + feedback.height < submit.y);
        assert!(matches!(
            click(&mut ui, &mut renderer, submit).as_slice(),
            [Message::TaskBoard(TaskBoardMessage::ImportTasksSubmitted)]
        ));
        capture(&mut ui, &mut renderer, &Theme::Dark, size, kind, mouse::Cursor::Unavailable);
    }
}
