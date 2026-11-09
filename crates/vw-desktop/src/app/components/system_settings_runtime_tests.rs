//! 验证本机与容器运行时选项、推理级别和条件设置区块。
//!
//! 本机与容器配置分别构造，确认视图可构建且选项标签稳定。

use super::*;
use crate::app::App;
use iced::Element;

fn test_app() -> App {
    App::new().0
}

#[test]
fn system_settings_runtime_tests_are_wired() {
    assert!(module_path!().contains("system_settings_runtime_tests"));
}

#[test]
fn labeled_option_displays_label_not_value() {
    let option = LabeledOption { value: "xhigh", label: "超高" };

    assert_eq!(option.to_string(), "超高");
}

#[test]
fn view_builds_native_runtime_settings() {
    let mut app = test_app();
    app.runtime_settings.kind = "native".to_string();
    let _: Element<'_, Message> = view(&app);

    app.runtime_settings.reasoning_enabled_input = "unknown".to_string();
    app.runtime_settings.reasoning_level_input = "unknown".to_string();
    let _: Element<'_, Message> = view(&app);
}

#[test]
fn view_builds_docker_runtime_settings() {
    let mut app = test_app();
    app.runtime_settings.kind = "docker".to_string();
    app.runtime_settings.docker_image = "ubuntu:24.04".to_string();
    app.runtime_settings.docker_network = "host".to_string();
    app.runtime_settings.docker_memory_limit_mb_input = String::new();
    app.runtime_settings.docker_cpu_limit_input = "1.5".to_string();
    app.runtime_settings.docker_read_only_rootfs = false;
    app.runtime_settings.docker_mount_workspace = false;
    app.runtime_settings.docker_allowed_workspace_roots_input =
        "/workspace/a\n/workspace/b".to_string();
    app.runtime_settings.reasoning_enabled_input = "true".to_string();
    app.runtime_settings.reasoning_level_input = "high".to_string();

    let _: Element<'_, Message> = view(&app);
}

#[cfg(not(target_arch = "wasm32"))]
mod rollout {
    use super::*;
    use crate::app::components::{
        system_settings::{self, SystemTab},
        system_settings_observability, system_settings_scheduler,
    };
    use crate::app::message::settings::HooksMessage;
    use crate::app::state::CronSettingsTab;
    use iced::advanced::{
        clipboard,
        renderer::{Headless, Style},
        widget::{Id, Operation, operation::TextInput},
    };
    use iced::{Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};

    type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

    const SCENES: [(DesktopLocale, Theme, u32); 3] = [
        (DesktopLocale::English, Theme::Light, 940),
        (DesktopLocale::SimplifiedChinese, Theme::Dark, 420),
        (DesktopLocale::TraditionalChinese, Theme::Light, 640),
    ];

    fn isolated(test: &str) -> bool {
        const CHILD: &str = "SHIJU_HELP_ROLLOUT_12_CHILD";
        if std::env::var_os(CHILD).is_some() {
            return true;
        }
        let home = tempfile::tempdir().unwrap();
        let bootstrap = GatewayClientSystemSettingsConfig {
            host: "[".into(),
            servers: Vec::new(),
            ..Default::default()
        };
        std::fs::write(
            home.path().join("gateway-client-bootstrap.json"),
            serde_json::to_vec(&bootstrap).unwrap(),
        )
        .unwrap();
        let module = module_path!().split_once("::").unwrap().1;
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &format!("{module}::{test}"), "--test-threads=1"])
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

    #[derive(Default)]
    struct Geometry {
        labels: Vec<(String, Rectangle)>,
        inputs: Vec<(String, Rectangle)>,
    }

    impl Operation for Geometry {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }

        fn text(&mut self, _: Option<&Id>, bounds: Rectangle, value: &str) {
            self.labels.push((value.into(), bounds));
        }

        fn text_input(&mut self, _: Option<&Id>, bounds: Rectangle, state: &mut dyn TextInput) {
            self.inputs.push((state.text().into(), bounds));
        }
    }

    impl Geometry {
        fn label(&self, value: &str) -> Rectangle {
            self.labels
                .iter()
                .find(|(label, _)| label == value)
                .unwrap_or_else(|| panic!("missing label {value}"))
                .1
        }
    }

    fn geometry(ui: &mut Ui<'_>, renderer: &Renderer) -> Geometry {
        let mut positions = Geometry::default();
        ui.operate(renderer, &mut positions);
        positions
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
        messages.retain(|message| !matches!(message, Message::None));
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

    fn escape(ui: &mut Ui<'_>, renderer: &mut Renderer) {
        use iced::keyboard::{self, Key, key};
        let key = Key::Named(key::Named::Escape);
        let event = Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: None,
            repeat: false,
        });
        assert!(send(ui, renderer, mouse::Cursor::Unavailable, &[event]).is_empty());
    }

    fn assert_help(
        ui: &mut Ui<'_>,
        renderer: &mut Renderer,
        (locale, theme, size): (DesktopLocale, &Theme, Size<u32>),
        (title, explanation): (&'static str, &'static str),
        check_menu: bool,
    ) {
        assert_localized_help(
            ui,
            renderer,
            (locale, theme, size),
            (runtime_text(locale, title), runtime_text(locale, explanation)),
            check_menu,
        );
    }

    fn assert_localized_help(
        ui: &mut Ui<'_>,
        renderer: &mut Renderer,
        (locale, theme, size): (DesktopLocale, &Theme, Size<u32>),
        (title, explanation): (&str, &str),
        check_menu: bool,
    ) {
        let label = geometry(ui, renderer).label(title);
        let frame = |ui: &mut Ui<'_>, renderer: &mut Renderer, suffix, cursor| {
            let redraw =
                Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()));
            assert!(send(ui, renderer, cursor, &[redraw]).is_empty());
            ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
            let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
            if let Some(directory) = std::env::var_os("SHIJU_HELP_ROLLOUT_12_SCREENSHOTS") {
                let name = format!("{locale:?}-{title}-{suffix}.png").replace('/', "-");
                image::save_buffer(
                    std::path::PathBuf::from(directory).join(name),
                    &pixels,
                    size.width,
                    size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
            pixels
        };
        let idle = frame(ui, renderer, "idle", mouse::Cursor::Unavailable);
        assert!(
            geometry(ui, renderer).labels.iter().all(|(label, _)| label != explanation),
            "{title}: static explanation is visible before opening help"
        );
        let icon = [
            Point::new(label.x + label.width + 15.0, label.center_y()),
            Point::new(label.x + 11.0, label.y + label.height + 15.0),
        ]
        .into_iter()
        .find(|&position| {
            let cursor = mouse::Cursor::Available(position);
            let hover = Event::Mouse(mouse::Event::CursorMoved { position });
            assert!(send(ui, renderer, cursor, &[hover]).is_empty());
            idle != frame(ui, renderer, "hover", cursor)
        })
        .unwrap_or_else(|| panic!("{title}: hover preview at {label:?}"));
        assert!(click(ui, renderer, icon).is_empty());
        assert!(
            idle != frame(ui, renderer, "open", mouse::Cursor::Unavailable),
            "{title}: pinned help card"
        );
        if check_menu {
            let left = icon.x - 15.0;
            let right = size.width as f32 - icon.x - 15.0;
            let x = if right >= 300.0 || right >= left {
                icon.x + 15.0
            } else {
                left - left.min(300.0)
            };
            let body = Point::new(x + 14.0, icon.y + 5.0);
            let item_height =
                f32::from(iced::advanced::text::LineHeight::default().to_absolute(12.0.into()))
                    + 10.0;
            let search =
                format!("https://www.google.com/search?q={}", urlencoding::encode(explanation));
            for index in [0.0, 3.0] {
                let right_click = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));
                assert!(
                    send(ui, renderer, mouse::Cursor::Available(body), &[right_click]).is_empty()
                );
                let item = Point::new(body.x + 24.0, body.y + 3.0 + item_height * (index + 0.5));
                let messages = click(ui, renderer, item);
                assert!(
                    match messages.as_slice() {
                        [Message::CopyCode(value)] if index == 0.0 => value == explanation,
                        [Message::View(crate::app::message::ViewMessage::OpenUrlExternal(url))]
                            if index == 3.0 =>
                            url == &search,
                        _ => false,
                    },
                    "{title}: {messages:?}"
                );
            }
        }
        escape(ui, renderer);
        assert!(
            idle == frame(ui, renderer, "closed", mouse::Cursor::Unavailable),
            "{title}: Escape restores the idle frame"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn help_rollout_12_renders_contextual_help() {
        if !isolated("help_rollout_12_renders_contextual_help") {
            return;
        }
        let mut renderer = renderer().await;
        let mut app = App::new().0;
        app.runtime_settings.kind = "docker".into();
        app.runtime_settings.docker_image = "fixture/image:1.0".into();
        app.runtime_settings.docker_read_only_rootfs = true;
        for (locale, theme, width) in SCENES {
            app.desktop_locale = locale;
            app.app_theme = theme.clone();
            let size = Size::new(width, 1020);
            let context = (locale, &theme, size);
            let mut ui = scene(view(&app), &mut renderer, size);
            assert_help(
                &mut ui,
                &mut renderer,
                context,
                (
                    "推理覆盖",
                    "该区域用于配置 runtime.reasoning_enabled 与兼容别名 runtime.reasoning_level。",
                ),
                locale == DesktopLocale::English,
            );
            for field in [
                ("运行时类型", "选择 native 或 docker 执行环境。"),
                ("镜像 (image)", "默认执行镜像。"),
                ("Docker 配置", "当 kind=docker 时生效，用于控制镜像、资源限制和工作区挂载。"),
            ] {
                assert_help(&mut ui, &mut renderer, context, field, false);
            }
            let positions = geometry(&mut ui, &renderer);
            assert!(positions.inputs.iter().any(|(value, _)| value == "fixture/image:1.0"));
            let toggle = positions.label(runtime_text(locale, "启用")).center();
            assert!(matches!(
                click(&mut ui, &mut renderer, toggle).as_slice(),
                [Message::Settings(SettingsMessage::Runtime(
                    RuntimeMessage::DockerReadOnlyRootfsToggled(false)
                ))]
            ));
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn help_rollout_12_renders_observability_contextual_help() {
        if !isolated("help_rollout_12_renders_observability_contextual_help") {
            return;
        }
        let mut renderer = renderer().await;
        let mut app = App::new().0;
        const ENDPOINT: &str = "https://collector.invalid/otlp";
        app.observability_settings.backend = "otel".into();
        app.observability_settings.otel_endpoint_input = ENDPOINT.into();
        app.observability_settings.runtime_trace_mode = "rolling".into();
        app.observability_settings.runtime_trace_max_entries = 25_000;
        for (locale, theme, width) in SCENES {
            app.desktop_locale = locale;
            app.app_theme = theme.clone();
            let size = Size::new(width, 760);
            let context = (locale, &theme, size);
            let mut ui = scene(system_settings_observability::view(&app), &mut renderer, size);
            assert_help(
                &mut ui,
                &mut renderer,
                context,
                ("观测后端", "选择观测后端并配置 OTel 端点。"),
                locale == DesktopLocale::English,
            );
            assert_help(&mut ui, &mut renderer, context, ("后端", "选择运行时观测后端。"), false);
            let positions = geometry(&mut ui, &renderer);
            assert_help(
                &mut ui,
                &mut renderer,
                context,
                ("OTEL 端点", "仅 backend=otel 时生效。"),
                false,
            );
            assert!(positions.inputs.iter().any(|(value, _)| value == ENDPOINT));
            let badge = positions.label("25000");
            let slider = Point::new(badge.x - 30.0, badge.center_y());
            assert!(matches!(
                click(&mut ui, &mut renderer, slider).as_slice(),
                [Message::Settings(SettingsMessage::ObservabilityRuntimeTraceMaxEntriesChanged(_))]
            ));
            assert!(matches!(
                click(&mut ui, &mut renderer, Point::new(width as f32 - 16.0, 16.0)).as_slice(),
                [Message::Settings(SettingsMessage::ObservabilityHelpOpen)]
            ));
        }
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn settings_help_gap_07_docker_help_follows_runtime_kind() {
        if !isolated("settings_help_gap_07_docker_help_follows_runtime_kind") {
            return;
        }
        let mut renderer = renderer().await;
        let mut app = App::new().0;
        app.show_system_settings = true;
        app.system_settings_tab = SystemTab::Runtime;
        app.runtime_settings.docker_image = "fixture/image:1.0".into();
        app.runtime_settings.docker_read_only_rootfs = true;
        app.runtime_settings.save_error = Some("fixture runtime error".into());
        for (locale, theme, width) in SCENES {
            app.desktop_locale = locale;
            app.app_theme = theme.clone();
            let size = Size::new(width, 1180);
            for kind in ["docker", "native"] {
                app.runtime_settings.kind = kind.into();
                let mut ui = scene(system_settings::view(&app), &mut renderer, size);
                let positions = geometry(&mut ui, &renderer);
                positions.label("fixture runtime error");
                assert_eq!(
                    positions.inputs.iter().any(|(value, _)| value == "fixture/image:1.0"),
                    kind == "docker"
                );
                if kind == "native" {
                    assert!(
                        positions
                            .labels
                            .iter()
                            .all(|(label, _)| { label != runtime_text(locale, "Docker 配置") })
                    );
                    continue;
                }
                assert_help(
                    &mut ui,
                    &mut renderer,
                    (locale, &theme, size),
                    ("Docker 配置", "当 kind=docker 时生效，用于控制镜像、资源限制和工作区挂载。"),
                    locale == DesktopLocale::English,
                );
                let toggle = positions.label(runtime_text(locale, "启用")).center();
                assert!(matches!(
                    click(&mut ui, &mut renderer, toggle).as_slice(),
                    [Message::Settings(SettingsMessage::Runtime(
                        RuntimeMessage::DockerReadOnlyRootfsToggled(false)
                    ))]
                ));
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn settings_help_gap_07_observability_keeps_mode_and_field_state() {
        if !isolated("settings_help_gap_07_observability_keeps_mode_and_field_state") {
            return;
        }
        let mut renderer = renderer().await;
        let mut app = App::new().0;
        app.show_system_settings = true;
        app.system_settings_tab = SystemTab::Observability;
        const ENDPOINT: &str = "https://collector.invalid/otlp";
        app.observability_settings.otel_endpoint_input = ENDPOINT.into();
        app.observability_settings.runtime_trace_max_entries = 25_000;
        for (locale, theme, width) in SCENES {
            app.desktop_locale = locale;
            app.app_theme = theme.clone();
            let size = Size::new(width, 1000);
            for (backend, trace, inactive) in [
                ("otel", "rolling", 0),
                ("log", "rolling", 1),
                ("otel", "full", 1),
                ("log", "none", 2),
            ] {
                app.observability_settings.backend = backend.into();
                app.observability_settings.runtime_trace_mode = trace.into();
                let mut ui = scene(system_settings::view(&app), &mut renderer, size);
                for field in [
                    ("OTEL 端点", "仅 backend=otel 时生效。"),
                    ("rolling 条数", "rolling 模式下保留的最大条目数。"),
                ] {
                    assert_help(
                        &mut ui,
                        &mut renderer,
                        (locale, &theme, size),
                        field,
                        inactive == 0 && locale == DesktopLocale::English,
                    );
                }
                let positions = geometry(&mut ui, &renderer);
                assert_eq!(
                    positions
                        .labels
                        .iter()
                        .filter(|(label, _)| {
                            label == runtime_text(locale, "当前模式下不生效。")
                        })
                        .count(),
                    inactive
                );
                assert!(positions.inputs.iter().any(|(value, _)| value == ENDPOINT));
                let badge = positions.label("25000");
                let slider = Point::new(badge.x - 30.0, badge.center_y());
                assert!(matches!(
                    click(&mut ui, &mut renderer, slider).as_slice(),
                    [Message::Settings(SettingsMessage::ObservabilityRuntimeTraceMaxEntriesChanged(value))]
                    if (1..=100_000).contains(value)
                ));
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn settings_help_gap_07_scheduler_help_works_in_cron_and_standalone() {
        if !isolated("settings_help_gap_07_scheduler_help_works_in_cron_and_standalone") {
            return;
        }
        let mut renderer = renderer().await;
        let mut app = App::new().0;
        app.show_system_settings = true;
        app.system_settings_tab = SystemTab::Cron;
        app.cron_settings.active_tab = CronSettingsTab::Scheduler;
        app.scheduler_settings.enabled = false;
        app.scheduler_settings.max_tasks = 63;
        app.scheduler_settings.max_concurrent = 7;
        for (locale, theme, width) in SCENES {
            app.desktop_locale = locale;
            app.app_theme = theme.clone();
            let size = Size::new(width, 980);
            let mut ui = scene(system_settings::view(&app), &mut renderer, size);
            let positions = geometry(&mut ui, &renderer);
            let recommendation = runtime_text(
                locale,
                "建议 max_tasks=64、max_concurrent=4；高并发场景可按资源逐步上调。",
            );
            assert!(
                positions.labels.iter().all(|(label, _)| label != recommendation),
                "Scheduler recommendation is visible before opening help"
            );
            positions.label(runtime_text(locale, "调度器"));
            positions.label("63");
            let badge = positions.label("7");
            let explanation = format!(
                "{}\n{recommendation}",
                runtime_text(locale, "允许并行执行的任务数量上限。"),
            );
            assert_localized_help(
                &mut ui,
                &mut renderer,
                (locale, &theme, size),
                (runtime_text(locale, "并发上限"), &explanation),
                locale == DesktopLocale::English,
            );
            assert!(matches!(
                click(&mut ui, &mut renderer, Point::new(badge.x - 78.0, badge.center_y())).as_slice(),
                [Message::Settings(SettingsMessage::SchedulerMaxConcurrentChanged(value))]
                if (1..=100).contains(value)
            ));
            let toggle = positions.label(runtime_text(locale, "开启内置调度器")).center();
            assert!(matches!(
                click(&mut ui, &mut renderer, toggle).as_slice(),
                [Message::Settings(SettingsMessage::SchedulerEnabledToggled(true))]
            ));

            let mut standalone = scene(system_settings_scheduler::view(&app), &mut renderer, size);
            assert_help(
                &mut standalone,
                &mut renderer,
                (locale, &theme, size),
                ("调度配置", "配置内置调度器的开关、吞吐和并发能力。"),
                locale == DesktopLocale::English,
            );
        }
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn settings_help_gap_07_hooks_keep_execution_and_review_state() {
        if !isolated("settings_help_gap_07_hooks_keep_execution_and_review_state") {
            return;
        }
        let mut renderer = renderer().await;
        let mut app = App::new().0;
        app.show_system_settings = true;
        app.system_settings_tab = SystemTab::Hooks;
        app.hooks_settings.command_logger = true;
        app.hooks_settings.catalog_error = Some("fixture catalog error".into());
        app.hooks_settings.catalog = vec![vw_gateway_client::HookCatalogEntryDto {
            plugin_id: "fixture".into(),
            event: "before_tool_call".into(),
            command: "fixture-command".into(),
            args: Vec::new(),
            working_dir: "/fixture".into(),
            mode: "blocking".into(),
            hook_hash: "fixture-hash".into(),
            trusted: false,
        }];
        for (locale, theme, width) in SCENES {
            app.desktop_locale = locale;
            app.app_theme = theme.clone();
            let size = Size::new(width, 1800);
            for enabled in [true, false] {
                app.hooks_settings.enabled = enabled;
                let mut ui = scene(system_settings::view(&app), &mut renderer, size);
                let positions = geometry(&mut ui, &renderer);
                let tr = |source| runtime_text(locale, source);
                let retention = tr("暂停执行时会保留内置钩子的启用状态。");
                assert!(positions.labels.iter().all(|(label, _)| {
                    label != retention
                        && label
                            != tr("关闭总开关后将暂停 hooks 执行，但会保留内置钩子的具体启用状态。")
                }));
                let pause = tr(if enabled {
                    "关闭总开关后将暂停 hooks 执行。"
                } else {
                    "hooks 执行已暂停。"
                });
                assert_eq!(positions.labels.iter().filter(|(label, _)| label == pause).count(), 1);
                for label in [
                    tr("仅加载已发现且已信任的插件 hook 声明。"),
                    tr("待审核声明不会执行；信任与撤销均由服务端按当前稳定哈希校验。"),
                    tr("待审核"),
                    "fixture catalog error",
                ] {
                    positions.label(label);
                }

                let explanation =
                    format!("{}\n{retention}", tr("控制是否开启运行时 hooks 执行。"),);
                assert_localized_help(
                    &mut ui,
                    &mut renderer,
                    (locale, &theme, size),
                    (tr("启用"), &explanation),
                    enabled && locale == DesktopLocale::English,
                );
                let toggle = positions.label(tr("开启运行时 hooks 执行")).center();
                assert!(matches!(
                    click(&mut ui, &mut renderer, toggle).as_slice(),
                    [Message::Settings(SettingsMessage::Hooks(HooksMessage::EnabledToggled(value)))]
                    if *value != enabled
                ));
            }
        }
    }
}
