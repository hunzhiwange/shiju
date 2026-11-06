//! Real native windows shape Chinese dynamic text in both themes and a narrow viewport.
use crate::app::components::system_settings_apps::{self, catalog::CatalogRow};
use crate::app::{Message as AppMessage, state::AppsCatalogState};
use iced::{Element, Subscription, Task, Theme, window};
use vw_gateway_client::vw_api_types::agent_apps::*;

#[derive(Clone, Debug)]
enum Message {
    Ui(AppMessage),
    Frame,
    Opened(window::Id),
    Capture(window::Screenshot),
}
struct Harness {
    state: AppsCatalogState,
    window: window::Id,
    case: usize,
    frames: usize,
    root: std::path::PathBuf,
}
impl Harness {
    fn setup(&mut self) {
        self.state = AppsCatalogState { host: "中文执行主机".into(), ..Default::default() };
        let request = AppActionPreviewRequest {
            app_id: if self.case % 6 == 1 { AppId::Hermes } else { AppId::GrokBuild },
            action: AppLifecycleAction::Uninstall,
            target: AppActionTarget::Installation {
                installation_id: AppInstallationId("中文目标实例二".into()),
            },
        };
        let lifecycle = &mut self.state.lifecycle;
        lifecycle.request = Some(request.clone());
        lifecycle.target_label =
            "实例路径：/隔离目录/中文应用/所选启动器\n真实路径：/真实目录/中文应用/所选程序\n观察来源：System · Macos".into();
        match self.case % 6 {
            0 => {
                self.state.rows = vec![CatalogRow {
                    entry: AppCatalogEntry {
                        id: AppId::GrokBuild,
                        name: "中文多实例应用".into(),
                        cli_command: Some("codex".into()),
                        runtime_kinds: vec![AppRuntimeKind::NativeCli],
                    },
                    observation: Ok(AppReadResponse {
                        desktop: None,
                        installations: ["第一实例", "第二实例"]
                            .into_iter()
                            .map(|name| AppInstallation {
                                id: AppInstallationId(name.into()),
                                app_id: AppId::GrokBuild,
                                target_host_id: "中文执行主机".into(),
                                environment: AppInstallationEnvironment::Macos,
                                path: format!("/隔离目录/{name}/codex"),
                                real_path: None,
                                source: if name == "第一实例" {
                                    AppInstallationSource::Unknown
                                } else {
                                    AppInstallationSource::System
                                },
                                is_path_default: false,
                                version: Some("1.0".into()),
                                runtime_status: AppInstallationRuntimeStatus::Runnable,
                            })
                            .collect(),
                    }),
                }];
            }
            1 | 4 => {
                lifecycle.preview = Some(AppActionPreview {
                    preview_id: AppActionPreviewId("review".into()),
                    request,
                    source: AppInstallationSource::System,
                    confirmation_required: true,
                    summary: "服务端已预检；请确认所选实例的真实路径、来源和执行主机。".into(),
                    display_command: None,
                });
                lifecycle.busy = self.case % 6 == 4;
                if lifecycle.busy {
                    lifecycle.preview.as_mut().unwrap().summary =
                        "长预检说明：请核对中文实际路径、来源及执行主机。".repeat(40);
                }
            }
            2 | 5 => {
                lifecycle.operation = Some(AppOperationStatus {
                    operation_id: vw_gateway_client::vw_api_types::id::TaskId(
                        "同一操作-恢复后".into(),
                    ),
                    status: if self.case % 6 == 2 {
                        vw_gateway_client::vw_api_types::task::TaskStatus::Running
                    } else {
                        vw_gateway_client::vw_api_types::task::TaskStatus::Cancelled
                    },
                    phase: if self.case % 6 == 2 {
                        AppOperationPhase::Executing
                    } else {
                        AppOperationPhase::Terminal
                    },
                })
            }
            _ => {
                lifecycle.uncertain = false;
                lifecycle.error = Some("计划已过期，请重新预检并确认；不会自动重试删除".into());
            }
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Opened(id) => self.window = id,
            Message::Ui(_) => {}
            Message::Frame => {
                self.frames += 1;
                if self.frames == 60 {
                    return window::screenshot(self.window).map(Message::Capture);
                }
            }
            Message::Capture(screenshot) => {
                assert!(screenshot.rgba.chunks_exact(4).any(|p| p[0] != p[1]));
                let name = format!(
                    "{}-{}.png",
                    if self.case < 6 { "light" } else { "dark" },
                    ["instances", "confirm", "progress", "error", "disabled", "cancelled"]
                        [self.case % 6]
                );
                image::save_buffer(
                    self.root.join(&name),
                    &screenshot.rgba,
                    screenshot.size.width,
                    screenshot.size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                self.case += 1;
                if self.case == 12 {
                    eprintln!("PASS: 12 native Chinese uninstall scenarios, no shaping panic");
                    return iced::exit();
                }
                self.frames = 0;
                self.setup();
            }
        }
        Task::none()
    }
    fn view(&self, _: window::Id) -> Element<'_, Message> {
        let content = iced::widget::container(
            iced::widget::scrollable(system_settings_apps::view(&self.state)).direction(
                iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
                ),
            ),
        )
        .padding(16)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill);
        system_settings_apps::view_overlays(&self.state, content.into()).map(Message::Ui)
    }
}
pub fn run() {
    let root = std::path::PathBuf::from(
        std::env::var("APPS_UNINSTALL_WINDOW_OUTPUT").expect("explicit evidence output directory"),
    );
    std::fs::create_dir_all(&root).unwrap();
    iced::daemon(
        move || {
            let (id, task) = window::open(window::Settings {
                size: iced::Size::new(600.0, 650.0),
                ..Default::default()
            });
            let mut harness = Harness {
                state: Default::default(),
                window: id,
                case: 0,
                frames: 0,
                root: root.clone(),
            };
            harness.setup();
            (harness, task.map(Message::Opened))
        },
        Harness::update,
        Harness::view,
    )
    .title("卸载实际渲染验收")
    .theme(|state: &Harness, _| if state.case < 6 { Theme::Light } else { Theme::Dark })
    .subscription(|_| window::frames().map(|_| Message::Frame))
    .settings(iced::Settings {
        fonts: crate::fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
}
