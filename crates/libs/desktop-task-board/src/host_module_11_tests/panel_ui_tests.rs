//! Standalone reasoning, executor and model selector rendering moved to desktop-task-board::desktop_split_20_tests.
#![cfg(not(target_arch = "wasm32"))]

use super::panel_fixture::{PanelFixture, build_task_panel};
use crate::message::TaskBoardDetailTab;
use crate::message::TaskBoardMessage;
use crate::panel::PanelMessage as Message;
use config_ui::{DesktopLocale, GatewayClientSystemSettingsConfig};
use desktop_i18n::I18n;
use desktop_task_model::{SubTask, Task, TaskStatus};
use iced::advanced::{
    clipboard,
    renderer::{Headless, Style},
    widget::{Id, Operation, operation},
};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_35_CHILD";
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

fn fixture() -> PanelFixture {
    PanelFixture::default()
}

async fn renderer() -> Renderer {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_35_SCREENSHOTS") {
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
    scrolls: Vec<(Option<Id>, Rectangle, iced::Vector)>,
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
        self.scrolls.push((id.cloned(), bounds, translation));
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
    fn input(&self, value: &str) -> Rectangle {
        self.inputs.iter().find(|(text, _)| text == value).expect("fixture input").1
    }

    fn scroll(&self, id: &str) -> (Rectangle, iced::Vector) {
        let id = Id::from(id.to_owned());
        self.scrolls
            .iter()
            .find(|(actual, _, _)| actual.as_ref() == Some(&id))
            .map(|(_, bounds, offset)| (*bounds, *offset))
            .expect("fixture scroll")
    }
}

fn wheel(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point, y: f32) -> Vec<Message> {
    send(
        ui,
        renderer,
        mouse::Cursor::Available(point),
        &[Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y },
        })],
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_35_layout_and_interaction() {
    if !isolated("ui_refresh_35_layout_and_interaction") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let mut task = Task {
        id: "fixture-任务".into(),
        status: TaskStatus::Failed,
        last_error: Some("测试失败 / fixture failure".into()),
        selected_worktree_path: Some(format!("/fixture/{}", "長い作業ディレクトリ/".repeat(7))),
        ..Task::default()
    };
    task.subtasks.push(SubTask::new("子任务 / 子任務 / Subtask".into()));
    app.task_board_viewing_logs = Some(task);
    app.task_board_new_subtask_content = "新增 / New fixture".into();
    app.task_board_draft.priority = "42".into();
    app.task_board_draft.acp_agent = Some("codex".into());
    app.acp_agents = (0..16).map(|index| format!("fixture-{index}")).collect();
    app.task_board_draft.model = format!("fixture/{}", "很长的模型標籤".repeat(8));
    app.task_board_logs_editor = iced::widget::text_editor::Content::with_text(
        &"任务日志 / 任務記錄 / Task log\n/fixture/工作區/日志.txt\n".repeat(80),
    );
    for (locale, width, language) in [
        (DesktopLocale::SimplifiedChinese, 420, "zh-cn"),
        (DesktopLocale::TraditionalChinese, 640, "zh-tw"),
        (DesktopLocale::English, 960, "en"),
    ] {
        app.desktop_locale = locale;
        let size = Size::new(width, 640);
        for (theme, mode) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
            app.task_board_detail_tab = TaskBoardDetailTab::Basic;
            let mut ui = scene(build_task_panel(&app, true), &mut renderer, size);
            let geometry = labels(&mut ui, &renderer);
            let i18n = I18n::new(locale);
            assert!(geometry.bounds(i18n.__("任务详情")).width > 0.0);
            assert!(geometry.inputs.iter().any(|(value, _)| value == "42"));
            capture(
                &mut ui,
                &mut renderer,
                &theme,
                size,
                &format!("{language}-{mode}-detail"),
                mouse::Cursor::Unavailable,
            );
            let title = geometry.bounds(i18n.__("任务详情"));
            let submit = geometry.bounds(i18n.__("保存修改"));
            let tabs = geometry.bounds(i18n.__("日志"));
            let viewport = geometry.scrolls[0].1;
            let gutter = Point::new(viewport.x + viewport.width - 7.0, viewport.center_y());
            assert!(viewport.y + viewport.height < submit.y);
            assert!(wheel(&mut ui, &mut renderer, gutter, -1600.0).is_empty());
            let scrolled = labels(&mut ui, &renderer);
            let offset = scrolled.scrolls[0].2;
            assert!(offset.y > 0.0);
            assert_eq!(scrolled.bounds(i18n.__("任务详情")), title);
            assert_eq!(scrolled.bounds(i18n.__("保存修改")), submit);
            let input = scrolled.input("新增 / New fixture");
            let add = scrolled.bounds(i18n.__("添加"));
            assert!((input.center_y() - add.center_y()).abs() < 0.5);
            capture(
                &mut ui,
                &mut renderer,
                &theme,
                size,
                &format!("{language}-{mode}-subtasks"),
                mouse::Cursor::Unavailable,
            );
            let up = scrolled.bounds("↑").center() - offset;
            assert!(viewport.contains(up));
            assert!(click(&mut ui, &mut renderer, up).is_empty());
            assert!(matches!(
                click(&mut ui, &mut renderer, add.center() - offset).as_slice(),
                [Message::TaskBoard(TaskBoardMessage::AddSubTask { task_id, content })]
                    if task_id == "fixture-任务" && content == "新增 / New fixture"
            ));
            assert!(matches!(
                click(&mut ui, &mut renderer, submit.center()).as_slice(),
                [Message::TaskBoard(TaskBoardMessage::SaveEditingTask)]
            ));
            assert!(matches!(
                click(&mut ui, &mut renderer, scrolled.bounds(i18n.__("取消")).center()).as_slice(),
                [Message::TaskBoard(TaskBoardMessage::CloseTaskLogs)]
            ));
            assert!(matches!(
                click(&mut ui, &mut renderer, tabs.center()).as_slice(),
                [Message::TaskBoard(TaskBoardMessage::SelectDetailTab(TaskBoardDetailTab::Logs))]
            ));
            if locale == DesktopLocale::English && mode == "dark" {
                let cache = ui.into_cache();
                app.task_board_executor_popover = true;
                let mut ui = UserInterface::build(
                    build_task_panel(&app, true),
                    Size::new(width as f32, 640.0),
                    cache,
                    &mut renderer,
                );
                capture(
                    &mut ui,
                    &mut renderer,
                    &theme,
                    size,
                    "detail-executor",
                    mouse::Cursor::Unavailable,
                );
                let (menu, _) =
                    labels(&mut ui, &renderer).scroll("task_board_executor_selector_scroll");
                assert!(menu.height > 0.0 && menu.y >= viewport.y);
                assert!(menu.y + menu.height <= viewport.y + viewport.height);
                assert!(wheel(&mut ui, &mut renderer, menu.center(), -120.0).is_empty());
                let after_wheel = labels(&mut ui, &renderer);
                assert!(after_wheel.scroll("task_board_executor_selector_scroll").1.y > 0.0);
                assert_eq!(after_wheel.scrolls[0].2, offset);
                drop(ui);
                app.task_board_executor_popover = false;
            } else {
                drop(ui);
            }
            app.task_board_detail_tab = TaskBoardDetailTab::Logs;
            let mut ui = scene(build_task_panel(&app, true), &mut renderer, size);
            capture(
                &mut ui,
                &mut renderer,
                &theme,
                size,
                &format!("{language}-{mode}-logs"),
                mouse::Cursor::Unavailable,
            );
            assert!(matches!(
                wheel(&mut ui, &mut renderer, Point::new(260.0, 300.0), -80.0).as_slice(),
                [Message::TaskBoard(TaskBoardMessage::LogsViewerEditorWheelScrolled { .. })]
            ));
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_35_sidebar_shields_the_board() {
    if !isolated("ui_refresh_35_sidebar_shields_the_board") {
        return;
    }
    let mut app = fixture();
    app.desktop_locale = DesktopLocale::English;
    app.task_board_worktree_pixel_office = false;
    app.task_board_detail_tab = TaskBoardDetailTab::Basic;
    app.task_board_viewing_logs = Some(Task { id: "wheel-fixture".into(), ..Task::default() });
    let mut renderer = renderer().await;
    let size = Size::new(960, 640);
    let mut ui = scene(super::panel_fixture::page(&app), &mut renderer, size);
    let initial = labels(&mut ui, &renderer);
    let inside = Point::new(670.0, 600.0);
    let outside = Point::new(100.0, 600.0);
    let board = initial
        .scrolls
        .iter()
        .position(|(_, bounds, _)| bounds.contains(inside) && bounds.contains(outside))
        .expect("board behind the sidebar");
    let start = initial.scrolls[board].2;
    let event = Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Pixels { x: -120.0, y: 0.0 },
    });
    assert!(
        send(
            &mut ui,
            &mut renderer,
            mouse::Cursor::Available(inside),
            std::slice::from_ref(&event),
        )
        .is_empty()
    );
    assert_eq!(
        labels(&mut ui, &renderer).scrolls[board].2,
        start,
        "wheel reached the board behind the sidebar"
    );
    assert!(click(&mut ui, &mut renderer, inside).is_empty());
    assert!(send(&mut ui, &mut renderer, mouse::Cursor::Available(outside), &[event]).is_empty());
    assert!(labels(&mut ui, &renderer).scrolls[board].2.x > start.x);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Dark,
        size,
        "board-sidebar",
        mouse::Cursor::Unavailable,
    );
}
