//! Real native windows shape Chinese dynamic text in both themes and a narrow viewport.
use crate::app::components::system_settings_apps::{self, catalog::CatalogRow};
use crate::app::{Message as AppMessage, i18n::I18n, state::AppsCatalogState};
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
        self.state =
            AppsCatalogState { host: vw_shared::local_apps::HOST_ID.into(), ..Default::default() };
        for (id, name, installed) in [
            (AppId::Claude, "Claude Code", true),
            (AppId::Codex, "Codex", true),
            (AppId::Gemini, "Gemini CLI", true),
            (AppId::OpenCode, "OpenCode", false),
            (AppId::Kimi, "Kimi Code", false),
            (AppId::Hermes, "Hermes", false),
            (AppId::Mcode, "MiniMax Code", false),
        ] {
            self.state.rows.push(CatalogRow {
                entry: AppCatalogEntry {
                    id,
                    name: name.into(),
                    cli_command: Some(name.into()),
                    runtime_kinds: vec![AppRuntimeKind::NativeCli],
                },
                observation: Ok(AppReadResponse {
                    installations: if installed {
                        vec![AppInstallation {
                            id: AppInstallationId(format!("fixture-{id:?}")),
                            app_id: id,
                            target_host_id: vw_shared::local_apps::HOST_ID.into(),
                            environment: AppInstallationEnvironment::Macos,
                            path: format!("/隔离目录/应用/{id:?}"),
                            real_path: None,
                            source: if id == AppId::Codex {
                                AppInstallationSource::System
                            } else {
                                AppInstallationSource::Homebrew
                            },
                            is_path_default: true,
                            version: Some(
                                match id {
                                    AppId::Claude => "2.1.316",
                                    AppId::Gemini => "0.66.0",
                                    _ => "0.112.0",
                                }
                                .into(),
                            ),
                            runtime_status: AppInstallationRuntimeStatus::Runnable,
                        }]
                    } else {
                        vec![]
                    },
                    desktop: None,
                }),
            });
        }
        self.state.preferences.saved = Some(AppPreferencesResult {
            user: AppUserPreferences {
                app_order: self
                    .state
                    .rows
                    .iter()
                    .map(|a| {
                        serde_json::to_value(a.entry.id).unwrap().as_str().unwrap().to_string()
                    })
                    .collect(),
                visible_apps: self.state.rows.iter().map(|a| a.entry.id).collect(),
            },
            host: AppHostPreferences::default(),
            restart_required: false,
            capabilities: None,
        });
        match self.case % 9 {
            0 => {}
            1 => self.state.expanded = Some(AppId::Codex),
            2 => {
                self.state.drag = system_settings_apps::drag::DragState {
                    source: Some(AppId::Claude),
                    target: Some(AppId::Gemini),
                }
            }
            3 => self.state.query = "Codex".into(),
            4 => {
                self.state.rows.clear();
                self.state.error = Some("本机应用服务暂不可用，请刷新重试。中文动态提示。".into());
            }
            5 => {
                self.state.rows.clear();
                self.state.loading = true;
            }
            6 => self.state.connection_open = true,
            7 => self.state.rows.clear(),
            _ => self.state.hovered = Some(AppId::Claude),
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
                    if self.case < 9 { "light" } else { "dark" },
                    [
                        "catalog",
                        "details",
                        "drag",
                        "search",
                        "error",
                        "loading",
                        "connection",
                        "empty",
                        "hover"
                    ][self.case % 9]
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
                if self.case == 18 {
                    eprintln!("PASS: 18 native Chinese catalog scenarios, no shaping panic");
                    return iced::exit();
                }
                self.frames = 0;
                self.setup();
            }
        }
        Task::none()
    }
    fn theme(&self) -> Theme {
        if self.case < 9 { Theme::Light } else { Theme::Dark }
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
        system_settings_apps::view_overlays(
            &self.state,
            I18n::new(Default::default()),
            &self.theme(),
            content.into(),
        )
        .map(Message::Ui)
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
    .theme(|state: &Harness, _| state.theme())
    .subscription(|_| window::frames().map(|_| Message::Frame))
    .settings(iced::Settings {
        fonts: crate::fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
}
