//! 从 JSON-RPC 边界验证两种模型选择接口和实际提示使用的模型。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use agent_client_protocol::{
    Agent, InitializeRequest, ModelInfo, NewSessionRequest, ProtocolVersion, SessionModelState,
    SetSessionModelRequest,
};
use serde_json::{Value, json};
use tokio::io::{AsyncWriteExt, BufReader};

use super::acceptance_tests::{read_frame, write_frame};
use super::{
    BackendCancelHandle, BackendError, BackendEvent, BackendPrompt, BackendSession,
    BackendTermination, PromptContent, SessionBackend, VibeWindowAgent,
};

#[derive(Default)]
struct ModelBackend(Mutex<HashMap<String, String>>);

struct NoopCancel;
impl BackendCancelHandle for NoopCancel {
    fn cancel(&self) -> bool {
        true
    }
}

fn state(current: String) -> SessionModelState {
    SessionModelState::new(
        current,
        vec![ModelInfo::new("auto", "Auto"), ModelInfo::new("fixture/fast", "Fast")],
    )
}

#[async_trait::async_trait]
impl SessionBackend for ModelBackend {
    async fn create_session(&self, cwd: PathBuf) -> Result<BackendSession, BackendError> {
        self.0.lock().unwrap().insert("session-1".into(), "auto".into());
        Ok(BackendSession { id: "session-1".into(), cwd, history: Vec::new() })
    }

    async fn load_session(
        &self,
        session_id: &str,
        cwd: PathBuf,
    ) -> Result<BackendSession, BackendError> {
        self.session_models(session_id).await?;
        Ok(BackendSession { id: session_id.into(), cwd, history: Vec::new() })
    }

    async fn session_models(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionModelState>, BackendError> {
        self.0
            .lock()
            .unwrap()
            .get(session_id)
            .cloned()
            .map(|model| Some(state(model)))
            .ok_or_else(|| BackendError::NotFound(session_id.into()))
    }

    async fn set_session_model(
        &self,
        session_id: &str,
        model_id: &str,
    ) -> Result<SessionModelState, BackendError> {
        if !["auto", "fixture/fast"].contains(&model_id) {
            return Err(BackendError::InvalidRequest("unknown model".into()));
        }
        let mut sessions = self.0.lock().unwrap();
        let selected = sessions
            .get_mut(session_id)
            .ok_or_else(|| BackendError::NotFound(session_id.into()))?;
        *selected = model_id.into();
        Ok(state(model_id.into()))
    }

    async fn start_prompt(
        &self,
        session_id: &str,
        _prompt: Vec<PromptContent>,
    ) -> Result<BackendPrompt, BackendError> {
        let model = self.0.lock().unwrap().get(session_id).cloned().unwrap();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tx.send(BackendEvent::Text(model)).unwrap();
        tx.send(BackendEvent::Terminal(BackendTermination::EndTurn)).unwrap();
        Ok(BackendPrompt::new(rx, Arc::new(NoopCancel)))
    }
}

async fn response(
    reader: &mut BufReader<&mut tokio::io::DuplexStream>,
    id: u64,
) -> (Value, Vec<Value>) {
    let mut updates = Vec::new();
    loop {
        let frame = read_frame(reader).await;
        if frame["id"] == id {
            return (frame, updates);
        }
        assert_eq!(frame["method"], "session/update");
        updates.push(frame["params"]["update"].clone());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn model_selection_protocol_updates_prompt_and_load_state() {
    let root = tempfile::tempdir().unwrap();
    let (mut client, server) = tokio::io::duplex(32 * 1024);
    let (reader, writer) = tokio::io::split(server);
    let server = super::serve_with_io(ModelBackend::default(), reader, writer);
    let client = async {
        let mut reader = BufReader::new(&mut client);
        for (id, method, params) in [
            (1, "initialize", json!({"protocolVersion": 1, "clientCapabilities": {}})),
            (2, "session/new", json!({"cwd": root.path(), "mcpServers": []})),
        ] {
            write_frame(
                reader.get_mut(),
                json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}),
            )
            .await;
            let (frame, _) = response(&mut reader, id).await;
            assert!(frame.get("error").is_none(), "{frame}");
            if id == 2 {
                assert_eq!(frame["result"]["models"]["currentModelId"], "auto");
                assert_eq!(frame["result"]["configOptions"][0]["category"], "model");
                assert_eq!(
                    frame["result"]["configOptions"][0]["options"][1]["value"],
                    "fixture/fast"
                );
            }
        }
        for (id, method, params, expected) in [
            (
                3,
                "session/set_config_option",
                json!({"sessionId":"session-1", "configId":"model", "value":"fixture/fast"}),
                "fixture/fast",
            ),
            (4, "session/set_model", json!({"sessionId":"session-1", "modelId":"auto"}), "auto"),
            (
                5,
                "session/set_model",
                json!({"sessionId":"session-1", "modelId":"fixture/fast"}),
                "fixture/fast",
            ),
        ] {
            write_frame(
                reader.get_mut(),
                json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}),
            )
            .await;
            let (frame, updates) = response(&mut reader, id).await;
            assert!(frame.get("error").is_none(), "{frame}");
            assert_eq!(updates.len(), 1);
            assert_eq!(updates[0]["sessionUpdate"], "config_option_update");
            assert_eq!(updates[0]["configOptions"][0]["currentValue"], expected);
            if id == 3 {
                assert_eq!(frame["result"]["configOptions"][0]["currentValue"], expected);
            }
        }
        for (id, method, params) in [
            (6, "session/set_model", json!({"sessionId":"session-1", "modelId":"missing"})),
            (
                7,
                "session/set_config_option",
                json!({"sessionId":"session-1", "configId":"unknown", "value":"auto"}),
            ),
            (8, "session/set_model", json!({"sessionId":"unknown", "modelId":"auto"})),
        ] {
            write_frame(
                reader.get_mut(),
                json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}),
            )
            .await;
            let (frame, updates) = response(&mut reader, id).await;
            assert!(frame.get("error").is_some(), "{frame}");
            assert!(updates.is_empty());
        }
        write_frame(reader.get_mut(), json!({"jsonrpc":"2.0", "id":9, "method":"session/prompt", "params":{"sessionId":"session-1", "prompt":[{"type":"text", "text":"hello"}]}})).await;
        let (frame, updates) = response(&mut reader, 9).await;
        assert_eq!(frame["result"]["stopReason"], "end_turn");
        assert_eq!(updates[0]["content"]["text"], "fixture/fast");
        write_frame(reader.get_mut(), json!({"jsonrpc":"2.0", "id":10, "method":"session/load", "params":{"sessionId":"session-1", "cwd":root.path(), "mcpServers":[]}})).await;
        let (frame, _) = response(&mut reader, 10).await;
        assert_eq!(frame["result"]["models"]["currentModelId"], "fixture/fast");
        assert_eq!(frame["result"]["configOptions"][0]["currentValue"], "fixture/fast");
        reader.get_mut().shutdown().await.unwrap();
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let (server, ()) = tokio::join!(server, client);
        server.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn model_selection_requires_initialized_and_bound_session() {
    let root = tempfile::tempdir().unwrap();
    let agent = VibeWindowAgent::new(ModelBackend::default());
    assert!(
        Agent::set_session_model(&agent, SetSessionModelRequest::new("session-1", "auto"))
            .await
            .is_err()
    );
    Agent::initialize(&agent, InitializeRequest::new(ProtocolVersion::V1)).await.unwrap();
    assert!(
        Agent::set_session_model(&agent, SetSessionModelRequest::new("session-1", "auto"))
            .await
            .is_err()
    );
    Agent::new_session(&agent, NewSessionRequest::new(root.path())).await.unwrap();
    Agent::set_session_model(&agent, SetSessionModelRequest::new("session-1", "fixture/fast"))
        .await
        .unwrap();
}
