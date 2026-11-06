//! Explicit native shaping and screenshot evidence for production preferences.
use crate::app::components::system_settings_apps::{self, catalog::CatalogRow};
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
        let windows = self.case % 5 >= 3;
        self.state.rows =
            [AppId::Codex, AppId::Claude, AppId::Gemini, AppId::GrokBuild, AppId::Hermes]
                .into_iter()
                .map(|id| CatalogRow {
                    entry: AppCatalogEntry {
                        id,
                        name: format!("中文动态应用 {id:?}"),
                        cli_command: Some("fixture".into()),
                        runtime_kinds: vec![AppRuntimeKind::NativeCli],
                    },
                    observation: Ok(AppReadResponse {
                        desktop: None,
                        installations: vec![AppInstallation {
                            id: AppInstallationId("中文实例".into()),
                            app_id: id,
                            target_host_id: self.state.host.clone(),
                            environment: if windows {
                                AppInstallationEnvironment::Wsl {
                                    distribution: "Ubuntu-中文发行版".into(),
                                }
                            } else {
                                AppInstallationEnvironment::Macos
                            },
                            path: "/隔离目录/中文应用".into(),
                            real_path: None,
                            source: AppInstallationSource::System,
                            is_path_default: false,
                            version: None,
                            runtime_status: AppInstallationRuntimeStatus::Runnable,
                        }],
                    }),
                })
                .collect();
        let value = AppPreferencesResult {
            user: AppUserPreferences {
                app_order: self
                    .state
                    .rows
                    .iter()
                    .map(|r| serde_json::to_value(r.entry.id).unwrap().as_str().unwrap().to_owned())
                    .collect(),
                visible_apps: vec![AppId::Codex],
            },
            host: AppHostPreferences {
                preferred_terminal: Some(if windows { "wt" } else { "terminal" }.into()),
                wsl_shells: vec![AppWslPreference {
                    app_id: AppId::Codex,
                    shell: Some("bash".into()),
                    flag: Some("-lc".into()),
                }],
            },
            restart_required: self.case % 5 == 0,
            capabilities: Some(AppPreferencesCapabilities {
                terminals: vec![if windows { "wt" } else { "terminal" }.into()],
                wsl_preferences: windows,
            }),
        };
        self.state.preferences = system_settings_apps::preferences::PreferencesState {
            saved: Some(value.clone()),
            draft: Some(value),
            open: true,
            busy: self.case % 5 == 2,
            error: (self.case % 5 == 1)
                .then(|| "保存被执行主机拒绝，已恢复原值；至少保留一项可见应用".into()),
            generation: 0,
        };
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Opened(id) => self.window = id,
            Message::Ui(_) => {}
            Message::Frame => {
                self.frames += 1;
                if self.frames == 2 {
                    return iced::widget::operation::snap_to(
                        "apps-preferences-body",
                        if self.case % 5 == 4 {
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
                        if self.case < 5 { "light" } else { "dark" },
                        ["saved", "error", "disabled", "wsl", "wsl-fields"][self.case % 5]
                    )),
                    &screenshot.rgba,
                    screenshot.size.width,
                    screenshot.size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                self.case += 1;
                if self.case == 10 {
                    eprintln!("PASS: 10 native Chinese preferences scenarios; no shaping panic");
                    return iced::exit();
                }
                self.frames = 0;
                self.setup();
            }
        }
        Task::none()
    }
    fn view(&self, _: window::Id) -> Element<'_, Message> {
        let base = iced::widget::container(iced::widget::text("应用设置实际渲染验收"))
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into();
        system_settings_apps::preferences::overlay(&self.state, base).map(Message::Ui)
    }
}
pub fn run() {
    let root = std::path::PathBuf::from(
        std::env::var("APPS_PREFERENCES_WINDOW_OUTPUT").expect("explicit evidence directory"),
    );
    std::fs::create_dir_all(&root).unwrap();
    iced::daemon(
        move || {
            let (id, task) = window::open(window::Settings {
                size: iced::Size::new(600., 650.),
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
    .title("应用设置实际渲染验收")
    .theme(|state: &Harness, _| if state.case < 5 { Theme::Light } else { Theme::Dark })
    .subscription(|_| window::frames().map(|_| Message::Frame))
    .settings(iced::Settings {
        fonts: crate::fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
}
