//! 验证客户端 Actor 句柄的复用、失效、关闭和请求响应通道行为。
//!
//! 最小客户端、通道和运行时 fixture 用于显式驱动异步状态转换与资源收尾。

use std::collections::HashMap;

use tokio::sync::{mpsc, oneshot};

use crate::{AcpAgentConfig, AcpError};

use super::{AcpClient, AcpClientActorHandle, ActorCommand};

fn client() -> AcpClient {
    AcpClient::new(
        "test-agent",
        AcpAgentConfig { command: "agent".to_string(), args: Vec::new(), env: HashMap::new() },
    )
}

fn install_actor_handle(client: &AcpClient) -> mpsc::UnboundedReceiver<ActorCommand> {
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    client.actor_state.lock().handle = Some(AcpClientActorHandle { command_tx, thread: None });
    command_rx
}

// 下列行为用例重点检查复用、取消、通道关闭和失败后的状态一致性。
#[tokio::test]
async fn start_reuses_running_actor_handle() {
    let client = client();
    let _command_rx = install_actor_handle(&client);
    let first_tx = client.actor_command_tx().expect("actor command tx");

    client.start().await.expect("start");

    let second_tx = client.actor_command_tx().expect("actor command tx");
    assert!(first_tx.same_channel(&second_tx));
}

#[tokio::test]
async fn start_replaces_closed_handle_and_clears_reusable_session() {
    let client = client();
    let command_rx = install_actor_handle(&client);
    drop(command_rx);
    client.store_reusable_session(Some("stale-session".to_string()));

    client.start().await.expect("start");

    assert!(client.actor_state.lock().reusable_session_id.is_none());
    assert!(client.actor_command_tx().is_ok());
    client.close().await.expect("close");
}

#[tokio::test]
async fn close_without_actor_is_ok() {
    let client = client();

    client.close().await.expect("close");

    assert!(client.actor_state.lock().handle.is_none());
}

#[tokio::test]
async fn close_sends_close_command_and_clears_actor_state() {
    let client = client();
    let mut command_rx = install_actor_handle(&client);
    client.store_reusable_session(Some("stale-session".to_string()));
    let receiver = tokio::spawn(async move {
        match command_rx.recv().await.expect("close command") {
            ActorCommand::Close { response_tx } => {
                response_tx.send(()).expect("close response");
            }
            _ => panic!("expected close command"),
        }
    });

    client.close().await.expect("close");
    receiver.await.expect("receiver task");

    let state = client.actor_state.lock();
    assert!(state.handle.is_none());
    assert!(state.reusable_session_id.is_none());
}

#[tokio::test]
async fn close_with_closed_command_channel_clears_actor_state() {
    let client = client();
    let command_rx = install_actor_handle(&client);
    client.store_reusable_session(Some("stale-session".to_string()));
    drop(command_rx);

    client.close().await.expect("close");

    let state = client.actor_state.lock();
    assert!(state.handle.is_none());
    assert!(state.reusable_session_id.is_none());
}

#[tokio::test]
async fn actor_command_tx_errors_without_running_actor() {
    let client = client();

    let err = client.actor_command_tx().expect_err("actor should not be running");

    assert!(
        matches!(err, AcpError::Initialize(message) if message == "ACP client actor is not running")
    );
}

#[tokio::test]
async fn send_actor_request_returns_actor_response() {
    let client = client();
    let mut command_rx = install_actor_handle(&client);
    let request_client = client.clone();
    let request = tokio::spawn(async move {
        request_client
            .send_actor_request(|response_tx| ActorCommand::SetSessionMode {
                session_id: "session-1".to_string(),
                cwd: ".".into(),
                mode_id: "plan".to_string(),
                response_tx,
            })
            .await
    });

    match command_rx.recv().await.expect("actor command") {
        ActorCommand::SetSessionMode { session_id, mode_id, response_tx, .. } => {
            assert_eq!(session_id, "session-1");
            assert_eq!(mode_id, "plan");
            response_tx.send(Ok(())).expect("actor response");
        }
        _ => panic!("expected set session mode command"),
    }

    request.await.expect("request task").expect("request result");
}

#[tokio::test]
async fn send_actor_request_invalidates_actor_when_send_fails() {
    let client = client();
    let command_rx = install_actor_handle(&client);

    let err = client
        .send_actor_request(|response_tx| {
            drop(command_rx);
            ActorCommand::SetSessionMode {
                session_id: "session-1".to_string(),
                cwd: ".".into(),
                mode_id: "plan".to_string(),
                response_tx,
            }
        })
        .await
        .expect_err("send should fail");

    assert!(
        matches!(err, AcpError::Initialize(message) if message == "ACP client actor is unavailable")
    );
    assert!(client.actor_state.lock().handle.is_none());
}

#[tokio::test]
async fn send_actor_request_invalidates_actor_when_response_channel_closes() {
    let client = client();
    let mut command_rx = install_actor_handle(&client);
    let request_client = client.clone();
    let request = tokio::spawn(async move {
        request_client
            .send_actor_request(|response_tx: oneshot::Sender<Result<(), AcpError>>| {
                ActorCommand::SetSessionMode {
                    session_id: "session-1".to_string(),
                    cwd: ".".into(),
                    mode_id: "plan".to_string(),
                    response_tx,
                }
            })
            .await
    });

    match command_rx.recv().await.expect("actor command") {
        ActorCommand::SetSessionMode { response_tx, .. } => {
            drop(response_tx);
        }
        _ => panic!("expected set session mode command"),
    }

    let err = request.await.expect("request task").expect_err("response should fail");

    assert!(matches!(err, AcpError::PromptJoin(_)));
    assert!(client.actor_state.lock().handle.is_none());
}
