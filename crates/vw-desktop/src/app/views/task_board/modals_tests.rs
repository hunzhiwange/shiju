#![allow(unused_imports, dead_code)]

//! 任务看板弹窗的测试，覆盖设置及上下文菜单。
//! 边界值 fixture 用于验证标签区分和分页。
use crate::app::state::TaskBoardSettingsModalTab;
use crate::app::task::{Task, TaskStatus};
use crate::app::{App, Message};
use iced::{Element, Length};
use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerDispatchModeDto;

fn test_app() -> App {
    let (app, _task) = App::new();
    app
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn labels_distinguish_wake_and_pool_promote_intervals_in_task_settings_modal() {
    let source = include_str!("modals.rs");

    assert!(source.contains("刷新间隔(秒)"));
    assert!(source.contains("执行唤醒间隔(秒)"));
    assert!(source.contains("任务池回推间隔(秒)"));
    assert!(source.contains("统一管理自动刷新、Agent Scheduler 唤醒与任务池回推节奏。"));
    assert!(!source.contains("任务调度间隔"));
    assert!(!source.contains("任务池调度间隔"));
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("modals_tests"));
}

#[test]
fn settings_modal_builds_refresh_tab_with_clamped_edges() {
    let mut app = test_app();
    app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::Refresh;
    app.task_board_settings.auto_refresh = false;
    app.task_board_settings.refresh_interval_seconds = 0;
    app.task_board_settings.scheduler_tick_interval_seconds = 0;
    app.task_board_settings.auto_promote_tick_interval_seconds = 0;
    app.task_board_settings.auto_promote_delay_seconds = 0;

    let low = super::build_task_settings_modal(&app);
    assert_eq!(low.as_widget().size().width, Length::Fixed(580.0));
    keep_element(low);

    app.task_board_settings.refresh_interval_seconds = 9_000;
    app.task_board_settings.scheduler_tick_interval_seconds = 99;
    app.task_board_settings.auto_promote_tick_interval_seconds = 9_000;
    app.task_board_settings.auto_promote_delay_seconds = 9_000;

    let high = super::build_task_settings_modal(&app);
    assert_eq!(high.as_widget().size().width, Length::Fixed(580.0));
    keep_element(high);
}

#[test]
fn settings_modal_builds_refresh_and_scheduling_tabs_with_midrange_values() {
    let mut app = test_app();
    app.task_board_settings.max_concurrent = 4;
    app.task_board_settings.refresh_interval_seconds = 30;
    app.task_board_settings.scheduler_tick_interval_seconds = 15;
    app.task_board_settings.auto_promote_tick_interval_seconds = 45;
    app.task_board_settings.auto_promote_delay_seconds = 120;
    app.task_board_settings.failed_retry_minutes = 30;
    app.task_board_settings.failed_retry_max_attempts = 3;
    app.task_board_settings.running_timeout_minutes = 90;
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 300;

    app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::Refresh;
    let refresh = super::build_task_settings_modal(&app);
    assert_eq!(refresh.as_widget().size().width, Length::Fixed(580.0));
    keep_element(refresh);

    app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::Scheduling;
    let scheduling = super::build_task_settings_modal(&app);
    assert_eq!(scheduling.as_widget().size().width, Length::Fixed(580.0));
    keep_element(scheduling);
}

#[test]
fn settings_modal_builds_scheduling_tab_with_clamped_edges() {
    let mut app = test_app();
    app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::Scheduling;
    app.task_board_settings.max_concurrent = 0;
    app.task_board_settings.failed_retry_minutes = 0;
    app.task_board_settings.failed_retry_max_attempts = 0;
    app.task_board_settings.running_timeout_minutes = 0;
    app.task_board_settings.recycle_worktree_on_task_finish = true;
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 0;

    let low = super::build_task_settings_modal(&app);
    assert_eq!(low.as_widget().size().width, Length::Fixed(580.0));
    keep_element(low);

    app.task_board_settings.max_concurrent = 99;
    app.task_board_settings.failed_retry_minutes = 9_999;
    app.task_board_settings.failed_retry_max_attempts = 99;
    app.task_board_settings.running_timeout_minutes = 9_999;
    app.task_board_settings.recycle_worktree_on_task_finish = false;
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 9_999;

    let high = super::build_task_settings_modal(&app);
    assert_eq!(high.as_widget().size().width, Length::Fixed(580.0));
    keep_element(high);
}

#[test]
fn task_settings_modal_no_longer_contains_capability_management_tab() {
    let mut app = test_app();
    app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::Refresh;

    let modal = super::build_task_settings_modal(&app);
    assert_eq!(modal.as_widget().size().width, Length::Fixed(580.0));
    keep_element(modal);

    let source = include_str!("modals.rs");
    let update_source = include_str!("../../message/task_board/update.rs");
    assert!(!source.contains("TaskBoardSettingsModalTab::Capabilities"));
    assert!(!source.contains("保存能力目录"));
    assert!(!update_source.contains("TaskBoardSettingsModalTab::Capabilities"));
}

#[test]
fn context_menu_builds_regular_and_failed_task_variants() {
    let mut regular = Task::new(10);
    regular.id = "regular-task".to_string();

    let regular_menu = super::build_context_menu(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        regular,
    );
    assert_eq!(regular_menu.as_widget().children().len(), 3);
    keep_element(regular_menu);

    let mut failed = Task::new(20);
    failed.id = "failed-task".to_string();
    failed.status = TaskStatus::Failed;

    let failed_menu = super::build_context_menu(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        failed,
    );
    assert_eq!(failed_menu.as_widget().children().len(), 4);
    keep_element(failed_menu);

    let mut running = Task::new(30);
    running.id = "running-task".to_string();
    running.status = TaskStatus::Running;
    let running_menu = super::build_context_menu(
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        running,
    );
    assert_eq!(running_menu.as_widget().children().len(), 3);
    keep_element(running_menu);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_peripheral_appearance_task_board() {
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::i18n::I18n;
    use crate::app::message::TaskBoardMessage;

    tab_appearance_tests::run_at_least(
        concat!(module_path!(), "::settings_tabs_peripheral_appearance_task_board"),
        420,
        test_app,
        |scene, app| {
            let _ = app.update(Message::TaskBoard(TaskBoardMessage::OpenSettingsModal));
            app.task_board_settings.agent_scheduler_dispatch_mode =
                AgentSchedulerDispatchModeDto::Local;
            let i18n = I18n::new(app.desktop_locale);
            let tabs = ["刷新与自动化", "并发与保护"].map(|label| i18n.__(label));
            scene.check_selected_segment(super::super::view(app), i18n.__("本地"));
            scene.save_preview(app, super::super::view(app), "task-settings");
            let message = scene.check_message(app, super::super::view(app), &tabs, 1, 14);
            assert!(matches!(
                message,
                Message::TaskBoard(TaskBoardMessage::SelectSettingsModalTab(
                    TaskBoardSettingsModalTab::Scheduling
                ))
            ));
            let _ = app.update(message);
            assert_eq!(app.task_board_settings_modal_tab, TaskBoardSettingsModalTab::Scheduling);
            assert_eq!(
                app.task_board_settings.agent_scheduler_dispatch_mode,
                AgentSchedulerDispatchModeDto::Local
            );
            let message = scene.click_message(super::super::view(app), tabs[0]);
            let _ = app.update(message);
            assert_eq!(app.task_board_settings_modal_tab, TaskBoardSettingsModalTab::Refresh);
        },
    )
    .await;
}

#[tokio::test]
async fn task_board_modal_form_controls_render_and_apply_settings() {
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::i18n::I18n;
    use crate::app::message::TaskBoardMessage;

    tab_appearance_tests::run_at_least(
        concat!(module_path!(), "::task_board_modal_form_controls_render_and_apply_settings"),
        420,
        test_app,
        |scene, app| {
            let _ = app.update(Message::TaskBoard(TaskBoardMessage::OpenSettingsModal));
            app.project_path = None;
            let i18n = I18n::new(app.desktop_locale);
            let view = super::super::view;

            app.task_board_settings.refresh_interval_seconds = 37;
            let message = scene.edit_modal_input(view(app), "37", "45");
            assert!(matches!(
                message,
                Message::TaskBoard(TaskBoardMessage::SetRefreshIntervalSeconds(45))
            ));
            let _ = app.update(message);
            assert_eq!(app.task_board_settings.refresh_interval_seconds, 45);

            let message = scene.click_message(view(app), i18n.__("并发与保护"));
            let _ = app.update(message);
            assert_eq!(app.task_board_settings_modal_tab, TaskBoardSettingsModalTab::Scheduling);
            app.task_board_settings.max_concurrent = 4;
            let message = scene.edit_modal_input(view(app), "4", "99");
            assert!(matches!(message, Message::TaskBoard(TaskBoardMessage::SetMaxConcurrent(10))));
            let _ = app.update(message);
            assert_eq!(app.task_board_settings.max_concurrent, 10);
            scene.check_modal_scroll(view(app), i18n.__("任务设置"), None);
        },
    )
    .await;
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::views::task_board) mod help_rollout {
    use super::*;
    use crate::app::i18n::I18n;
    use iced::advanced::{
        clipboard,
        renderer::{Headless, Style},
        widget::{Id, Operation, operation},
    };
    use iced::keyboard::{self, Key, key::Named};
    use iced::{Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

    pub(crate) type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

    pub(crate) fn isolated_fixture(module: &str, test: &str) -> Option<App> {
        const CHILD: &str = "SHIJU_HELP_ROLLOUT_24_CHILD";
        if std::env::var_os(CHILD).is_some() {
            assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
            return Some(test_app());
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
            .args(["--exact", &format!("{}::{test}", module.split_once("::").unwrap().1)])
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
        None
    }

    pub(crate) const LAYOUTS: [(DesktopLocale, Theme, u32); 3] = [
        (DesktopLocale::SimplifiedChinese, Theme::Dark, 420),
        (DesktopLocale::English, Theme::Light, 420),
        (DesktopLocale::TraditionalChinese, Theme::Dark, 640),
    ];

    pub(crate) async fn renderer() -> Renderer {
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

    pub(crate) fn scene<'a>(
        view: Element<'a, Message>,
        renderer: &mut Renderer,
        size: Size<u32>,
    ) -> Ui<'a> {
        UserInterface::build(
            view,
            Size::new(size.width as f32, size.height as f32),
            Cache::new(),
            renderer,
        )
    }

    pub(crate) fn send(
        ui: &mut Ui<'_>,
        renderer: &mut Renderer,
        point: Point,
        events: &[Event],
    ) -> Vec<Message> {
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

    pub(crate) fn click(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Vec<Message> {
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

    pub(crate) fn key(
        ui: &mut Ui<'_>,
        renderer: &mut Renderer,
        key: Key,
        value: Option<&str>,
    ) -> Vec<Message> {
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
                text: value.map(Into::into),
                repeat: false,
            })],
        )
    }

    #[derive(Default, Debug)]
    pub(crate) struct Geometry {
        pub(crate) labels: Vec<(String, Rectangle)>,
        pub(crate) inputs: Vec<(String, Rectangle)>,
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
            input: &mut dyn operation::TextInput,
        ) {
            self.inputs.push((input.text().into(), bounds));
        }
    }

    pub(crate) fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
        let mut result = Geometry::default();
        ui.operate(renderer, &mut result);
        result
    }

    pub(crate) fn locate(entries: &[(String, Rectangle)], value: &str) -> Rectangle {
        entries
            .iter()
            .rfind(|(label, _)| label == value)
            .unwrap_or_else(|| panic!("missing {value:?} in {entries:?}"))
            .1
    }

    fn frame(
        ui: &mut Ui<'_>,
        renderer: &mut Renderer,
        theme: &Theme,
        size: Size<u32>,
        point: Point,
    ) -> Vec<u8> {
        assert!(
            send(
                ui,
                renderer,
                point,
                &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))]
            )
            .is_empty()
        );
        ui.draw(
            renderer,
            theme,
            &Style { text_color: theme.palette().text },
            mouse::Cursor::Available(point),
        );
        renderer.screenshot(size, 1.0, theme.palette().background)
    }

    pub(crate) fn check_help(
        ui: &mut Ui<'_>,
        renderer: &mut Renderer,
        app: &App,
        size: Size<u32>,
        title: &str,
        description: &str,
        artifact: &str,
    ) -> Point {
        let i18n = I18n::new(app.desktop_locale);
        let title = i18n.__(title);
        let description = i18n.__(description);
        let labels = geometry(ui, renderer).labels;
        assert!(
            labels.iter().all(|(label, _)| label != description),
            "{title}: static explanation is still inline: {description}"
        );
        let label = locate(&labels, title);
        let icon = Point::new(label.x + label.width + 15.0, label.center_y());
        assert!(check_help_at(ui, renderer, app, size, title, icon, artifact).is_empty());
        icon
    }

    pub(crate) fn check_help_at(
        ui: &mut Ui<'_>,
        renderer: &mut Renderer,
        app: &App,
        size: Size<u32>,
        title: &str,
        icon: Point,
        artifact: &str,
    ) -> Vec<Message> {
        assert!(
            Rectangle::with_size(Size::new(size.width as f32, size.height as f32)).contains(icon),
            "{title}: help anchor outside viewport: {icon:?}"
        );
        let idle = frame(ui, renderer, &app.app_theme, size, Point::ORIGIN);
        assert!(
            send(ui, renderer, icon, &[Event::Mouse(mouse::Event::CursorMoved { position: icon })])
                .is_empty()
        );
        let hover = frame(ui, renderer, &app.app_theme, size, icon);
        assert!(idle != hover, "{title}: hovering must open the help card");
        let click_messages = click(ui, renderer, icon);
        let pinned = frame(ui, renderer, &app.app_theme, size, Point::ORIGIN);
        assert!(idle != pinned, "{title}: clicking must pin the help card");
        assert!(key(ui, renderer, Key::Named(Named::Escape), None).is_empty());
        let closed = frame(ui, renderer, &app.app_theme, size, Point::ORIGIN);
        assert!(idle == closed, "{title}: Escape must close only the help card");

        if let Some(directory) = std::env::var_os("SHIJU_HELP_ROLLOUT_24_SCREENSHOTS") {
            for (state, pixels) in [("idle", idle), ("open", pinned)] {
                ::image::save_buffer(
                    std::path::PathBuf::from(&directory)
                        .join(format!("{artifact}-{:?}-{state}.png", app.desktop_locale)),
                    &pixels,
                    size.width,
                    size.height,
                    ::image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
        click_messages
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn help_rollout_24_renders_contextual_help() {
        use crate::app::message::TaskBoardMessage;

        let Some(mut app) =
            isolated_fixture(module_path!(), "help_rollout_24_renders_contextual_help")
        else {
            return;
        };
        app.task_board_settings_modal_open = true;
        app.task_board_settings.refresh_interval_seconds = 37;
        app.task_board_settings.max_concurrent = 7;
        let mut renderer = renderer().await;
        for (locale, theme, width) in LAYOUTS {
            app.desktop_locale = locale;
            app.app_theme = theme;
            let i18n = I18n::new(locale);
            let size = Size::new(width, 600);
            for (tab, title, description, value, artifact) in [
                (
                    TaskBoardSettingsModalTab::Refresh,
                    "刷新与自动化",
                    "统一管理自动刷新、Agent Scheduler 唤醒与任务池回推节奏。",
                    "37",
                    "refresh",
                ),
                (
                    TaskBoardSettingsModalTab::Scheduling,
                    "并发与保护",
                    "控制任务吞吐、失败重试、超时与 worktree 回收策略。",
                    "7",
                    "scheduling",
                ),
            ] {
                app.task_board_settings_modal_tab = tab;
                let mut ui = scene(super::super::super::view(&app), &mut renderer, size);
                check_help(&mut ui, &mut renderer, &app, size, title, description, artifact);
                let geometry = geometry(&mut ui, &renderer);
                let notice =
                    locate(&geometry.labels, i18n.__("数值会立即作用于当前项目任务看板。"));
                let input = locate(&geometry.inputs, value);
                assert!(input.center_x() < width as f32);
                let plus = geometry
                    .labels
                    .iter()
                    .find(|(label, bounds)| {
                        label == "+"
                            && (bounds.center_y() - input.center_y()).abs() < 1.0
                            && bounds.x > input.x
                    })
                    .expect("the current value has an increment button")
                    .1;
                let messages = click(&mut ui, &mut renderer, plus.center());
                assert!(matches!(
                    (tab, messages.as_slice()),
                    (
                        TaskBoardSettingsModalTab::Refresh,
                        [Message::TaskBoard(TaskBoardMessage::SetRefreshIntervalSeconds(42))]
                    ) | (
                        TaskBoardSettingsModalTab::Scheduling,
                        [Message::TaskBoard(TaskBoardMessage::SetMaxConcurrent(8))]
                    )
                ));
                let close =
                    Point::new(width.min(604) as f32 / 2.0 + width as f32 / 2.0 - 46.0, notice.y);
                assert!(matches!(
                    click(&mut ui, &mut renderer, close).as_slice(),
                    [Message::TaskBoard(TaskBoardMessage::CloseSettingsModal)]
                ));
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn help_rollout_24_menu_uses_static_explanation() {
        let Some(mut app) =
            isolated_fixture(module_path!(), "help_rollout_24_menu_uses_static_explanation")
        else {
            return;
        };
        app.desktop_locale = DesktopLocale::English;
        app.task_board_settings_modal_open = true;
        app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::Refresh;
        app.task_board_settings.refresh_interval_seconds = 1973;
        let i18n = I18n::new(app.desktop_locale);
        let description = "统一管理自动刷新、Agent Scheduler 唤醒与任务池回推节奏。";
        let mut renderer = renderer().await;
        let size = Size::new(640, 600);
        let mut ui = scene(super::super::super::view(&app), &mut renderer, size);
        let icon =
            check_help(&mut ui, &mut renderer, &app, size, "刷新与自动化", description, "menu");
        assert!(click(&mut ui, &mut renderer, icon).is_empty());
        let body = Point::new(icon.x + 29.0, icon.y + 8.0);
        let line_height = iced::advanced::text::LineHeight::default().to_absolute(12.0.into());
        let item_height = f32::from(line_height) + 10.0;
        for index in [0, 3] {
            assert!(
                send(
                    &mut ui,
                    &mut renderer,
                    body,
                    &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))]
                )
                .is_empty()
            );
            let item = Point::new(body.x + 24.0, body.y + 3.0 + item_height * (index as f32 + 0.5));
            let messages = click(&mut ui, &mut renderer, item);
            match (index, messages.as_slice()) {
                (0, [Message::CopyCode(value)]) => assert_eq!(value, i18n.__(description)),
                (3, [Message::View(crate::app::message::ViewMessage::OpenUrlExternal(url))]) => {
                    assert_eq!(
                        url,
                        &format!(
                            "https://www.google.com/search?q={}",
                            urlencoding::encode(i18n.__(description))
                        )
                    );
                }
                _ => panic!("unexpected help action: {messages:?}"),
            }
        }
    }
}
