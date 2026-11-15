//! 会话相关 API。
//!
//! 本模块封装聊天会话生命周期管理与 UI 持久化相关接口，覆盖：
//! - 会话增删改查
//! - 消息列表与 diff 查询
//! - 会话 fork、reset、summarize
//! - todo、scope、archived 状态维护
//! - UI 会话快照与预览元数据读写

use std::collections::HashMap;

use serde::{Deserialize};
use serde_json::Value;

use super::GatewaySessionClient;
use gateway_http::directory_query;
use session_ui_types::{
    ChatSession, ChatSessionStep,
};

#[derive(Debug, Clone, Deserialize)]
struct SessionStepWithRaw {
    #[serde(flatten)]
    step: ChatSessionStep,
    #[serde(default)]
    llm_raw: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct ChatSessionWithRaw {
    pub id: String,
    #[serde(default)]
    pub agent_binding: Option<vw_api_chat_identity::SessionAgentIdentity>,
    pub title: String,
    #[serde(default)]
    pub messages: Vec<session_ui_types::ChatMessage>,
    #[serde(default)]
    pub message_ids: Vec<Option<String>>,
    #[serde(default)]
    pub calls: Vec<Value>,
    #[serde(default)]
    pub steps: Vec<SessionStepWithRaw>,
    #[serde(default)]
    pub llm_raw_step_indices: Vec<u32>,
    #[serde(default)]
    pub created_ms: u64,
    #[serde(default)]
    pub updated_ms: u64,
}

#[derive(Debug, Deserialize)]
struct StreamStopResponse {
    cancelled: bool,
}

/// 会话 UI 批量加载结果。
///
/// 普通会话数据保持可直接保存的 `ChatSession` 形状；每步原始 LLM 载荷单独按稳定的
/// `(session_id, turn_id, step_index)` 保存，避免把只读加载数据混入持久化模型。
#[derive(Debug, Clone)]
pub struct SessionUiLoadResult {
    pub session: ChatSession,
    pub agent_binding: Option<vw_api_chat_identity::SessionAgentIdentity>,
    pub step_llm_raw: HashMap<(String, String, u32), Value>,
    pub llm_raw_step_indices: Vec<(String, String, u32)>,
}

impl From<ChatSessionWithRaw> for SessionUiLoadResult {
    fn from(value: ChatSessionWithRaw) -> Self {
        let mut step_llm_raw = HashMap::new();
        let session_id = value.id.clone();
        let steps = value
            .steps
            .into_iter()
            .map(|mut step| {
                step.step.turn_id =
                    step.step.turn_id.take().filter(|turn_id| !turn_id.trim().is_empty());
                if let Some(raw) = step.llm_raw {
                    let turn_id = step
                        .step
                        .turn_id
                        .clone()
                        .unwrap_or_else(|| format!("legacy-step-{}", step.step.index));
                    step_llm_raw.insert((session_id.clone(), turn_id, step.step.index), raw);
                }
                step.step
            })
            .collect::<Vec<_>>();
        let llm_raw_step_indices = steps
            .iter()
            .filter(|step| value.llm_raw_step_indices.contains(&step.index))
            .map(|step| {
                let turn_id =
                    step.turn_id.clone().unwrap_or_else(|| format!("legacy-step-{}", step.index));
                (session_id.clone(), turn_id, step.index)
            })
            .collect();
        let session = ChatSession {
            id: value.id,
            title: value.title,
            messages: value.messages,
            message_ids: value.message_ids,
            calls: value.calls,
            steps,
            created_ms: value.created_ms,
            updated_ms: value.updated_ms,
        };
        Self { session, agent_binding: value.agent_binding, step_llm_raw, llm_raw_step_indices }
    }
}

impl GatewaySessionClient {
    pub async fn acp_model_source_capabilities(
        &self,
        agent: &str,
    ) -> Result<vw_api_acp_model::acp_session::AcpSessionOptions, String> {
        let path = format!("/v1/acp-model-source/{}", encode_session_path_segment(agent));
        self.transport.get_json(&path, &[]).await
    }

    pub async fn acp_session_model_source(
        &self,
        session_id: &str,
        directory: Option<&str>,
        update: &vw_api_acp_model::acp_model_source::AcpModelSourceUpdate,
    ) -> Result<vw_api_acp_model::acp_session::AcpSessionOptions, String> {
        self.transport.put_json(
            &format!("{}/acp-model-source", session_path(session_id)?),
            &directory_query(directory),
            update,
        )
        .await
    }

    pub async fn acp_session_options(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<vw_api_acp_model::acp_session::AcpSessionOptions, String> {
        self.transport.get_json(
            &format!("{}/acp-options", session_path(session_id)?),
            &directory_query(directory),
        )
        .await
    }

    pub async fn session_set_full_access(
        &self,
        session_id: &str,
        directory: Option<&str>,
        full_access: bool,
    ) -> Result<bool, String> {
        self.transport.put_json(
            &format!("{}/full-access", session_path(session_id)?),
            &directory_query(directory),
            &full_access,
        )
        .await
    }
    pub async fn acp_session_select(
        &self,
        session_id: &str,
        directory: Option<&str>,
        selection: &vw_api_acp_model::acp_session::AcpSessionSelection,
    ) -> Result<vw_api_acp_model::acp_session::AcpSessionOptions, String> {
        self.transport.put_json(
            &format!("{}/acp-options", session_path(session_id)?),
            &directory_query(directory),
            selection,
        )
        .await
    }

    /// 按可恢复流请求标识幂等停止聊天运行。
    pub async fn chat_stream_stop(&self, request_id: &str) -> Result<bool, String> {
        let path = format!(
            "/v1/chat/stream/{}/stop",
            encode_session_path_segment(required_stream_request_id(request_id)?)
        );
        let response: StreamStopResponse =
            self.transport.post_json(&path, &[], &serde_json::json!({})).await?;
        Ok(response.cancelled)
    }

    /// 读取会话 UI 数据，并保留响应中每步的原始 LLM 载荷。
    pub async fn session_ui_get_with_raw(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Option<SessionUiLoadResult>, String> {
        let loaded = self.transport
            .get_json_with_404::<ChatSessionWithRaw>(
                &format!("{}/ui", session_path(session_id)?),
                &directory_query(directory),
            )
            .await?;
        Ok(loaded.map(SessionUiLoadResult::from))
    }

    /// 读取会话 UI 数据和 Raw 索引，但不加载每步原始 LLM 载荷。
    pub async fn session_ui_get_without_raw(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Option<SessionUiLoadResult>, String> {
        let loaded = self.transport
            .get_json_with_404::<ChatSessionWithRaw>(
                &format!("{}/ui/without-raw", session_path(session_id)?),
                &directory_query(directory),
            )
            .await?;
        Ok(loaded.map(SessionUiLoadResult::from))
    }

    /// 无目录限制地读取会话 UI 数据及每步原始 LLM 载荷。
    pub async fn session_ui_get_any_with_raw(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionUiLoadResult>, String> {
        let loaded = self.transport
            .get_json_with_404::<ChatSessionWithRaw>(
                &format!("{}/any", session_path(session_id)?),
                &[],
            )
            .await?;
        Ok(loaded.map(SessionUiLoadResult::from))
    }
}

fn required_stream_request_id(request_id: &str) -> Result<&str, String> {
    if request_id.trim().is_empty() {
        return Err("stream request id must not be empty".to_string());
    }
    Ok(request_id)
}

pub(super) fn encode_session_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}


use super::session_api::session_path;
