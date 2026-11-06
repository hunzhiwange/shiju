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
        match self.case % 4 {
            0 => {
                self.state.rows = vec![CatalogRow {
                    entry: AppCatalogEntry {
                        id: AppId::Codex,
                        name: "中文动态应用 Codex".into(),
                        cli_command: Some("codex".into()),
                        runtime_kinds: vec![AppRuntimeKind::NativeCli, AppRuntimeKind::AcpAdapter],
                    },
                    observation: Ok(AppReadResponse {
                        installations: vec![AppInstallation {
                            id: AppInstallationId("opaque".into()),
                            app_id: AppId::Codex,
                            target_host_id: "中文执行主机".into(),
                            environment: AppInstallationEnvironment::Macos,
                            path: "/隔离目录/应用/codex".into(),
                            real_path: None,
                            source: AppInstallationSource::Unknown,
                            is_path_default: true,
                            version: Some("1.0".into()),
                            runtime_status: AppInstallationRuntimeStatus::ProbeFailed {
                                reason: AppInstallationProbeFailure::DamagedInstallation,
                            },
                        }],
                        desktop: None,
                    }),
                }];
            }
            1 => self.state.error = Some("远端离线或服务不可用，请重试".into()),
            2 => self.state.loading = true,
            _ => self.state.connection_open = true,
        }
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Opened(id) => self.window = id,
            Message::Ui(_) => {}
            Message::Frame => {
                self.frames += 1;
                if self.frames == 8 {
                    return window::screenshot(self.window).map(Message::Capture);
                }
            }
            Message::Capture(screenshot) => {
                assert!(screenshot.rgba.chunks_exact(4).any(|p| p[0] != p[1]));
                let name = format!(
                    "{}-{}.png",
                    if self.case < 4 { "light" } else { "dark" },
                    ["catalog", "error", "loading", "connection"][self.case % 4]
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
                if self.case == 8 {
                    eprintln!("PASS: 8 native Chinese catalog scenarios, no shaping panic");
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
        std::env::var("APPS_CATALOG_WINDOW_OUTPUT").expect("explicit evidence output directory"),
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
    .title("应用列表实际渲染验收")
    .theme(|state: &Harness, _| if state.case < 4 { Theme::Light } else { Theme::Dark })
    .subscription(|_| window::frames().map(|_| Message::Frame))
    .settings(iced::Settings {
        fonts: crate::fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
}
