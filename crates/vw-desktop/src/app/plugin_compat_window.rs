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
                if let AppMessage::Settings(SettingsMessage::PluginsLoaded(result)) = &event {
                    let items = result.as_ref().expect("real gateway catalog");
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
                }
                self.app.update(event).map(Message::Ui)
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
                let name = if self.case == 0 { "t55-light.png" } else { "t55-dark.png" };
                assert!(image.rgba.chunks_exact(4).any(|p| p[0] != p[1]));
                image::save_buffer(
                    self.root.join(name),
                    &image.rgba,
                    image.size.width,
                    image.size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                eprintln!("PLUGIN WINDOW CAPTURE {name}; actual Unicode shaping completed");
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
