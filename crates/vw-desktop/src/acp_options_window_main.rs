//! Explicit native render check for backend-advertised ACP controls.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
mod apps;
#[cfg(not(target_arch = "wasm32"))]
mod desktop_update_helper;
mod fonts;
use app::{App, Message as AppMessage};
use iced::{Element, Subscription, Task, Theme, window};
use vw_gateway_client::vw_api_types::acp_session::*;
#[derive(Debug, Clone)]
enum Message {
    Opened(window::Id),
    Frame,
    Captured(window::Screenshot),
    Ui(AppMessage),
}
struct Harness {
    app: App,
    window: window::Id,
    frames: usize,
    dark: bool,
    capturing: bool,
    root: std::path::PathBuf,
}
impl Harness {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Opened(id) => self.window = id,
            Message::Frame => {
                self.frames += 1;
                if self.frames == 8 && !self.capturing {
                    self.capturing = true;
                    return window::screenshot(self.window).map(Message::Captured);
                }
            }
            Message::Captured(image) => {
                let name = if self.dark { "dark.png" } else { "light.png" };
                image::save_buffer(
                    self.root.join(name),
                    &image.rgba,
                    image.size.width,
                    image.size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                if self.dark {
                    return iced::exit();
                }
                self.dark = true;
                self.frames = 0;
                self.capturing = false;
            }
            Message::Ui(_) => {}
        }
        Task::none()
    }
    fn view(&self, _: window::Id) -> Element<'_, Message> {
        let base = iced::widget::container(iced::widget::text("ACP 会话参数验收"))
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into();
        app::components::input_panel::acp_options::with_modal(&self.app, base).map(Message::Ui)
    }
}
fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("evidence directory"));
    std::fs::create_dir_all(&root).unwrap();
    iced::daemon(
        move || {
            let (mut app, _) = App::new();
            app.active_session_id = Some("render-session".into());
            let runtime = app.get_session_runtime_mut("render-session");
            runtime.acp_agent = Some("fixture".into());
            runtime.acp_options_open = true;
            runtime.acp_options = AcpSessionOptions {
                acp_model_source:
                    vw_gateway_client::vw_api_types::acp_model_source::AcpModelSource::SessionModel,
                model_source_capabilities: Vec::new(),
                selected_model: None,
                verified_cli_release: None,
                verified_platform: None,
                connected: true,
                options: vec![
                    AcpSessionOption {
                        kind: AcpOptionKind::Mode,
                        name: "会话模式".into(),
                        current: "ask".into(),
                        choices: vec![
                            AcpChoice { id: "ask".into(), name: "询问（保持权限）".into() },
                            AcpChoice { id: "code".into(), name: "编码".into() },
                        ],
                    },
                    AcpSessionOption {
                        kind: AcpOptionKind::Config { id: "effort".into() },
                        name: "思考强度".into(),
                        current: "high".into(),
                        choices: vec![
                            AcpChoice { id: "low".into(), name: "低".into() },
                            AcpChoice { id: "high".into(), name: "高".into() },
                        ],
                    },
                ],
            };
            let (window, task) = window::open(window::Settings {
                size: iced::Size::new(720.0, 640.0),
                ..Default::default()
            });
            (
                Harness {
                    app,
                    window,
                    frames: 0,
                    dark: false,
                    capturing: false,
                    root: root.clone(),
                },
                task.map(Message::Opened),
            )
        },
        Harness::update,
        Harness::view,
    )
    .title("ACP session options render validation")
    .theme(|state: &Harness, _| if state.dark { Theme::Dark } else { Theme::Light })
    .subscription(|_: &Harness| window::frames().map(|_| Message::Frame))
    .settings(iced::Settings {
        fonts: fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
}
