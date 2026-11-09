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
    scene: usize,
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
                let name = [
                    "light-approval.png",
                    "dark-full-access.png",
                    "light-disconnected.png",
                    "dark-failed.png",
                ][self.scene];
                image::save_buffer(
                    self.root.join(name),
                    &image.rgba,
                    image.size.width,
                    image.size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                if self.scene == 3 {
                    return iced::exit();
                }
                self.scene += 1;
                let runtime = self.app.get_session_runtime_mut("render-session");
                runtime.full_access_enabled = self.scene == 1;
                runtime.acp_options.connected = self.scene != 2;
                runtime.acp_options.access_status = match self.scene {
                    1 => Some(AcpAccessStatus {
                        requested: true,
                        support: AcpAccessSupport::Partial,
                        explanation: "后端原生工具仍可能要求确认。".into(),
                        ..fixture_status()
                    }),
                    3 => Some(AcpAccessStatus {
                        support: AcpAccessSupport::Failed,
                        explanation: "连接失败；请刷新重试。".into(),
                        ..fixture_status()
                    }),
                    _ => None,
                };
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
fn fixture_status() -> AcpAccessStatus {
    AcpAccessStatus {
        requested: false,
        backend: "fixture".into(),
        version: Some("1.0".into()),
        support: AcpAccessSupport::Unavailable,
        explanation: "沿用工作区工具审批规则。".into(),
    }
}

fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("evidence directory"));
    std::fs::create_dir_all(&root).unwrap();
    iced::daemon(
        move || {
            let (mut app, _) = App::new();
            app.active_session_id = Some("render-session".into());
            app.project_path = Some(root.to_string_lossy().into_owned());
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
                access_status: Some(fixture_status()),
                connected: true,
                options: vec![AcpSessionOption {
                    kind: AcpOptionKind::Config { id: "effort".into() },
                    name: "思考强度".into(),
                    current: "high".into(),
                    choices: vec![
                        AcpChoice { id: "low".into(), name: "低".into() },
                        AcpChoice { id: "high".into(), name: "高".into() },
                    ],
                }],
            };
            let (window, task) = window::open(window::Settings {
                size: iced::Size::new(720.0, 640.0),
                ..Default::default()
            });
            (
                Harness { app, window, frames: 0, scene: 0, capturing: false, root: root.clone() },
                task.map(Message::Opened),
            )
        },
        Harness::update,
        Harness::view,
    )
    .title("ACP session options render validation")
    .theme(|state: &Harness, _| if state.scene % 2 == 1 { Theme::Dark } else { Theme::Light })
    .subscription(|_: &Harness| window::frames().map(|_| Message::Frame))
    .settings(iced::Settings {
        fonts: fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
}
