//! Explicit native render check for backend-advertised ACP controls.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
mod apps;
#[cfg(not(target_arch = "wasm32"))]
mod desktop_update_helper;
mod fonts;
use app::components::input_panel::{self, acp_options};
use app::message::chat::{self, ChatMessage};
use app::{App, Message as AppMessage};
use iced::{Element, Task, Theme, window};
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
                if self.frames == 2 && self.scene > 0 {
                    return iced::widget::operation::snap_to_end(acp_options::OPTIONS_SCROLL);
                }
                if self.frames == 8 && !self.capturing {
                    self.capturing = true;
                    return window::screenshot(self.window).map(Message::Captured);
                }
            }
            Message::Captured(image) => {
                let name = [
                    "light-entry.png",
                    "light-effort.png",
                    "dark-pending.png",
                    "dark-confirmed.png",
                    "light-unavailable.png",
                ][self.scene];
                image::save_buffer(
                    self.root.join(name),
                    &image.rgba,
                    image.size.width,
                    image.size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                if self.scene == 4 {
                    return iced::exit();
                }
                self.scene += 1;
                self.frames = 0;
                self.capturing = false;
                self.app.app_theme = self.theme();
                match self.scene {
                    1 => {
                        return self
                            .update(Message::Ui(AppMessage::Chat(ChatMessage::AcpOptionsOpen)));
                    }
                    2 => {
                        let option = &self.app.current_session_runtime().acp_options.options[0];
                        return self.update(Message::Ui(AppMessage::Chat(
                            ChatMessage::AcpOptionSelected(AcpSessionSelection {
                                kind: option.kind.clone(),
                                value: "high".into(),
                            }),
                        )));
                    }
                    3 => {
                        let mut options = self.app.current_session_runtime().acp_options;
                        options.options[0].current = "high".into();
                        self.load_options(options);
                    }
                    4 => {
                        let mut options = self.app.current_session_runtime().acp_options;
                        options.options.clear();
                        options.connected = false;
                        self.load_options(options);
                    }
                    _ => unreachable!(),
                }
            }
            Message::Ui(AppMessage::Chat(message)) => {
                let opening = matches!(message, ChatMessage::AcpOptionsOpen);
                // Exercise the production handler. Replace only the gateway response.
                drop(chat::update(&mut self.app, message));
                if opening {
                    let runtime = self.app.current_session_runtime();
                    assert!(runtime.acp_options_open && runtime.acp_options_pending);
                    self.load_options(runtime.acp_options);
                }
            }
            Message::Ui(_) => {}
        }
        Task::none()
    }

    fn load_options(&mut self, options: AcpSessionOptions) {
        drop(chat::update(
            &mut self.app,
            ChatMessage::AcpOptionsLoaded {
                session_id: "render-session".into(),
                result: Ok(options),
            },
        ));
    }

    fn theme(&self) -> Theme {
        if matches!(self.scene, 2 | 3) { Theme::Dark } else { Theme::Light }
    }

    fn view(&self, _: window::Id) -> Element<'_, Message> {
        let base = iced::widget::column![
            iced::widget::Space::new().height(iced::Length::Fill),
            input_panel::view(&self.app),
        ]
        .into();
        acp_options::with_modal(&self.app, base).map(Message::Ui)
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
    app::config::install_window_test_bootstrap(9);
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("evidence directory"));
    std::fs::create_dir_all(&root).unwrap();
    iced::daemon(
        move || {
            let (mut app, _) = App::new();
            app.app_theme = Theme::Light;
            app.desktop_locale = vw_config_types::ui::DesktopLocale::SimplifiedChinese;
            app.active_session_id = Some("render-session".into());
            app.project_path = Some(root.to_string_lossy().into_owned());
            let runtime = app.get_session_runtime_mut("render-session");
            runtime.acp_agent = Some("copilot".into());
            runtime.acp_model_source =
                vw_gateway_client::vw_api_types::acp_model_source::AcpModelSource::SessionModel;
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
                    kind: AcpOptionKind::Config { id: "reasoning_effort".into() },
                    name: "推理档位".into(),
                    current: "low".into(),
                    choices: vec![
                        AcpChoice { id: "low".into(), name: "低".into() },
                        AcpChoice {
                            id: "high".into(),
                            name: "高档位，使用较多推理预算来处理复杂问题和多个约束条件。此长选项用于检查中文折行和滚动区域的留白。".into(),
                        },
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
    .theme(|state: &Harness, _| state.theme())
    .subscription(|_: &Harness| window::frames().map(|_| Message::Frame))
    .settings(iced::Settings {
        fonts: fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
}
