//! Real Iced window, real authenticated gateway data, screenshots and clean shutdown.
use crate::app::{App, Message as AppMessage, message::SettingsMessage, state::SkillsSettingsTab};
use iced::{Element, Subscription, Task, Theme, window};
use std::{
    path::PathBuf,
    process::{Child, Command},
    time::{Duration, Instant},
};
#[derive(Debug, Clone)]
enum Message {
    Opened(window::Id),
    Ui(AppMessage),
    Frame,
    Capture(window::Screenshot),
}
struct Harness {
    app: App,
    window: window::Id,
    root: PathBuf,
    case: usize,
    frames: usize,
    loaded: bool,
    started: Instant,
    mode: u8,
    step: u8,
    capture_name: String,
    install_source: String,
    update_source: String,
    target: PathBuf,
}
impl Harness {
    fn update(&mut self, message: Message) -> Task<Message> {
        assert!(
            self.started.elapsed() < Duration::from_secs(60),
            "native window acceptance timeout"
        );
        match message {
            Message::Opened(id) => {
                self.window = id;
                self.app
                    .update(AppMessage::Settings(SettingsMessage::PluginsRefresh))
                    .map(Message::Ui)
            }
            Message::Ui(event) => {
                let task = self.app.update(event.clone()).map(Message::Ui);
                if let AppMessage::Settings(SettingsMessage::PluginsLoaded(result)) = &event {
                    let items = result.as_ref().expect("real gateway catalog");
                    if self.mode == 55 {
                        assert_eq!(items.len(), 1);
                        assert!(
                            items[0]
                                .dependency_checks
                                .iter()
                                .any(|c| c.status == "missing_host" && c.dependency == "node")
                        );
                        assert_eq!(items[0].hook_review, "pending");
                        self.loaded = true;
                        self.frames = 0;
                    } else if self.step == 0 {
                        assert!(!self.target.exists());
                        self.step = 1;
                        let _ = self
                            .app
                            .update(AppMessage::Settings(SettingsMessage::PluginsInstallOpen));
                        let _ = self.app.update(AppMessage::Settings(
                            SettingsMessage::PluginsInstallSourceChanged(
                                "/missing-plugin-package".into(),
                            ),
                        ));
                        return Task::batch([
                            task,
                            self.action(SettingsMessage::PluginsInstallPrepare),
                        ]);
                    } else if self.step == 7 {
                        assert!(items.iter().any(|p| p.id == "newcomer" && p.version == "2"));
                        self.step = 8;
                        let _ = self.app.update(AppMessage::Settings(
                            SettingsMessage::PluginsUpdateOpen("newcomer".into()),
                        ));
                        let _ = self.app.update(AppMessage::Settings(
                            SettingsMessage::PluginsInstallSourceChanged(
                                self.update_source.clone(),
                            ),
                        ));
                        return Task::batch([
                            task,
                            self.action(SettingsMessage::PluginsInstallPrepare),
                        ]);
                    } else if self.step == 10 {
                        assert!(items.iter().any(|p| p.id == "newcomer" && p.version == "3"));
                        self.arm("t56-success-dark");
                    }
                }
                if self.mode == 56 {
                    match &event {
                        AppMessage::Settings(SettingsMessage::PluginsInstallPrepared {
                            result,
                            ..
                        }) => {
                            if self.step == 1 {
                                assert!(result.is_err());
                                self.arm("t56-error-light");
                            } else if self.step == 2 {
                                assert!(result.is_ok());
                                assert!(!self.target.exists());
                                self.arm("t56-review-light");
                            } else if self.step == 5 {
                                assert!(result.is_ok());
                                assert!(!self.target.exists());
                                self.step = 6;
                                return Task::batch([
                                    task,
                                    self.action(SettingsMessage::PluginsInstallConfirm),
                                ]);
                            } else if self.step == 8 {
                                let preview = result.as_ref().unwrap();
                                assert_eq!(preview.old_version.as_deref(), Some("2"));
                                assert_eq!(preview.version.as_deref(), Some("3"));
                                assert_eq!(
                                    preview.previous_capabilities.as_ref().unwrap().mcp_servers,
                                    0
                                );
                                assert_eq!(preview.capabilities.mcp_servers, 1);
                                self.arm("t56-update-dark");
                            }
                        }
                        AppMessage::Settings(SettingsMessage::PluginsPreviewDiscarded(result))
                            if self.step == 4 =>
                        {
                            assert!(result.is_ok());
                            assert!(!self.target.exists());
                            assert!(self.app.skills_settings.plugin_install.is_none());
                            self.step = 5;
                            let _ = self
                                .app
                                .update(AppMessage::Settings(SettingsMessage::PluginsInstallOpen));
                            let _ = self.app.update(AppMessage::Settings(
                                SettingsMessage::PluginsInstallSourceChanged(
                                    self.install_source.clone(),
                                ),
                            ));
                            return Task::batch([
                                task,
                                self.action(SettingsMessage::PluginsInstallPrepare),
                            ]);
                        }
                        AppMessage::Settings(SettingsMessage::PluginsInstallCompleted {
                            result,
                            ..
                        }) => {
                            assert!(result.is_ok(), "{result:?}");
                            assert!(self.app.skills_settings.plugin_install.is_none());
                            assert!(self.target.is_dir());
                            assert!(!self.root.join("MUST_NOT_RUN").exists());
                            self.step = if self.step == 6 { 7 } else { 10 };
                        }
                        _ => {}
                    }
                }
                task
            }
            Message::Frame => {
                if self.loaded {
                    self.frames += 1;
                    if self.frames == 5 {
                        return window::screenshot(self.window).map(Message::Capture);
                    }
                }
                Task::none()
            }
            Message::Capture(image) => {
                let name = if self.mode == 56 {
                    format!("{}.png", self.capture_name)
                } else if self.case == 0 {
                    "t55-light.png".into()
                } else {
                    "t55-dark.png".into()
                };
                assert!(image.rgba.chunks_exact(4).any(|p| p[0] != p[1]));
                image::save_buffer(
                    self.root.join(&name),
                    &image.rgba,
                    image.size.width,
                    image.size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                eprintln!("PLUGIN WINDOW CAPTURE {name}; actual Unicode shaping completed");
                if self.mode == 56 {
                    self.loaded = false;
                    match self.step {
                        1 => {
                            self.step = 2;
                            let _ = self.app.update(AppMessage::Settings(
                                SettingsMessage::PluginsInstallSourceChanged(
                                    self.install_source.clone(),
                                ),
                            ));
                            return self.action(SettingsMessage::PluginsInstallPrepare);
                        }
                        2 => {
                            self.step = 3;
                            self.app.app_theme = Theme::Dark;
                            self.arm("t56-review-dark");
                            return Task::none();
                        }
                        3 => {
                            assert!(!self.target.exists());
                            self.step = 4;
                            return self.action(SettingsMessage::PluginsInstallClose);
                        }
                        8 => {
                            self.step = 9;
                            return self.action(SettingsMessage::PluginsInstallConfirm);
                        }
                        10 => {
                            std::fs::write(self.root.join("window-result56.json"),r#"{"themes":2,"real_gateway":true,"retry":true,"cancel_without_publish":true,"explicit_install":true,"explicit_update":true,"capability_diff":true,"unicode_long_path":true,"glyph_panic":false,"no_auto_execution":true}"#).unwrap();
                            return iced::exit();
                        }
                        _ => panic!("unexpected capture state"),
                    }
                }
                if self.case == 0 {
                    self.case = 1;
                    self.frames = 0;
                    self.app.app_theme = Theme::Dark;
                    Task::none()
                } else {
                    std::fs::write(self.root.join("window-result.json"),r#"{"themes":2,"real_gateway":true,"missing_node":true,"hook_pending":true,"unicode_long_path":true,"glyph_panic":false,"closed_without_mutation":true}"#).unwrap();
                    iced::exit()
                }
            }
        }
    }
    fn action(&mut self, message: SettingsMessage) -> Task<Message> {
        self.app.update(AppMessage::Settings(message)).map(Message::Ui)
    }
    fn arm(&mut self, name: &str) {
        self.capture_name = name.into();
        self.loaded = true;
        self.frames = 0;
    }
    fn view(&self, _: window::Id) -> Element<'_, Message> {
        let content: Element<'_, AppMessage> = iced::widget::container(
            iced::widget::scrollable(
                iced::widget::container(
                    crate::app::components::system_settings_skills::plugins::view(&self.app),
                )
                .padding([0, 7]),
            )
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new().width(4),
            )),
        )
        .padding(20)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into();
        let content =
            crate::app::components::system_settings_skills::with_plugin_install(&self.app, content);
        let content: Element<'_, AppMessage> = iced::widget::stack![
            content,
            iced::widget::container(crate::app::components::toast::view(&self.app))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding(12)
        ]
        .into();
        content.map(Message::Ui)
    }
    fn subscription(&self) -> Subscription<Message> {
        window::frames().map(|_| Message::Frame)
    }
}
struct Fixture {
    child: Child,
    root: PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::write(self.root.join("stop"), "stop");
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.child.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}
pub(crate) fn run() {
    let mut args = std::env::args().skip(1);
    let backend = args.next().expect("explicit backend test executable");
    let root = PathBuf::from(args.next().expect("absolute evidence directory"));
    let mode: u8 = args.next().unwrap_or_else(|| "55".into()).parse().unwrap();
    assert!(matches!(mode, 55 | 56));
    assert!(root.is_absolute() && root.is_dir());
    std::env::set_current_dir(&root).unwrap();
    let child = Command::new(backend)
        .args([
            "plugins::compat::plugin_window_fixture::plugin_window_gateway_fixture",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env("SHIJU_PLUGIN_WINDOW_ROOT", &root)
        .spawn()
        .unwrap();
    std::fs::write(root.join("backend.pid"), child.id().to_string()).unwrap();
    let _fixture = Fixture { child, root: root.clone() };
    let deadline = Instant::now() + Duration::from_secs(30);
    while !root.join("ready.json").exists() {
        assert!(Instant::now() < deadline, "gateway fixture readiness timeout");
        std::thread::sleep(Duration::from_millis(20));
    }
    let ready: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("ready.json")).unwrap()).unwrap();
    crate::app::config::install_window_test_bootstrap(ready["port"].as_u64().unwrap() as u16);
    let install_source = ready["install_source"].as_str().unwrap().to_owned();
    let update_source = ready["update_source"].as_str().unwrap().to_owned();
    let target = PathBuf::from(ready["long_config"].as_str().unwrap()).join("plugins/newcomer");
    iced::daemon(
        move || {
            let (mut app, _) = App::new();
            app.app_theme = Theme::Light;
            let (id, task) = window::open(window::Settings {
                size: iced::Size::new(1100., 850.),
                exit_on_close_request: false,
                ..Default::default()
            });
            (
                Harness {
                    app,
                    window: id,
                    root: root.clone(),
                    case: 0,
                    frames: 0,
                    loaded: false,
                    started: Instant::now(),
                    mode,
                    step: 0,
                    capture_name: String::new(),
                    install_source: install_source.clone(),
                    update_source: update_source.clone(),
                    target: target.clone(),
                },
                task.map(Message::Opened),
            )
        },
        Harness::update,
        Harness::view,
    )
    .title("Plugin compatibility validation")
    .theme(|state: &Harness, _| state.app.app_theme.clone())
    .subscription(Harness::subscription)
    .settings(iced::Settings {
        fonts: crate::fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
}
