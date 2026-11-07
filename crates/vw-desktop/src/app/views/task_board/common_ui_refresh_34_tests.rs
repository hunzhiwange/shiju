//! Exercise task board forms through the actual Iced layout, event, and draw paths.
#![cfg(not(target_arch = "wasm32"))]

use crate::app::i18n::I18n;
use crate::app::message::TaskBoardMessage;
use crate::app::state::{TaskBoardSettingsModalTab, ToastKind};
use crate::app::{App, Message, Screen};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
    widget::{Id, Operation, operation},
};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

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
    app.screen = Screen::TaskBoard;
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

fn scroll(ui: &mut Ui<'_>, renderer: &mut Renderer, bounds: Rectangle, y: f32) {
    assert!(
        send(
            ui,
            renderer,
            mouse::Cursor::Available(Point::new(bounds.x + bounds.width - 7.0, bounds.center_y())),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y },
            })]
        )
        .is_empty()
    );
}

fn press_label(ui: &mut Ui<'_>, renderer: &mut Renderer, title: &str) -> Vec<Message> {
    let point = labels(ui, renderer).bounds(title).center();
    click(ui, renderer, point)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_34_layout_and_interaction() {
    if !isolated("ui_refresh_34_layout_and_interaction") {
        return;
    }
    let mut app = fixture();
    app.task_board_worktree_pixel_office = false;
    app.task_board_settings.max_concurrent = 4;
    let mut renderer = renderer().await;

    for (locale, width, language) in [
        (DesktopLocale::SimplifiedChinese, 420, "zh-cn"),
        (DesktopLocale::TraditionalChinese, 640, "zh-tw"),
        (DesktopLocale::English, 960, "en"),
    ] {
        app.desktop_locale = locale;
        let i18n = I18n::new(locale);
        let size = Size::new(width, 480);
        for (theme, mode) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
            app.task_board_settings_modal_open = true;
            app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::Refresh;
            let mut ui = scene(super::super::view(&app), &mut renderer, size);
            let initial = labels(&mut ui, &renderer);
            let title = initial.bounds(i18n.__("任务设置"));
            let tab = initial.bounds(i18n.__("并发与保护"));
            let (viewport, _) = initial.scroll[0];
            assert!(title.y < viewport.y);
            for (value, bounds) in initial.text.iter().chain(&initial.inputs) {
                assert!(bounds.width > 0.0 && bounds.x >= 0.0, "{value}: {bounds:?}");
                assert!(bounds.x + bounds.width <= width as f32 + 0.1, "{value}: {bounds:?}");
            }
            capture(
                &mut ui,
                &mut renderer,
                &theme,
                size,
                &format!("{language}-{mode}-refresh"),
                mouse::Cursor::Unavailable,
            );
            scroll(&mut ui, &mut renderer, viewport, -160.0);
            let scrolled = labels(&mut ui, &renderer);
            assert!(scrolled.scroll[0].1.y > 0.0);
            assert_eq!(scrolled.bounds(i18n.__("任务设置")), title);
            assert_eq!(scrolled.bounds(i18n.__("并发与保护")), tab);
            assert!(key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).is_empty());
            assert!(matches!(
                click(&mut ui, &mut renderer, tab.center()).as_slice(),
                [Message::TaskBoard(TaskBoardMessage::SelectSettingsModalTab(
                    TaskBoardSettingsModalTab::Scheduling
                ))]
            ));
            drop(ui);

            app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::Scheduling;
            let mut ui = scene(super::super::view(&app), &mut renderer, size);
            let initial = labels(&mut ui, &renderer);
            let plus = initial.bounds("+");
            assert!((initial.inputs[0].1.center_y() - plus.center_y()).abs() < 0.5);
            capture(
                &mut ui,
                &mut renderer,
                &theme,
                size,
                &format!("{language}-{mode}-scheduling"),
                mouse::Cursor::Unavailable,
            );
            assert!(matches!(
                click(&mut ui, &mut renderer, plus.center()).as_slice(),
                [Message::TaskBoard(TaskBoardMessage::SetMaxConcurrent(5))]
            ));
            assert!(click(&mut ui, &mut renderer, Point::new(1.0, 1.0)).is_empty());
            // Shared close button uses a 28px target at the right of the modal heading.
            let close = Point::new(
                (width as f32 + (width as f32 - 24.0).min(580.0)) / 2.0 - 38.0,
                initial.bounds(i18n.__("任务设置")).center_y() + 10.0,
            );
            assert!(matches!(
                click(&mut ui, &mut renderer, close).as_slice(),
                [Message::TaskBoard(TaskBoardMessage::CloseSettingsModal)]
            ));
            drop(ui);

            app.task_board_settings_modal_open = false;
            app.task_board_create_modal_open = true;
            for (import, screen, action) in
                [(false, "single", "创建任务"), (true, "import", "导入任务")]
            {
                app.task_board_is_import_mode = import;
                let mut ui = scene(super::super::view(&app), &mut renderer, size);
                let geometry = labels(&mut ui, &renderer);
                let submit = geometry.bounds(i18n.__(action));
                let viewport = geometry.scroll.last().unwrap().0;
                assert!(viewport.y + viewport.height < submit.y);
                assert!(submit.x + submit.width <= width as f32 && submit.width > 0.0);
                capture(
                    &mut ui,
                    &mut renderer,
                    &theme,
                    size,
                    &format!("{language}-{mode}-{screen}"),
                    mouse::Cursor::Unavailable,
                );
            }
            app.task_board_create_modal_open = false;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_34_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_34_preserves_values_and_disabled_states") {
        return;
    }
    let mut app = fixture();
    app.project_path = None;
    app.task_board_worktree_pixel_office = false;
    app.desktop_locale = DesktopLocale::English;
    app.task_board_create_modal_open = true;
    app.task_board_draft.priority = "42".into();
    app.task_board_draft.model = "fixture/example-model".into();
    app.task_board_draft.acp_agent = Some("example-executor".into());
    app.acp_agents = vec!["example-executor".into(), "another-executor".into()];
    let prompt = "示例提示・測試文字・fixture";
    app.task_board_draft.prompt = prompt.into();
    app.task_board_prompt_editor = iced::widget::text_editor::Content::with_text(prompt);
    let import = "priority,prompt\n7,draft";
    app.task_board_import_editor = iced::widget::text_editor::Content::with_text(import);
    let i18n = I18n::new(app.desktop_locale);
    let mut renderer = renderer().await;
    let size = Size::new(960, 640);
    let theme = Theme::Dark;

    let mut ui = scene(super::super::view(&app), &mut renderer, size);
    let before = labels(&mut ui, &renderer);
    capture(&mut ui, &mut renderer, &theme, size, "single", mouse::Cursor::Unavailable);
    let submit = before.bounds(i18n.__("创建任务"));
    let viewport = before.scroll.last().unwrap().0;
    scroll(&mut ui, &mut renderer, viewport, -210.0);
    let after = labels(&mut ui, &renderer);
    assert!(after.scroll.last().unwrap().1.y > 0.0);
    assert_eq!(submit, after.bounds(i18n.__("创建任务")));
    let priority = after.inputs.iter().find(|(value, _)| value == "42").unwrap().1;
    let point = Point::new(
        priority.x + priority.width - 9.0,
        priority.center_y() - after.scroll.last().unwrap().1.y,
    );
    assert!(click(&mut ui, &mut renderer, point).is_empty());
    assert!(matches!(
        key(&mut ui, &mut renderer, Key::Character("7".into()), Some("7")).as_slice(),
        [Message::TaskBoard(TaskBoardMessage::UpdateDraftPriority(value))] if value == "427"
    ));
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).is_empty());
    assert!(matches!(
        click(&mut ui, &mut renderer, submit.center()).as_slice(),
        [Message::TaskBoard(TaskBoardMessage::CreateTaskSubmitted)]
    ));
    scroll(&mut ui, &mut renderer, viewport, 900.0);
    let messages = press_label(&mut ui, &mut renderer, i18n.__("批量导入"));
    drop(ui);
    assert!(matches!(
        messages.as_slice(),
        [Message::TaskBoard(TaskBoardMessage::ToggleImportMode(true))]
    ));
    let _ = app.update(messages.into_iter().next().unwrap());

    let mut ui = scene(super::super::view(&app), &mut renderer, size);
    capture(&mut ui, &mut renderer, &theme, size, "import", mouse::Cursor::Unavailable);
    let select = labels(&mut ui, &renderer).selects[0];
    assert!(click(&mut ui, &mut renderer, select.center()).is_empty());
    let menu = labels(&mut ui, &renderer).select_menu.expect("example menu open");
    assert!(menu.x >= 0.0 && menu.x + menu.width <= size.width as f32);
    assert!(menu.y >= 0.0 && menu.y + menu.height <= size.height as f32);
    assert_eq!(menu.height, 3.0 * 36.0 + 2.0 * 2.0);
    let json = Point::new(menu.center_x(), menu.y + 18.0);
    let csv = Point::new(menu.center_x(), menu.y + 56.0);
    let idle =
        capture(&mut ui, &mut renderer, &theme, size, "examples", mouse::Cursor::Unavailable);
    for (point, name) in [(json, "example-json"), (csv, "example-csv")] {
        let cursor = mouse::Cursor::Available(point);
        assert!(
            send(
                &mut ui,
                &mut renderer,
                cursor,
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
            )
            .is_empty()
        );
        assert!(capture(&mut ui, &mut renderer, &theme, size, name, cursor) != idle);
    }
    assert!(
        send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Unavailable,
            &[Event::Mouse(mouse::Event::CursorLeft)],
        )
        .is_empty()
    );
    assert!(
        capture(&mut ui, &mut renderer, &theme, size, "examples-idle", mouse::Cursor::Unavailable)
            == idle
    );
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Escape), None).is_empty());
    assert!(click(&mut ui, &mut renderer, select.center()).is_empty());
    assert!(matches!(
        click(&mut ui, &mut renderer, csv).as_slice(),
        [Message::TaskBoard(TaskBoardMessage::InsertDemoData(value))] if value == "CSV 示例"
    ));
    assert!(click(&mut ui, &mut renderer, select.center()).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::End), None).is_empty());
    assert!(matches!(
        key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).as_slice(),
        [Message::TaskBoard(TaskBoardMessage::InsertDemoData(value))] if value == "TSV 示例"
    ));
    let messages = press_label(&mut ui, &mut renderer, i18n.__("单个任务"));
    drop(ui);
    assert!(matches!(
        messages.as_slice(),
        [Message::TaskBoard(TaskBoardMessage::ToggleImportMode(false))]
    ));
    let _ = app.update(messages.into_iter().next().unwrap());
    assert_eq!(app.task_board_draft.priority, "42");
    assert_eq!(app.task_board_draft.model, "fixture/example-model");
    assert_eq!(app.task_board_draft.acp_agent.as_deref(), Some("example-executor"));
    assert_eq!(app.task_board_draft.prompt, prompt);
    assert_eq!(app.task_board_prompt_editor.text().trim_end(), prompt);
    assert_eq!(app.task_board_import_editor.text().trim_end(), import);

    let mut ui = scene(super::super::view(&app), &mut renderer, size);
    assert!(matches!(
        press_label(&mut ui, &mut renderer, i18n.__("取消")).as_slice(),
        [Message::TaskBoard(TaskBoardMessage::CreateTaskCancelled)]
    ));
}
