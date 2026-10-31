//! Actual native Iced drawing and user-action automation through the production form path.
use crate::app::impls::mcp_form_stream::{FormRun, watch};
use crate::app::message::mcp_form::{McpFormMessage, current};
use crate::app::{AgentRequest, App, Message as AppMessage, state::AcpHistoryReplayMode};
use iced::{Element, Subscription, Task, Theme, window};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use vw_gateway_client::{GatewayClient, vw_api_types::mcp_form::McpFormAction};

#[derive(Debug, Clone)]
enum Message {
    Ui(AppMessage),
    Opened(window::Id),
    Host(Arc<FormRun>),
    Result(Value),
    Frame,
    Capture(usize, Stage, window::Screenshot),
    Next,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Waiting,
    Top,
    Fields,
    Bottom,
    Reply,
    Finished,
}
struct Harness {
    app: App,
    root: PathBuf,
    port: u16,
    window: window::Id,
    case: usize,
    stage: Stage,
    frames: usize,
    capture_pending: bool,
    started: Instant,
    guard: Option<Arc<FormRun>>,
    results: Vec<Value>,
}
fn configure(app: &mut App, case: usize, root: &std::path::Path) {
    let session = format!("中文会话-{case}");
    app.recent_projects.clear();
    app.recent_projects_meta.clear();
    app.recent_projects_edits.clear();
    app.active_session_id = Some(session.clone());
    app.app_theme = if case < 3 { Theme::Light } else { Theme::Dark };
    let runtime = app.get_session_runtime_mut(&session);
    runtime.is_requesting = true;
    runtime.active_agent_request = Some(Arc::new(AgentRequest {
        id: case as u64 + 25,
        session,
        query: String::new(),
        root: Some(root.display().to_string()),
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
        history: vec![],
    }));
}
impl Harness {
    fn begin(&mut self) -> Task<Message> {
        self.stage = Stage::Waiting;
        self.frames = 0;
        self.capture_pending = false;
        self.app.active_toast = None;
        self.started = Instant::now();
        configure(&mut self.app, self.case, &self.root);
        let session = self.app.active_session_id.clone().unwrap();
        let root = self.root.clone();
        let port = self.port;
        let run_id = self.case as u64 + 25;
        let client =
            Arc::new(GatewayClient::new(crate::app::config::gateway_client_endpoint()).unwrap());
        Task::run(
            iced::stream::channel(
                64,
                move |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
                    use iced::futures::SinkExt;
                    let (form_sender, mut form_receiver) =
                        iced::futures::channel::mpsc::channel(32);
                    let guard = Arc::new(
                        watch(
                            client,
                            run_id,
                            session.clone(),
                            Some(root.display().to_string()),
                            form_sender,
                        )
                        .await
                        .unwrap(),
                    );
                    sender.send(Message::Host(guard)).await.unwrap();
                    let call = reqwest::Client::new()
                        .post(format!("http://127.0.0.1:{port}/trigger"))
                        .json(&json!({"session":session}))
                        .send();
                    tokio::pin!(call);
                    use iced::futures::StreamExt;
                    loop {
                        tokio::select! {
                            event = form_receiver.next() => if let Some(event) = event { if sender.send(Message::Ui(event)).await.is_err() { break; } } else { break; },
                            response = &mut call => {
                                let result: Value = response.unwrap().error_for_status().unwrap().json().await.unwrap();
                                let _ = sender.send(Message::Result(result)).await; break;
                            }
                        }
                    }
                },
            ),
            |m| m,
        )
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Opened(id) => {
                self.window = id;
                self.begin()
            }
            Message::Host(guard) => {
                self.guard = Some(guard);
                Task::none()
            }
            Message::Ui(event) => self.app.update(event).map(Message::Ui),
            Message::Next => {
                self.case += 1;
                if self.case == 6 {
                    std::fs::write(
                        self.root.join("window-results.json"),
                        serde_json::to_vec_pretty(&self.results).unwrap(),
                    )
                    .unwrap();
                    return iced::exit();
                }
                self.begin()
            }
            Message::Frame => {
                assert!(
                    self.started.elapsed() < Duration::from_secs(90),
                    "native form flow timed out"
                );
                if self.stage == Stage::Waiting && current(&self.app).is_some() {
                    self.stage = Stage::Top;
                    self.frames = 0;
                }
                if matches!(
                    self.stage,
                    Stage::Top | Stage::Fields | Stage::Bottom | Stage::Finished
                ) {
                    self.frames += 1;
                    if self.frames == 4 && !self.capture_pending {
                        self.capture_pending = true;
                        let case = self.case;
                        let stage = self.stage;
                        return window::screenshot(self.window)
                            .map(move |image| Message::Capture(case, stage, image));
                    }
                }
                Task::none()
            }
            Message::Capture(case, capture_stage, screenshot) => {
                if case != self.case || capture_stage != self.stage {
                    return Task::none();
                }
                self.capture_pending = false;
                let theme = if self.case < 3 { "light" } else { "dark" };
                let action = ["accept", "decline", "cancel"][self.case % 3];
                let stage = match self.stage {
                    Stage::Top => "top",
                    Stage::Fields => "fields",
                    Stage::Bottom => "bottom",
                    Stage::Finished => "finished",
                    _ => panic!("unexpected capture"),
                };
                let name = format!("{theme}-{action}-{stage}.png");
                image::save_buffer(
                    self.root.join(&name),
                    &screenshot.rgba,
                    screenshot.size.width,
                    screenshot.size.height,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                eprintln!(
                    "MCP FORM WINDOW {name}: {}x{}; glyph shaping completed",
                    screenshot.size.width, screenshot.size.height
                );
                self.frames = 0;
                if self.stage == Stage::Finished {
                    self.stage = Stage::Reply;
                    return Task::done(Message::Next);
                }
                if self.stage == Stage::Top {
                    let id = current(&self.app).unwrap().0.id.clone();
                    for (field, value) in [
                        ("name", "用户甲"),
                        ("age", "24"),
                        ("approved", "false"),
                        ("choice", "raw-b"),
                    ] {
                        let _ = self.app.update(AppMessage::McpForm(McpFormMessage::Changed {
                            id: id.clone(),
                            field: field.into(),
                            value: value.into(),
                        }));
                    }
                    self.stage = Stage::Fields;
                    return iced::widget::operation::snap_to(
                        iced::widget::Id::new("mcp-form-content"),
                        iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(0.13) },
                    );
                }
                if self.stage == Stage::Fields {
                    self.stage = Stage::Bottom;
                    return iced::widget::operation::snap_to(
                        iced::widget::Id::new("mcp-form-content"),
                        iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(1.0) },
                    );
                }
                self.stage = Stage::Reply;
                let id = current(&self.app).unwrap().0.id.clone();
                let event = match self.case % 3 {
                    0 => AppMessage::McpForm(McpFormMessage::Submit {
                        id,
                        action: McpFormAction::Accept,
                    }),
                    1 => AppMessage::McpForm(McpFormMessage::Submit {
                        id,
                        action: McpFormAction::Decline,
                    }),
                    _ => AppMessage::View(crate::app::message::ViewMessage::GlobalKeyPressed(
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                        iced::keyboard::Modifiers::NONE,
                    )),
                };
                self.app.update(event).map(Message::Ui)
            }
            Message::Result(result) => {
                let action = ["accept", "decline", "cancel"][self.case % 3];
                assert_eq!(result["form_reply"]["result"]["action"], action);
                if action == "accept" {
                    assert_eq!(
                        result["form_reply"]["result"]["content"],
                        json!({"name":"用户甲","age":24,"approved":false,"choice":"raw-b"})
                    );
                } else {
                    assert!(result["form_reply"]["result"].get("content").is_none());
                }
                self.results.push(json!({"case":self.case,"theme":if self.case<3 {"light"}else{"dark"},"wire_result":result["form_reply"]["result"],"native_rendered":true}));
                self.guard.take();
                let session = self.app.active_session_id.clone().unwrap();
                self.app.get_session_runtime_mut(&session).is_requesting = false;
                let _ = self.app.update(AppMessage::None);
                assert!(current(&self.app).is_none());
                assert!(self.app.mcp_forms.drafts.is_empty());
                self.stage = Stage::Finished;
                self.frames = 0;
                Task::none()
            }
        }
    }
    fn view(&self, _: window::Id) -> Element<'_, Message> {
        self.app.view().map(Message::Ui)
    }
    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(60)).map(|_| Message::Frame)
    }
}
pub(crate) fn run() {
    let mut args = std::env::args().skip(1);
    let port: u16 = args.next().expect("explicit fixture port").parse().unwrap();
    let root = PathBuf::from(args.next().expect("explicit absolute output directory"));
    assert!(root.is_absolute() && root.is_dir());
    std::env::set_current_dir(&root).unwrap();
    crate::app::config::install_window_test_bootstrap(port);
    iced::daemon(
        move || {
            let (app, _) = App::new();
            let (window, task) = window::open(window::Settings {
                size: iced::Size::new(1000.0, 800.0),
                exit_on_close_request: false,
                ..Default::default()
            });
            (
                Harness {
                    app,
                    root: root.clone(),
                    port,
                    window,
                    case: 0,
                    stage: Stage::Waiting,
                    frames: 0,
                    capture_pending: false,
                    started: Instant::now(),
                    guard: None,
                    results: vec![],
                },
                task.map(Message::Opened),
            )
        },
        Harness::update,
        Harness::view,
    )
    .title("MCP form native validation")
    .theme(|h: &Harness, _| h.app.app_theme.clone())
    .subscription(Harness::subscription)
    .settings(iced::Settings {
        fonts: crate::fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
    eprintln!(
        "MCP FORM WINDOW: 6 real form responses and native light/dark Chinese/long-content renders passed"
    );
}
