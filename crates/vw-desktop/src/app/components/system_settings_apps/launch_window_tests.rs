//! Explicit native shaping and screenshot evidence for production launch form.
use crate::app::components::system_settings_apps;
use crate::app::{Message as AppMessage, state::AppsCatalogState};
use iced::{Element, Task, Theme, window};
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
        self.state =
            AppsCatalogState { host: "远端中文执行主机".into(), ..Default::default() };
        self.state.launch = system_settings_apps::launch::LaunchState {
            request: Some(LaunchRequest {
                app_id: AppId::Codex,
                installation_id: AppInstallationId("远端中文实例".into()),
                cwd: "/授权目录/中文项目".into(),
                args: vec![],
                skip_permissions: false,
            }),
            args: vec!["中文动态参数；不执行 shell".into(), "第二个参数".into()],
            identities_ready: true,
            identities: if self.case % 6 == 3 {
                vec![]
            } else {
                vec![
                    LaunchIdentity {
                        kind: LaunchIdentityKind::Provider,
                        id: "provider-fixture".into(),
                        label: "中文供应商".into(),
                    },
                    LaunchIdentity {
                        kind: LaunchIdentityKind::OfficialAccount,
                        id: "account-fixture".into(),
                        label: "中文官方账号".into(),
                    },
                ]
            },
            selected: Some(system_settings_apps::launch::LaunchChoice::Identity(LaunchIdentity {
                kind: LaunchIdentityKind::OfficialAccount,
                id: "account-fixture".into(),
                label: "中文官方账号".into(),
            })),
            phase: (self.case % 6 == 0).then_some(LaunchSessionPhase::Completed),
            accepted: self.case % 6 == 0,
            busy: self.case % 6 == 2,
            error: (self.case % 6 == 1)
                .then(|| "执行主机拒绝目录；没有回退本地终端或其他账号".into()),
            ..Default::default()
        };
        if self.case % 6 == 3 {
            self.state.launch.selected = None;
        } else if self.case % 6 == 5 {
            self.state.host = vw_shared::local_apps::HOST_ID.into();
            self.state.launch.identities.clear();
            self.state.launch.selected =
                Some(system_settings_apps::launch::LaunchChoice::ExistingConfiguration);
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Opened(id) => self.window = id,
            Message::Ui(_) => {}
            Message::Frame => {
                self.frames += 1;
                if self.frames == 2 {
                    return iced::widget::operation::snap_to(
                        "apps-launch-body",
                        if self.case % 6 == 4 {
                            iced::widget::scrollable::RelativeOffset::END
                        } else {
                            iced::widget::scrollable::RelativeOffset::START
                        },
                    );
                }
                if self.frames == 60 {
                    return window::screenshot(self.window).map(Message::Capture);
                }
            }
            Message::Capture(screenshot) => {
                assert!(screenshot.rgba.chunks_exact(4).any(|p| p[0] != p[1]));
                image::save_buffer(
                    self.root.join(format!(
                        "{}-{}.png",
                        if self.case < 6 { "light" } else { "dark" },
                        ["accepted", "error", "disabled", "ready", "fields", "local-ready"]
                            [self.case % 6]
                    )),
                    &screenshot.rgba,
                    screenshot.size.width,
                    screenshot.size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                self.case += 1;
                if self.case == 12 {
                    eprintln!("PASS: 12 native Chinese launch scenarios; no shaping panic");
                    return iced::exit();
                }
                self.frames = 0;
                self.setup();
            }
        }
        Task::none()
    }
    fn view(&self, _: window::Id) -> Element<'_, Message> {
        let base = iced::widget::container(iced::widget::text("终端启动实际渲染验收"))
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into();
        system_settings_apps::launch::overlay(&self.state, base).map(Message::Ui)
    }
}
pub fn run() {
    let root = std::path::PathBuf::from(
        std::env::var("APPS_LAUNCH_WINDOW_OUTPUT").expect("explicit evidence directory"),
    );
    std::fs::create_dir_all(&root).unwrap();
    iced::daemon(
        move || {
            let (id, task) = window::open(window::Settings {
                size: iced::Size::new(560., 650.),
                ..Default::default()
            });
            let mut state = Harness {
                state: Default::default(),
                window: id,
                case: 0,
                frames: 0,
                root: root.clone(),
            };
            state.setup();
            (state, task.map(Message::Opened))
        },
        Harness::update,
        Harness::view,
    )
    .title("终端启动实际渲染验收")
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
