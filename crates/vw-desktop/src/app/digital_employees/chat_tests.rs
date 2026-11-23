use super::super::{assistants, chat, shared, shared_api};
use super::*;
use crate::app::{
    Screen,
    config::desktop_test_gateway::TestServer,
    message::{
        ChatMessage,
        project::{PreparedSessionOpenSource, ProjectMessage},
    },
};
use iced::futures::StreamExt;
use iced_runtime::{Action, task::into_stream};
use serde_json::{Value, json};
use std::sync::Arc;

const ACCOUNT: &str = r#"{"user_id":"103","company_id":"42","account_id":"fixture","account_name":"fixture","name":"测试账号","source_type":"local","external_user_id":"","allow_console_login":true,"status":"enabled"}"#;

fn signed_in_app(port: u16) -> App {
    let server = config_ui::GatewayClientServerConfig {
        port,
        ..Default::default()
    };
    let id = server.id.clone();
    crate::app::update_gateway_client_config(move |config| config.set_servers(vec![server], id))
        .unwrap();

    use crate::app::account::{
        AccountIdentity, AccountOrigin, AccountRepository, SecretSkey, VerifiedAccount,
    };
    let mut repository = AccountRepository::default();
    repository.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::new("42", "测试公司", "103", "enabled").unwrap(),
            SecretSkey::new("fixture-login").unwrap(),
        ),
        AccountOrigin::ManualSkey,
    );
    let (mut app, _) = App::new();
    app.account.replace_repository(repository);
    app.navigation_tabs.screen = Screen::DigitalEmployees;
    app
}

async fn output(task: Task<Message>) -> Message {
    let mut stream = into_stream(task).expect("message task");
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if let Action::Output(message) = stream.next().await.expect("task output") {
                return message;
            }
        }
    })
    .await
    .expect("gateway fixture response")
}

async fn check_entry(kind: &str) {
    let (_lock, home, _guard) = crate::app::config::desktop_test_gateway::temp_home();
    let server = TestServer::start_matching(
        3,
        |request| request.path.starts_with("/v1/chat/") || request.path == "/v1/shiju/account/me",
        |request| {
            if request.path == "/v1/shiju/account/me" {
                return (200, ACCOUNT);
            }
            if request.path.ends_with("/stop") {
                return (200, r#"{"cancelled":true}"#);
            }
            (
                200,
                concat!(
                    "id: 1\ndata: {\"type\":\"chat.delta\",\"delta\":\"员工回复\"}\n\n",
                    "id: 2\ndata: {\"type\":\"chat.done\",\"message_id\":\"assistant-1\",\"parent_message_id\":\"user-1\"}\n\n",
                ),
            )
        },
    );
    let mut app = signed_in_app(server.port);
    let project_path = home.path().to_string_lossy().into_owned();
    app.navigation_tabs.project_path = Some(project_path.clone());
    app.workspace_state.project_id = Some("fixture-project".into());

    let event = if kind == "private" {
        app.digital_employees.page = Page::Assistants;
        app.digital_employees.assistants.data = Some(Arc::new(assistants::Snapshot {
            mine: vec![
                serde_json::from_value(json!({
                    "employee_id":"worker-1","name":"测试员工","department_path":"",
                    "owner_user_id":"103","created_by":"103","owned_by_caller":true,
                    "publish_status":"PUBLISHED","status":"ACTIVE","runtime_status":"on_duty",
                    "capability_summary":{"skill_count":0,"mcp_count":0},"usage_count":0,
                }))
                .unwrap(),
            ],
            governance: Ok(None),
            members: Default::default(),
        }));
        Event::Assistants(assistants::ListEvent::Chat("worker-1".into()))
    } else {
        app.digital_employees.page = Page::Shared;
        app.digital_employees.shared.directory =
            Some(Arc::new(shared_api::Directory {
                employees: vec![serde_json::from_value(json!({
                "employee_id":"worker-1","name":"测试员工","visibility":"org","status":"ACTIVE",
                "publish_status":"PUBLISHED","active_version_id":"version-1",
            })).unwrap()],
                access: Default::default(),
                templates: None,
                counts: None,
            }));
        Event::Shared(shared::ListEvent::Chat("worker-1".into()))
    };
    let response = output(app.update(Message::DigitalEmployees(event))).await;
    let _ = app.update(response);
    assert_eq!(app.current_session_runtime().agent_label.as_deref(), Some("测试员工"));

    app.chat_view.active_session_view_state.base_ready = true;
    app.workspace_state.project_id = Some("fixture-project".into());
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("你好");
    let _ = app.update(Message::Chat(ChatMessage::SendPressed));
    let runtime = app.current_session_runtime();
    let request = runtime.request_lifecycle.active_agent_request.expect("employee chat request");
    let endpoint = crate::app::config::gateway_client_endpoint();
    let credential = chat::credential(&app, &endpoint).expect("matched account");
    let mut stream = desktop_session::agent_stream::employee_agent_stream(
        &request,
        crate::app::config::config_access(),
        endpoint,
        credential,
        runtime.employee_target,
    );
    let mut done = None;
    let mut delta = false;
    while let Some(event) = stream.next().await {
        let message = ChatMessage::from(event);
        match message {
            ChatMessage::AgentStreamDone(_, usage, assistant, parent) => {
                done = Some((usage, assistant, parent));
                break;
            }
            ChatMessage::AgentStreamDelta(_, ref text) => {
                delta |= text == "员工回复";
                let _ = app.update(Message::Chat(message));
            }
            ChatMessage::AgentStreamError(_, ref error) => panic!("{error}"),
            _ => {}
        }
    }
    assert!(delta);
    let (usage, assistant, parent) = done.expect("completed employee stream");
    let stopped = output(app.update(Message::Chat(ChatMessage::CancelPressed))).await;
    assert!(matches!(stopped, Message::Chat(ChatMessage::AgentStreamStopFinished(_, Ok(true)))));
    let recorded = server.take_requests();
    let body: Value = serde_json::from_str(&recorded[1].body).unwrap();
    assert_eq!(body["agent"], "acp-employee-acp");
    assert_eq!(body["session_id"], request.session);
    assert_eq!(body["options"]["employee_target"], json!({"employee_id":"worker-1","kind":kind}));
    assert_eq!(body["messages"], json!([{"role":"user","content":"你好"}]));
    assert!(recorded[2].path.ends_with("/stop"));

    let info = session_info::Info {
        id: request.session.clone(),
        slug: "fixture".into(),
        project_id: "fixture-project".into(),
        directory: project_path.clone(),
        parent_id: None,
        summary: None,
        share: None,
        title: "员工会话".into(),
        version: "0.0.0".into(),
        time: session_info::TimeInfo { created: 1, updated: 2, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    };
    let _ = app.update(Message::Chat(ChatMessage::DraftResponseReloaded {
        id: request.id,
        usage,
        assistant_message_id: assistant,
        parent_message_id: parent,
        publication: Some(Box::new(
            desktop_session::message::chat::stream::draft::DraftPublication { info, preview: None },
        )),
    }));
    assert!(app.session_state.sessions.iter().any(|info| info.id == request.session));

    let messages = app.chat_view.chat.clone();
    let ids = app.chat_view.chat_message_ids.clone();
    assert_eq!(messages.len(), 2);
    assert_eq!(ids, [Some("user-1".into()), Some("assistant-1".into())]);
    let _ = crate::app::message::project::open_session_draft(
        &mut app,
        project_path.clone(),
        project_path.clone(),
    );
    app.session_state.session_runtime_states.remove(&request.session);
    app.session_state.session_chat_cache.remove(&request.session);
    app.session_state.session_chat_message_id_cache.remove(&request.session);
    app.session_state.session_view_cache.remove(&request.session);
    let _ = app.update(Message::Project(ProjectMessage::OpenProjectSessionPressed(
        project_path.clone(),
        request.session.clone(),
    )));
    assert_eq!(app.session_state.active_session_id.as_deref(), Some(request.session.as_str()));
    let mut prepared = crate::app::session::prepare_session_open(
        request.session.clone(),
        messages,
        ids.clone(),
        Default::default(),
        Vec::new(),
        Vec::new(),
        2,
        None,
        false,
    );
    prepared.agent_binding = Some(vw_api_chat_identity::SessionAgentIdentity {
        agent: EMPLOYEE_CHAT_PROFILE.into(),
        label: Some("数字员工 · worker-1".into()),
        acp_agent: Some("employee-acp".into()),
    });
    let open_generation = app.session_state.session_view_cache.begin_open(&request.session);
    for _ in 0..2 {
        let _ = app.update(Message::Project(ProjectMessage::SessionOpenPrepared {
            target_session_id: request.session.clone(),
            project_path: project_path.clone(),
            open_generation,
            source: PreparedSessionOpenSource::RemoteHistory,
            result: Ok(prepared.clone()),
        }));
    }
    assert_eq!(app.chat_view.chat_message_ids, ids);
    assert!(app.current_session_runtime().is_employee_chat());
    assert!(app.current_session_runtime().employee_target.is_none());
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("你好");
    let _ = app.update(Message::Chat(ChatMessage::SendPressed));
    let continuation = app
        .current_session_runtime()
        .request_lifecycle
        .active_agent_request
        .expect("continued employee chat");
    assert_eq!(continuation.session, request.session);
    assert_ne!(continuation.id, request.id);
    assert_eq!(
        app.chat_view
            .chat
            .iter()
            .filter(|message| {
                message.role == crate::app::models::ChatRole::User && message.content == "你好"
            })
            .count(),
        2
    );

    let _ = app.update(Message::Account(crate::app::message::AccountMessage::RepositoryLoaded(
        Ok(Default::default()),
    )));
    let _ =
        app.update(Message::Chat(ChatMessage::AgentStreamDelta(continuation.id, "旧回复".into())));
    let _ = app
        .update(Message::Chat(ChatMessage::AgentStreamError(continuation.id, "旧会话错误".into())));
    assert!(
        app.toast_state
            .active_toast
            .as_ref()
            .is_none_or(|toast| !toast.message.contains("旧会话错误"))
    );
    assert!(app.session_state.active_session_id.is_none());
    assert!(app.chat_view.chat.is_empty());
}

#[tokio::test]
async fn employee_acp_desktop_private_chat_request_and_stop() {
    check_entry("private").await;
}

#[tokio::test]
async fn employee_acp_desktop_shared_chat_request_and_stop() {
    check_entry("shared").await;
}

#[tokio::test]
async fn employee_acp_desktop_chat_action_has_readable_text_in_both_themes() {
    use desktop_ui_controls::system_settings_common::rounded_action_btn_style;
    use iced::advanced::{
        clipboard,
        renderer::{Headless, Style},
    };
    use iced::{Background, Font, Point, Renderer, Size, Theme, mouse, widget::button::Status};
    use iced_runtime::user_interface::{Cache, UserInterface};

    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let size = Size::new(80.0, 50.0);
    for theme in [Theme::Light, Theme::Dark] {
        let style = rounded_action_btn_style(&theme, Status::Active);
        let Some(Background::Color(background)) = style.background else {
            panic!("chat action needs a visible background");
        };
        assert!(style.text_color.is_readable_on(background));

        let button =
            super::super::view::chat_action(Some(Message::DigitalEmployees(Event::Refresh)));
        let mut ui = UserInterface::build(button, size, Cache::new(), &mut renderer);
        let mut messages = Vec::new();
        let _ = ui.update(
            &[iced::Event::Window(
                iced::window::Event::RedrawRequested(iced::time::Instant::now()),
            )],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        assert!(messages.is_empty());
        ui.draw(
            &mut renderer,
            &theme,
            &Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        let pixels = renderer.screenshot(Size::new(160, 100), 2.0, theme.palette().background);
        let [red, green, blue, _] = style.text_color.into_rgba8();
        // The centered glyphs are inside the padding, away from the rounded border.
        let glyph_pixels = pixels
            .chunks_exact(160 * 4)
            .skip(16)
            .take(42)
            .flat_map(|row| row.chunks_exact(4).skip(24).take(56))
            .filter(|pixel| {
                pixel[0].abs_diff(red) <= 24
                    && pixel[1].abs_diff(green) <= 24
                    && pixel[2].abs_diff(blue) <= 24
            })
            .count();
        assert!(glyph_pixels > 20, "{theme:?}: found {glyph_pixels} legible text pixels");
        let _ = ui.update(
            &[
                iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(Point::new(25.0, 15.0)),
            &mut renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        assert!(matches!(messages.as_slice(), [Message::DigitalEmployees(Event::Refresh)]));
    }
}
