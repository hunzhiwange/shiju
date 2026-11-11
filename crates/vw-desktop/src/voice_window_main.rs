//! Explicit, isolated voice rendering and live acceptance. Never runs in the default suite.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
mod apps;
mod desktop_update_helper;
mod fonts;

use app::voice::{
    conversation::{Phase, Snapshot},
    preferences::{Edit, Preferences},
};
use app::{
    App, Message as AppMessage,
    message::{settings, voice},
};
use iced::{
    Element, Length, Task, Theme,
    widget::{button, column, container, row, text},
    window,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use vw_config_types::voice::VoiceConfig;

#[derive(Clone, Debug)]
enum Message {
    Ui(AppMessage),
    Frame,
    Closed(window::Id),
    Watchdog,
    Opened(window::Id),
    Capture(window::Screenshot),
    Confirm,
}
struct Harness {
    app: App,
    snapshot: Snapshot,
    window: window::Id,
    case: usize,
    frames: usize,
    root: PathBuf,
    live: bool,
    heard: bool,
    confirmed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    began: Instant,
}
impl Harness {
    fn setup(&mut self) {
        self.app.voice.settings.edit = None;
        if self.case % 3 != 0 {
            self.app.voice.settings.edit = Some(Edit::new(
                VoiceConfig {
                    dictionary: (0..45).map(|n| format!("中文词典条目{n}")).collect(),
                    ..Default::default()
                },
                Preferences::default(),
            ));
        }
    }
    fn theme(&self) -> Theme {
        if self.case < 3 { Theme::Light } else { Theme::Dark }
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Opened(id) => self.window = id,
            Message::Closed(_) => {
                voice::cancel(&mut self.app);
                return iced::exit();
            }
            Message::Watchdog => {
                if !self.live && self.began.elapsed() > Duration::from_secs(90) {
                    voice::cancel(&mut self.app);
                    return iced::exit();
                }
            }
            Message::Ui(message) if self.live => {
                return match message {
                    AppMessage::Voice(event) => {
                        voice::update(&mut self.app, event).map(Message::Ui)
                    }
                    AppMessage::Settings(app::message::SettingsMessage::Voice(event)) => {
                        settings::voice::update(&mut self.app, event).map(Message::Ui)
                    }
                    _ => Task::none(),
                };
            }
            Message::Ui(_) => {}
            Message::Confirm if self.live => {
                let has_dictation = self
                    .app
                    .current_session_runtime_ref()
                    .is_some_and(|r| !r.input_editor.text().trim().is_empty());
                if !has_dictation || !self.heard || self.app.voice.conversation.is_some() {
                    return self.app.show_error_toast("请先完成本次听写、实际语音播放并结束通话，再核对真人验收清单。失败或未执行的场景不可确认。").map(Message::Ui);
                }
                voice::cancel(&mut self.app);
                self.confirmed.store(true, std::sync::atomic::Ordering::Release);
                return iced::exit();
            }
            Message::Confirm => {}
            Message::Frame => {
                self.frames += 1;
                if self.live {
                    self.heard |= self.app.voice.conversation.as_ref().is_some_and(|c| {
                        let state = c.snapshot();
                        state.microphone && state.playing
                    });
                    return Task::none();
                }
                if self.frames == 10 && self.case % 3 == 2 {
                    return iced::widget::operation::snap_to(
                        "voice-settings-body",
                        iced::widget::scrollable::RelativeOffset::END,
                    );
                }
                if self.frames == 40 {
                    return window::screenshot(self.window).map(Message::Capture);
                }
                assert!(
                    self.began.elapsed() < Duration::from_secs(90),
                    "native rendering deadline exceeded"
                );
            }
            Message::Capture(screenshot) => {
                assert!(
                    screenshot.rgba.chunks_exact(4).any(|p| p[0] != p[1]),
                    "blank native frame"
                );
                image::save_buffer(
                    self.root.join(format!("voice-{}.png", self.case)),
                    &screenshot.rgba,
                    screenshot.size.width,
                    screenshot.size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                self.case += 1;
                self.frames = 0;
                if self.case == 6 {
                    self.confirmed.store(true, std::sync::atomic::Ordering::Release);
                    eprintln!(
                        "VOICE_NATIVE frames=6 themes=2 elapsed_ms={}",
                        self.began.elapsed().as_millis()
                    );
                    return iced::exit();
                }
                self.setup();
            }
        }
        Task::none()
    }
    fn view(&self, _: window::Id) -> Element<'_, Message> {
        if !self.live {
            let base =
                container(text("隔离语音验收窗口")).width(Length::Fill).height(Length::Fill).into();
            return if self.case % 3 == 0 {
                app::components::voice_chat::overlay_screen(base, self.snapshot.clone(), None)
                    .map(Message::Ui)
            } else {
                app::components::system_settings_voice::overlay(&self.app, base).map(Message::Ui)
            };
        }
        use app::components::system_settings_common::rounded_action_btn_style;
        let control = |label, event| {
            button(text(label))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press(AppMessage::Voice(event))
        };
        let base = container(
            column![
                text("真人语音验收：只使用本次录音，不上传历史录音").size(22),
                row![
                    control("开始听写", voice::Event::Start),
                    control("完成听写", voice::Event::Finish),
                    control("取消", voice::Event::Cancel)
                ]
                .spacing(8),
                row![
                    control("语音聊天", voice::Event::ChatStart),
                    control("静音", voice::Event::ChatMute),
                    control("打断", voice::Event::ChatInterrupt),
                    control("结束", voice::Event::ChatEnd)
                ]
                .spacing(8),
                text(
                    self.app
                        .current_session_runtime_ref()
                        .map(|r| r.input_editor.text())
                        .unwrap_or_default()
                ),
                button("语音设置与音色试听")
                    .padding([8, 14])
                    .style(rounded_action_btn_style)
                    .on_press(AppMessage::Settings(app::message::SettingsMessage::Voice(
                        settings::voice::Event::Open
                    ))),
                text("完成文档中所有真人场景后，关闭语音弹窗并确认；直接关闭窗口视为未完成。"),
            ]
            .spacing(16),
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
        let content = app::components::voice_chat::with_voice_chat(&self.app, base);
        let content = app::components::system_settings_voice::overlay(&self.app, content);
        column![
            content.map(Message::Ui),
            app::components::toast::view(&self.app).map(Message::Ui),
            button("确认全部真人场景已验收")
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press(Message::Confirm)
        ]
        .into()
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("VOICE_ACCEPTANCE_FAILED: {error}");
        std::process::exit(2);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let live = match args.as_slice() {
        [flag, value] if flag == "--scenario" && value == "deterministic" => false,
        [flag, value] if flag == "--scenario" && value == "live" => true,
        _ => return Err("usage: --scenario deterministic|live".into()),
    };
    // The live bootstrap must be explicitly selected by the operator, never discovered
    // from Codex credentials or an unrelated user's configuration.
    let bootstrap = if live {
        let path = std::env::var_os("VOICE_ACCEPTANCE_GATEWAY_BOOTSTRAP").ok_or("live requires VOICE_ACCEPTANCE_GATEWAY_BOOTSTRAP pointing to an already configured ShiJu gateway bootstrap; no service request was sent")?;
        let bytes = std::fs::read(path)?;
        serde_json::from_slice::<vw_config_types::ui::GatewayClientSystemSettingsConfig>(&bytes)
            .map_err(|_| "invalid ShiJu gateway bootstrap; no service request was sent")?;
        Some(bytes)
    } else {
        None
    };
    let session = if live {
        std::env::var("VOICE_ACCEPTANCE_SESSION").map_err(
            |_| "live requires VOICE_ACCEPTANCE_SESSION for an existing isolated chat session",
        )?
    } else {
        "acceptance".into()
    };
    let _: vw_gateway_client::vw_api_types::voice::VoiceId =
        session.clone().try_into().map_err(|_| "invalid acceptance chat session ID")?;
    let storage = tempfile::tempdir()?;
    // SAFETY: before starting the Tokio runtime, Iced or any worker thread.
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", storage.path());
    }
    if let Some(bootstrap) = bootstrap {
        let dir = app::project_dirs().ok_or("isolated directories unavailable")?;
        std::fs::create_dir_all(dir.config_dir())?;
        std::fs::write(dir.config_dir().join("gateway-client-bootstrap.json"), bootstrap)?;
    } else {
        app::config::install_window_test_bootstrap(9);
    }
    let runtime = tokio::runtime::Runtime::new()?;
    let mut snapshot = Snapshot { phase: Phase::Speaking, ..Default::default() };
    if live {
        let endpoint = app::gateway_client_endpoint();
        if !endpoint.is_loopback() {
            return Err("live audio currently requires a loopback ShiJu gateway".into());
        }
        let config: VoiceConfig = runtime.block_on(async {
            tokio::time::timeout(
                Duration::from_secs(10),
                vw_gateway_client::GatewayClient::new(endpoint)?.voice_config(),
            )
            .await
            .map_err(|_| "gateway preflight timed out".to_owned())?
            .map_err(|e| e.to_string())
        })?;
        if !config.dictation_enabled || !config.realtime_enabled || config.provider_id.is_none() {
            return Err(
                "live requires configured dictation and realtime services on the gateway".into()
            );
        }
        use app::voice::permissions::{
            Permission, PermissionBackend, PermissionStatus, SystemPermissions,
        };
        if SystemPermissions.query(Permission::Microphone) != PermissionStatus::Granted {
            return Err("live requires user-granted microphone permission; no permission dialog was accepted automatically".into());
        }
        if app::voice::capture::input_devices()?.is_empty() {
            return Err("live requires a physical microphone".into());
        }
    } else {
        let captions = runtime.block_on(app::voice::acceptance_fixture::roundtrip())?;
        for (index, (role, text)) in captions.into_iter().enumerate() {
            snapshot.caption(role, format!("caption-{index}").try_into().unwrap(), 0, text, true);
        }
    }
    drop(runtime);
    let root =
        std::env::var_os("VOICE_ACCEPTANCE_OUTPUT").map(PathBuf::from).unwrap_or_else(|| {
            std::env::temp_dir().join(format!("voice19-window-{}", std::process::id()))
        });
    std::fs::create_dir_all(&root)?;
    eprintln!("VOICE_EVIDENCE {}", root.display());
    let confirmed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let outcome = confirmed.clone();
    iced::daemon(
        move || {
            let (id, task) = window::open(window::Settings {
                size: iced::Size::new(780., 760.),
                ..Default::default()
            });
            let mut app = App::new().0; // Deliberately do not poll bootstrap or history tasks.
            app.active_session_id = Some(session.clone());
            app.current_session_runtime_mut();
            app.show_system_settings = true;
            app.system_settings_tab = app::components::system_settings::SystemTab::Voice;
            let mut state = Harness {
                app,
                snapshot: snapshot.clone(),
                window: id,
                case: 0,
                frames: 0,
                root: root.clone(),
                live,
                heard: false,
                confirmed: confirmed.clone(),
                began: Instant::now(),
            };
            if !live {
                state.setup();
            }
            (state, task.map(Message::Opened))
        },
        Harness::update,
        Harness::view,
    )
    .title("ShiJu 语音隔离验收")
    .theme(|state: &Harness, _| state.theme())
    .subscription(|_| {
        iced::Subscription::batch([
            window::frames().map(|_| Message::Frame),
            window::close_events().map(Message::Closed),
            iced::time::every(Duration::from_secs(1)).map(|_| Message::Watchdog),
        ])
    })
    .settings(iced::Settings {
        fonts: fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()?;
    if !outcome.load(std::sync::atomic::Ordering::Acquire) {
        return Err("window closed before acceptance completed".into());
    }
    Ok(())
}
