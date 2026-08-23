//! 验证会话命令与提示词请求通过 Actor 通道转发和返回结果。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tokio::sync::mpsc;

use super::*;

fn client_with_actor() -> (AcpClient, mpsc::UnboundedReceiver<ActorCommand>) {
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let client = AcpClient::new(
        "test-agent",
        AcpAgentConfig { command: "unused".to_string(), args: Vec::new(), env: HashMap::new() },
    );
    client.actor_state.lock().handle = Some(AcpClientActorHandle { command_tx, thread: None });

    (client, command_rx)
}

fn assert_cwd(actual: &Path, expected: &Path) {
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn session_commands_forward_payloads_and_return_actor_responses() {
    let (client, mut command_rx) = client_with_actor();
    let cwd = PathBuf::from("/tmp/vw-acp-commands");

    let create = client.create_session(&cwd);
    let create_actor = async {
        match command_rx.recv().await.expect("create command") {
            ActorCommand::CreateSession { cwd: actual_cwd, response_tx } => {
                assert_cwd(&actual_cwd, &cwd);
                response_tx
                    .send(Ok(SessionInfo {
                        session_id: "created".to_string(),
                        models: None,
                        config_options: None,
                    }))
                    .expect("create response should send");
            }
            _ => panic!("expected create session command"),
        }
    };
    let (created, _) = tokio::join!(create, create_actor);
    assert_eq!(created.expect("created session").session_id, "created");

    let load = client.load_session("load-id", &cwd);
    let load_actor = async {
        match command_rx.recv().await.expect("load command") {
            ActorCommand::LoadSession { session_id, cwd: actual_cwd, response_tx } => {
                assert_eq!(session_id, "load-id");
                assert_cwd(&actual_cwd, &cwd);
                response_tx
                    .send(Ok(SessionInfo {
                        session_id: "loaded".to_string(),
                        models: None,
                        config_options: None,
                    }))
                    .expect("load response should send");
            }
            _ => panic!("expected load session command"),
        }
    };
    let (loaded, _) = tokio::join!(load, load_actor);
    assert_eq!(loaded.expect("loaded session").session_id, "loaded");

    let resume = client.resume_session("resume-id", &cwd);
    let resume_actor = async {
        match command_rx.recv().await.expect("resume command") {
            ActorCommand::ResumeSession { session_id, cwd: actual_cwd, response_tx } => {
                assert_eq!(session_id, "resume-id");
                assert_cwd(&actual_cwd, &cwd);
                response_tx
                    .send(Ok(SessionInfo {
                        session_id: "resumed".to_string(),
                        models: None,
                        config_options: None,
                    }))
                    .expect("resume response should send");
            }
            _ => panic!("expected resume session command"),
        }
    };
    let (resumed, _) = tokio::join!(resume, resume_actor);
    assert_eq!(resumed.expect("resumed session").session_id, "resumed");

    let set_mode = client.set_session_mode("mode-session", &cwd, "plan");
    let mode_actor = async {
        match command_rx.recv().await.expect("mode command") {
            ActorCommand::SetSessionMode { session_id, cwd: actual_cwd, mode_id, response_tx } => {
                assert_eq!(session_id, "mode-session");
                assert_cwd(&actual_cwd, &cwd);
                assert_eq!(mode_id, "plan");
                response_tx.send(Ok(())).expect("mode response should send");
            }
            _ => panic!("expected set session mode command"),
        }
    };
    let (mode_result, _) = tokio::join!(set_mode, mode_actor);
    mode_result.expect("mode should be set");

    let set_config = client.set_session_config_option("config-session", &cwd, "effort", "high");
    let config_actor = async {
        match command_rx.recv().await.expect("config command") {
            ActorCommand::SetSessionConfigOption {
                session_id,
                cwd: actual_cwd,
                option_name,
                value_id,
                response_tx,
            } => {
                assert_eq!(session_id, "config-session");
                assert_cwd(&actual_cwd, &cwd);
                assert_eq!(option_name, "effort");
                assert_eq!(value_id, "high");
                response_tx
                    .send(Ok(acp::SetSessionConfigOptionResponse::new(Vec::new())))
                    .expect("config response should send");
            }
            _ => panic!("expected set config option command"),
        }
    };
    let (config_result, _) = tokio::join!(set_config, config_actor);
    assert!(config_result.expect("config should be set").config_options.is_empty());

    let set_model = client.set_session_model("model-session", &cwd, "sonnet");
    let model_actor = async {
        match command_rx.recv().await.expect("model command") {
            ActorCommand::SetSessionModel { session_id, cwd: actual_cwd, model, response_tx } => {
                assert_eq!(session_id, "model-session");
                assert_cwd(&actual_cwd, &cwd);
                assert_eq!(model, "sonnet");
                response_tx.send(Ok(())).expect("model response should send");
            }
            _ => panic!("expected set model command"),
        }
    };
    let (model_result, _) = tokio::join!(set_model, model_actor);
    model_result.expect("model should be set");
}

#[tokio::test]
async fn session_command_returns_actor_error_response() {
    let (client, mut command_rx) = client_with_actor();

    let call = client.set_session_model("session", ".", "missing-model");
    let actor = async {
        match command_rx.recv().await.expect("model command") {
            ActorCommand::SetSessionModel { response_tx, .. } => response_tx
                .send(Err(AcpError::SetSessionModel("model rejected".to_string())))
                .expect("error response should send"),
            _ => panic!("expected set model command"),
        }
    };

    let (result, _) = tokio::join!(call, actor);
    assert!(
        matches!(result, Err(AcpError::SetSessionModel(message)) if message == "model rejected")
    );
}

#[tokio::test]
async fn run_prompt_forwards_events_and_returns_prompt_result() {
    let (client, mut command_rx) = client_with_actor();
    let request = PromptRequest::new(PathBuf::from("/tmp/vw-acp-prompt"), "hello");
    let mut events = Vec::new();
    let mut on_event = |event| events.push(event);

    let call = client.run_prompt(request.clone(), &mut on_event);
    let actor = async {
        match command_rx.recv().await.expect("prompt command") {
            ActorCommand::RunPrompt { request: actual_request, event_tx, response_tx } => {
                assert_eq!(actual_request.cwd, request.cwd);
                assert_eq!(actual_request.prompt, request.prompt);
                event_tx
                    .send(PromptEvent::TextDelta("one".to_string()))
                    .expect("text event should send");
                event_tx
                    .send(PromptEvent::SessionChanged {
                        expected: "old-session".to_string(),
                        actual: "new-session".to_string(),
                    })
                    .expect("session event should send");
                response_tx
                    .send(Ok(PromptResult {
                        session_id: "new-session".to_string(),
                        deltas: vec!["one".to_string()],
                        finish_reason: Some("end_turn".to_string()),
                        usage: Some(PromptUsage {
                            input_tokens: 1,
                            output_tokens: 2,
                            cached_tokens: 3,
                            reasoning_tokens: 4,
                        }),
                    }))
                    .expect("prompt response should send");
            }
            _ => panic!("expected run prompt command"),
        }
    };

    let (result, _) = tokio::join!(call, actor);
    let result = result.expect("prompt should complete");

    assert_eq!(result.session_id, "new-session");
    assert_eq!(result.deltas, vec!["one"]);
    assert_eq!(result.finish_reason.as_deref(), Some("end_turn"));
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], PromptEvent::TextDelta(ref delta) if delta == "one"));
    assert!(matches!(
        events[1],
        PromptEvent::SessionChanged { ref expected, ref actual }
            if expected == "old-session" && actual == "new-session"
    ));
}

#[tokio::test]
async fn run_prompt_delivers_buffered_events_before_returning_response() {
    let (client, mut command_rx) = client_with_actor();
    let mut events = Vec::new();
    let mut on_event = |event| events.push(event);

    let call = client.run_prompt(PromptRequest::new(PathBuf::from("."), "hello"), &mut on_event);
    let actor = async {
        match command_rx.recv().await.expect("prompt command") {
            ActorCommand::RunPrompt { event_tx, response_tx, .. } => {
                response_tx
                    .send(Ok(PromptResult {
                        session_id: "completed-session".to_string(),
                        deltas: Vec::new(),
                        finish_reason: Some("stop".to_string()),
                        usage: None,
                    }))
                    .expect("prompt response should send");
                event_tx
                    .send(PromptEvent::TextDelta("buffered".to_string()))
                    .expect("buffered event should send");
            }
            _ => panic!("expected run prompt command"),
        }
    };

    let (result, _) = tokio::join!(call, actor);

    assert_eq!(result.expect("prompt result").session_id, "completed-session");
    assert!(matches!(events.as_slice(), [PromptEvent::TextDelta(delta)] if delta == "buffered"));
}

#[tokio::test]
async fn run_prompt_invalidates_actor_when_response_channel_closes() {
    let (client, mut command_rx) = client_with_actor();
    let mut on_event = |_| {};

    let call = client.run_prompt(PromptRequest::new(PathBuf::from("."), "hello"), &mut on_event);
    let actor = async {
        match command_rx.recv().await.expect("prompt command") {
            ActorCommand::RunPrompt { response_tx, .. } => drop(response_tx),
            _ => panic!("expected run prompt command"),
        }
    };

    let (result, _) = tokio::join!(call, actor);
    assert!(matches!(result, Err(AcpError::PromptJoin(_))));
    assert!(client.actor_state.lock().handle.is_none());
}

#[tokio::test]
async fn run_prompt_waits_for_response_after_event_channel_closes() {
    let (client, mut command_rx) = client_with_actor();
    let mut on_event = |_| {};

    let call = client.run_prompt(PromptRequest::new(PathBuf::from("."), "hello"), &mut on_event);
    let actor = async {
        match command_rx.recv().await.expect("prompt command") {
            ActorCommand::RunPrompt { event_tx, response_tx, .. } => {
                drop(event_tx);
                tokio::task::yield_now().await;
                response_tx
                    .send(Ok(PromptResult {
                        session_id: "session-after-events".to_string(),
                        deltas: Vec::new(),
                        finish_reason: None,
                        usage: None,
                    }))
                    .expect("prompt response should send");
            }
            _ => panic!("expected run prompt command"),
        }
    };

    let (result, _) = tokio::join!(call, actor);
    assert_eq!(result.expect("prompt result").session_id, "session-after-events");
}
