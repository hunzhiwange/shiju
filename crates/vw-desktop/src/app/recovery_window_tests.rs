//! 显式主线程窗口验收入口：实际订阅、消息分发、聊天控件和窗口截图。
use crate::app::{
    AgentRequest, App, Message as AppMessage, message::chat::ChatMessage, models,
    state::AcpHistoryReplayMode,
};
use iced::{Element, Subscription, Task, Theme, window};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    process::{Child, Command},
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
enum Message {
    Ui(AppMessage),
    Opened(window::Id),
    Frame,
    Capture(window::Screenshot),
    Released,
    Next,
}

struct Harness {
    app: App,
    window: window::Id,
    root: PathBuf,
    model_port: u16,
    scenario: usize,
    capture: Option<&'static str>,
    frames: usize,
    cancel_ack: bool,
    waiting_seen: bool,
    results: Vec<Value>,
    started: Instant,
}

const CASES: [&str; 4] = ["transient", "missing", "exhausted", "cancel"];

impl Harness {
    fn begin(&mut self) -> Task<Message> {
        self.capture = None;
        self.cancel_ack = false;
        self.waiting_seen = false;
        self.started = Instant::now();
        let theme = if self.scenario < 4 { "light" } else { "dark" };
        self.app.app_theme = if theme == "light" { Theme::Light } else { Theme::Dark };
        let case = CASES[self.scenario % 4];
        let session = format!("window-{case}-{theme}");
        self.app.active_session_id = Some(session.clone());
        self.app.chat = vec![models::ChatMessage {
            role: models::ChatRole::User,
            content: format!("{case}-{theme}"),
            model: None,
            think_timing: vec![],
            processing_timing: None,
        }];
        self.app.chat_message_ids = vec![None];
        self.app.chat_render_cache.clear();
        self.app.active_toast = None;
        let request = Arc::new(AgentRequest {
            id: self.scenario as u64 + 100,
            session: session.clone(),
            query: format!("{case}-{theme}"),
            root: None,
            model: Some(format!(
                "recovery-window/{}",
                if case == "missing" { "missing" } else { "original" }
            )),
            reasoning_effort: None,
            acp_test: false,
            acp_agent: None,
            acp_allowed_tools: None,
            agent: None,
            allowed_tools: Some(vec![]),
            require_agent_tool: false,
            acp_force_new_session: false,
            acp_history_mode: AcpHistoryReplayMode::Discard,
            acp_recent_count: 0,
            full_access_enabled: false,
            resume_history_only: false,
            workflow_mode_enabled: false,
            goal_mode_enabled: false,
            history: vec![],
        });
        let runtime = self.app.get_session_runtime_mut(&session);
        runtime.active_agent_request = Some(request);
        runtime.is_requesting = true;
        self.app.sync_active_session_from_chat();
        eprintln!("WINDOW CASE {case}-{theme}");
        Task::none()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Opened(id) => {
                self.window = id;
                self.begin()
            }
            Message::Next => {
                self.scenario += 1;
                if self.scenario == 8 {
                    std::fs::write(
                        self.root.join("window-results.json"),
                        serde_json::to_vec_pretty(&self.results).unwrap(),
                    )
                    .unwrap();
                    return iced::exit();
                }
                self.begin()
            }
            Message::Ui(event) => {
                if let AppMessage::Chat(ChatMessage::AgentStreamStopFinished(id, result)) = &event {
                    assert_eq!(*id, self.scenario as u64 + 100);
                    assert!(
                        matches!(result, Ok(true)),
                        "Gateway must acknowledge explicit cancellation: {result:?}"
                    );
                    self.cancel_ack = true;
                }
                if let AppMessage::Chat(ChatMessage::AgentStreamError(_, error)) = &event {
                    eprintln!("WINDOW ERROR {error}");
                }
                let waiting = matches!(&event, AppMessage::Chat(ChatMessage::AgentModelRecovery(_, state)) if state.phase == vw_gateway_client::GatewayModelRecoveryPhase::Waiting);
                let terminal = matches!(
                    &event,
                    AppMessage::Chat(
                        ChatMessage::AgentStreamDone(..) | ChatMessage::AgentStreamError(..)
                    )
                );
                if matches!(&event, AppMessage::Chat(ChatMessage::AgentModelRecovery(_, state)) if state.phase == vw_gateway_client::GatewayModelRecoveryPhase::Recovered)
                {
                    assert!(self.waiting_seen, "recovery must follow waiting");
                }
                let task = self.app.update(event).map(Message::Ui);
                if waiting {
                    self.waiting_seen = true;
                    assert!(self.app.current_session_is_requesting());
                    assert!(
                        crate::app::components::chat_panel::message_view::recovery_notice_text(
                            &self.app
                        )
                        .is_some()
                    );
                    if self.capture.is_none() && (self.scenario % 4 == 0 || self.scenario % 4 == 3)
                    {
                        self.capture = Some("waiting");
                        self.frames = 0;
                    }
                }
                if terminal {
                    self.check_terminal();
                    self.capture = Some("terminal");
                    self.frames = 0;
                }
                task
            }
            Message::Frame => {
                assert!(
                    self.started.elapsed() < Duration::from_secs(75),
                    "window scenario timed out"
                );
                if self.capture == Some("terminal") && self.scenario % 4 == 3 && !self.cancel_ack {
                    return Task::none();
                }
                if self.capture.is_some() {
                    self.frames += 1;
                    if self.frames == 3 {
                        return window::screenshot(self.window).map(Message::Capture);
                    }
                }
                Task::none()
            }
            Message::Capture(screenshot) => {
                let stage = self.capture.take().expect("capture stage");
                let theme = if self.scenario < 4 { "light" } else { "dark" };
                let case = CASES[self.scenario % 4];
                let name = format!("{case}-{theme}-{stage}.png");
                assert!(screenshot.rgba.chunks_exact(4).any(|pixel| pixel[0] != pixel[1]));
                image::save_buffer(
                    self.root.join(&name),
                    &screenshot.rgba,
                    screenshot.size.width,
                    screenshot.size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                eprintln!("WINDOW CAPTURE {name}");
                if stage == "waiting" && case == "transient" {
                    let url = format!("http://127.0.0.1:{}/release", self.model_port);
                    return Task::perform(
                        async move {
                            reqwest::Client::new()
                                .post(url)
                                .send()
                                .await
                                .unwrap()
                                .error_for_status()
                                .unwrap();
                        },
                        |_| Message::Released,
                    );
                }
                if stage == "waiting" && case == "cancel" {
                    let task = self
                        .app
                        .update(AppMessage::Chat(ChatMessage::CancelPressed))
                        .map(Message::Ui);
                    assert!(!self.app.current_session_is_requesting());
                    assert!(
                        self.app.current_session_runtime_ref().unwrap().model_recovery.is_none(),
                        "cancel button must clear recovery state"
                    );
                    assert!(
                        self.app.active_toast.is_none(),
                        "cancel button must clear recovery toast"
                    );
                    self.capture = Some("terminal");
                    self.frames = 0;
                    return task;
                }
                if stage == "terminal" {
                    self.results.push(json!({"case":case,"theme":theme,"waiting_seen":self.waiting_seen,"screenshot":name,"chat":self.app.chat.iter().map(|m| &m.content).collect::<Vec<_>>() }));
                    return Task::done(Message::Next);
                }
                Task::none()
            }
            Message::Released => Task::none(),
        }
    }

    fn check_terminal(&self) {
        assert!(!self.app.current_session_is_requesting());
        assert!(self.app.current_session_runtime_ref().unwrap().model_recovery.is_none());
        let content = self
            .app
            .chat
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        match self.scenario % 4 {
            0 => {
                assert!(self.waiting_seen, "{content}");
                assert!(content.contains("瞬态故障后恢复成功"), "{content}");
            }
            1 => {
                assert!(!self.waiting_seen);
                assert!(content.contains("未重试"), "{content}");
            }
            2 => {
                assert!(self.waiting_seen, "{content}");
                assert!(content.contains("已尝试 3 次"), "{content}");
            }
            _ => unreachable!(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.app.agent_stream_subscription().map(Message::Ui),
            window::frames().map(|_| Message::Frame),
        ])
    }

    fn view(&self, _: window::Id) -> Element<'_, Message> {
        let content: Element<'_, AppMessage> = iced::widget::column![
            crate::app::components::chat_panel::view(&self.app),
            crate::app::components::input_panel::view(&self.app)
        ]
        .padding(16)
        .spacing(8)
        .into();
        let overlay: Element<'_, AppMessage> = iced::widget::stack![
            content,
            iced::widget::container(crate::app::components::toast::view(&self.app))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding(iced::Padding::default().right(12).bottom(44)),
        ]
        .into();
        overlay.map(Message::Ui)
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
    assert_eq!(
        args.next().as_deref(),
        Some("--gateway-test-binary"),
        "explicit gateway fixture executable is required"
    );
    let backend = args.next().expect("gateway test executable");
    let root = PathBuf::from(args.next().expect("existing absolute evidence directory"));
    assert!(root.is_absolute() && root.is_dir());
    std::env::set_current_dir(&root).expect("use isolated project directory");
    let child = Command::new(backend)
        .args([
            "gateway::recovery_window_tests::recovery_window_gateway_fixture",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("VW_MODEL_RECOVERY_TEST_ROOT", &root)
        .env("VW_RECOVERY_WINDOW_HTTP", "1")
        .env("VIBEWINDOW_CONFIG_DIR", root.join("config"))
        .env_remove("VIBEWINDOW_WORKSPACE")
        .env_remove("VIBEWINDOW_SERVER_PASSWORD")
        .env("VW_RECOVERY_WINDOW_API_KEY", "fixture-only")
        .spawn()
        .unwrap();
    let _fixture = Fixture { child, root: root.clone() };
    let deadline = Instant::now() + Duration::from_secs(30);
    while !root.join("ready.json").exists() {
        assert!(Instant::now() < deadline, "gateway readiness timeout");
        std::thread::sleep(Duration::from_millis(20));
    }
    let ready: Value =
        serde_json::from_slice(&std::fs::read(root.join("ready.json")).unwrap()).unwrap();
    crate::app::config::install_window_test_bootstrap(
        ready["gateway_port"].as_u64().unwrap() as u16
    );
    let model_port = ready["model_port"].as_u64().unwrap() as u16;
    iced::daemon(
        move || {
            let (app, _) = App::new();
            let (id, task) = window::open(window::Settings {
                size: iced::Size::new(1000.0, 760.0),
                exit_on_close_request: false,
                ..Default::default()
            });
            (
                Harness {
                    app,
                    window: id,
                    root: root.clone(),
                    model_port,
                    scenario: 0,
                    capture: None,
                    frames: 0,
                    cancel_ack: false,
                    waiting_seen: false,
                    results: vec![],
                    started: Instant::now(),
                },
                task.map(Message::Opened),
            )
        },
        Harness::update,
        Harness::view,
    )
    .title("Recovery window validation")
    .theme(|state: &Harness, _| state.app.app_theme.clone())
    .subscription(Harness::subscription)
    .settings(iced::Settings {
        fonts: crate::fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let stats: Value = runtime.block_on(async {
        reqwest::get(format!("http://127.0.0.1:{model_port}/stats"))
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    });
    for theme in ["light", "dark"] {
        assert_eq!(stats[format!("transient-{theme}")], 2);
        assert_eq!(stats[format!("exhausted-{theme}")], 3);
        assert_eq!(stats[format!("cancel-{theme}")], 1);
        assert_eq!(stats.get(format!("missing-{theme}")).and_then(Value::as_u64).unwrap_or(0), 0);
    }
    std::fs::write(
        _fixture.root.join("model-calls.json"),
        serde_json::to_vec_pretty(&stats).unwrap(),
    )
    .unwrap();
    eprintln!("WINDOW VALIDATION: 8 scenarios passed; screenshots and real HTTP counts recorded");
}
