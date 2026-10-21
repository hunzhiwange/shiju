//! 会话相关 API。
//!
//! 本模块封装聊天会话生命周期管理与 UI 持久化相关接口，覆盖：
//! - 会话增删改查
//! - 消息列表与 diff 查询
//! - 会话 fork、reset、summarize
//! - todo、scope、archived 状态维护
//! - UI 会话快照与预览元数据读写

use std::collections::HashMap;

use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;

use super::GatewayClient;
use crate::http::directory_query;
use crate::session::{
    GatewaySessionCreateBody, GatewaySessionDiffQuery, GatewaySessionForkBody,
    GatewaySessionMessageDeltaResponse, GatewaySessionMessageListQuery, GatewaySessionMessagePage,
    GatewaySessionMessagePageQuery, GatewaySessionPatchBody, GatewaySessionResetBody,
    GatewaySessionScopeBody, GatewaySessionSummarizeBody, GatewaySessionTitleGenerateBody,
    GatewaySessionTitleGenerateResponse, GatewaySessionTodoPutBody, GatewaySessionUiSummary,
};
use vw_shared::session::ui_types::{
    ChatSession, ChatSessionMeta, ChatSessionStep, SessionMessageDelta, SessionMessageRecord,
};
use vw_shared::todo::Todo;

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
    pub title: String,
    #[serde(default)]
    pub messages: Vec<vw_shared::session::ui_types::ChatMessage>,
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
        Self { session, step_llm_raw, llm_raw_step_indices }
    }
}

impl GatewayClient {
    /// 按可恢复流请求标识幂等停止聊天运行。
    pub async fn chat_stream_stop(&self, request_id: &str) -> Result<bool, String> {
        let path = format!(
            "/v1/chat/stream/{}/stop",
            encode_session_path_segment(required_stream_request_id(request_id)?)
        );
        let response: StreamStopResponse =
            self.post_json(&path, &[], &serde_json::json!({})).await?;
        Ok(response.cancelled)
    }

    /// 读取会话 UI 数据，并保留响应中每步的原始 LLM 载荷。
    pub async fn session_ui_get_with_raw(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Option<SessionUiLoadResult>, String> {
        let loaded = self
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
        let loaded = self
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
        let loaded = self
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

fn encode_session_path_segment(value: &str) -> String {
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

impl GatewayClient {
    /// 列出目录上下文下的会话集合。
    ///
    /// 该方法保持返回值泛型化，以兼容不同调用方对列表结构的反序列化需求。
    pub async fn session_list<T: DeserializeOwned>(
        &self,
        directory: Option<&str>,
    ) -> Result<T, String> {
        self.get_json("/v1/session", &directory_query(directory)).await
    }

    /// 读取目录上下文下的会话运行状态表。
    pub async fn session_status<T: DeserializeOwned>(
        &self,
        directory: Option<&str>,
    ) -> Result<T, String> {
        self.get_json("/v1/session/status", &directory_query(directory)).await
    }

    /// 读取单个会话详情。
    pub async fn session_get<T: DeserializeOwned>(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<T, String> {
        self.get_json(&session_path(session_id)?, &directory_query(directory)).await
    }

    /// 读取指定会话的直接子会话。
    pub async fn session_children<T: DeserializeOwned>(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<T, String> {
        self.get_json(
            &format!("{}/children", session_path(session_id)?),
            &directory_query(directory),
        )
        .await
    }

    /// 读取指定会话的消息列表，使用默认分页参数。
    ///
    /// 当前默认只透传目录，不显式设置 `limit`，由后端决定默认页大小。
    pub async fn session_messages<T: DeserializeOwned>(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<T, String> {
        self.session_messages_query(
            session_id,
            &GatewaySessionMessageListQuery {
                directory: directory.map(ToOwned::to_owned),
                limit: None,
                cursor: None,
            },
        )
        .await
    }

    /// 读取不包含历史正文的会话摘要。
    pub async fn session_ui_summary(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<GatewaySessionUiSummary, String> {
        self.get_json(
            &format!("{}/ui/summary", session_path(session_id)?),
            &directory_query(directory),
        )
        .await
    }

    /// 读取固定大小的会话消息页。
    pub async fn session_message_page(
        &self,
        session_id: &str,
        query: &GatewaySessionMessagePageQuery,
    ) -> Result<GatewaySessionMessagePage<SessionMessageRecord>, String> {
        self.get_json(
            &format!("{}/ui/messages", session_path(session_id)?),
            &session_message_page_query(query),
        )
        .await
    }

    /// 原子提交会话消息增量。
    pub async fn session_message_delta(
        &self,
        session_id: &str,
        directory: Option<&str>,
        delta: &SessionMessageDelta,
    ) -> Result<GatewaySessionMessageDeltaResponse, String> {
        self.patch_json(
            &format!("{}/ui/messages", session_path(session_id)?),
            &directory_query(directory),
            delta,
        )
        .await
    }

    /// 按查询条件读取指定会话的消息列表。
    pub async fn session_messages_query<T: DeserializeOwned>(
        &self,
        session_id: &str,
        query: &GatewaySessionMessageListQuery,
    ) -> Result<T, String> {
        self.get_json(
            &format!("{}/message", session_path(session_id)?),
            &session_message_list_query(query),
        )
        .await
    }

    /// 读取单条会话消息及其片段。
    pub async fn session_message_get<T: DeserializeOwned>(
        &self,
        session_id: &str,
        message_id: &str,
        directory: Option<&str>,
    ) -> Result<T, String> {
        self.get_json(
            &format!(
                "{}/message/{}",
                session_path(session_id)?,
                required_encoded_id(message_id, "message")?
            ),
            &directory_query(directory),
        )
        .await
    }

    /// 更新单个消息片段。
    pub async fn session_message_part_update<B: serde::Serialize, T: DeserializeOwned>(
        &self,
        session_id: &str,
        message_id: &str,
        part_id: &str,
        directory: Option<&str>,
        body: &B,
    ) -> Result<T, String> {
        self.patch_json(
            &session_part_path(session_id, message_id, part_id)?,
            &directory_query(directory),
            body,
        )
        .await
    }

    /// 删除单个消息片段。
    pub async fn session_message_part_delete(
        &self,
        session_id: &str,
        message_id: &str,
        part_id: &str,
        directory: Option<&str>,
    ) -> Result<bool, String> {
        self.delete_json(
            &session_part_path(session_id, message_id, part_id)?,
            &directory_query(directory),
            &serde_json::json!({}),
        )
        .await
    }

    /// 创建新会话。
    ///
    /// `directory` 会作为查询参数传入，用于确定会话所属工作区上下文。
    pub async fn session_create<T: DeserializeOwned>(
        &self,
        directory: &str,
        body: &Option<GatewaySessionCreateBody>,
    ) -> Result<T, String> {
        self.post_json("/v1/session", &directory_query(Some(directory)), body).await
    }

    /// 更新会话元数据或状态。
    pub async fn session_update<T: DeserializeOwned>(
        &self,
        session_id: &str,
        directory: Option<&str>,
        body: &GatewaySessionPatchBody,
    ) -> Result<T, String> {
        self.patch_json(&session_path(session_id)?, &directory_query(directory), body).await
    }

    /// 删除会话。
    pub async fn session_delete(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<(), String> {
        self.delete_empty(&session_path(session_id)?, &directory_query(directory)).await
    }

    /// 从现有会话派生一个新会话。
    pub async fn session_fork<T: DeserializeOwned>(
        &self,
        session_id: &str,
        directory: Option<&str>,
        body: &Option<GatewaySessionForkBody>,
    ) -> Result<T, String> {
        self.post_json(
            &format!("{}/fork", session_path(session_id)?),
            &directory_query(directory),
            body,
        )
        .await
    }

    /// 将会话重置到指定消息节点或初始状态。
    pub async fn session_reset<T: DeserializeOwned>(
        &self,
        session_id: &str,
        directory: Option<&str>,
        body: &GatewaySessionResetBody,
    ) -> Result<T, String> {
        self.post_json(
            &format!("{}/reset", session_path(session_id)?),
            &directory_query(directory),
            body,
        )
        .await
    }

    /// 为会话触发摘要生成。
    pub async fn session_summarize(
        &self,
        session_id: &str,
        directory: Option<&str>,
        body: &GatewaySessionSummarizeBody,
    ) -> Result<bool, String> {
        self.post_json(
            &format!("{}/summarize", session_path(session_id)?),
            &directory_query(directory),
            body,
        )
        .await
    }

    /// 覆盖写入会话待办列表。
    pub async fn session_todo_update(
        &self,
        session_id: &str,
        directory: Option<&str>,
        body: &GatewaySessionTodoPutBody,
    ) -> Result<bool, String> {
        self.put_json(
            &format!("{}/todo", session_path(session_id)?),
            &directory_query(directory),
            body,
        )
        .await
    }

    /// 获取会话待办列表。
    pub async fn session_todo_get(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Vec<Todo>, String> {
        self.get_json(&format!("{}/todo", session_path(session_id)?), &directory_query(directory))
            .await
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// 以阻塞方式获取会话待办列表。
    pub fn session_todo_get_blocking(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Vec<Todo>, String> {
        use crate::client::request_helpers::get_json_blocking;

        get_json_blocking(
            &self.endpoint,
            &format!("/v1/session/{session_id}/todo"),
            &directory_query(directory),
        )
    }

    /// 获取会话在指定消息节点上的差异数据。
    pub async fn session_diff<T: DeserializeOwned>(
        &self,
        session_id: &str,
        query: &GatewaySessionDiffQuery,
    ) -> Result<T, String> {
        self.get_json(&format!("{}/diff", session_path(session_id)?), &session_diff_query(query))
            .await
    }

    /// 读取会话 UI 持久化数据，不存在时返回 None。
    ///
    /// 对 404 响应做了特殊处理，因此调用方无需把“未保存过 UI”视为错误。
    pub async fn session_ui_get(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Option<ChatSession>, String> {
        let with_raw = self.session_ui_get_with_raw(session_id, directory).await?;
        Ok(with_raw.map(|loaded| loaded.session))
    }

    /// 无目录限制地读取会话 UI 持久化数据。
    pub async fn session_ui_get_any(
        &self,
        session_id: &str,
    ) -> Result<Option<ChatSession>, String> {
        let with_raw = self.session_ui_get_any_with_raw(session_id).await?;
        Ok(with_raw.map(|loaded| loaded.session))
    }

    /// 保存会话 UI 持久化数据。
    pub async fn session_ui_save(
        &self,
        session_id: &str,
        directory: Option<&str>,
        session: &ChatSession,
    ) -> Result<bool, String> {
        self.put_json(
            &format!("{}/ui", session_path(session_id)?),
            &directory_query(directory),
            session,
        )
        .await
    }

    /// 列出目录下会话 UI 预览元数据。
    pub async fn session_ui_previews(
        &self,
        directory: Option<&str>,
    ) -> Result<Vec<ChatSessionMeta>, String> {
        self.get_json("/v1/session/ui-previews", &directory_query(directory)).await
    }

    /// 获取单个会话的预览元数据。
    pub async fn session_preview_meta_get(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Option<ChatSessionMeta>, String> {
        self.get_json_with_404(
            &format!("{}/preview", session_path(session_id)?),
            &directory_query(directory),
        )
        .await
    }

    /// 获取会话对应的持久化路径。
    pub async fn session_path_get(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Option<String>, String> {
        self.get_json_with_404(
            &format!("{}/path", session_path(session_id)?),
            &directory_query(directory),
        )
        .await
    }

    /// 读取指定会话步骤的原始 LLM 请求/响应载荷。
    pub async fn session_step_llm_raw_get(
        &self,
        session_id: &str,
        turn_id: &str,
        step_index: u32,
        directory: Option<&str>,
    ) -> Result<Option<serde_json::Value>, String> {
        let legacy_step_turn_id = format!("legacy-step-{step_index}");
        let request_turn_id = if turn_id.trim().is_empty() || turn_id == legacy_step_turn_id {
            "legacy"
        } else {
            turn_id
        };
        self.get_json_with_404(
            &format!(
                "{}/turn/{}/step/{step_index}/llm-raw",
                session_path(session_id)?,
                required_encoded_id(request_turn_id, "turn")?
            ),
            &directory_query(directory),
        )
        .await
    }

    /// 获取当前目录下被归档的会话 ID 列表。
    pub async fn session_archived_get(
        &self,
        directory: Option<&str>,
    ) -> Result<Vec<String>, String> {
        self.get_json("/v1/session/archived", &directory_query(directory)).await
    }

    /// 覆盖写入归档会话 ID 列表。
    pub async fn session_archived_put(
        &self,
        directory: Option<&str>,
        ids: &[String],
    ) -> Result<bool, String> {
        let body: Vec<String> = ids.to_vec();
        self.put_json("/v1/session/archived", &directory_query(directory), &body).await
    }

    /// 读取当前全局会话作用域。
    ///
    /// 当前实现忽略目录与项目参数，始终读取网关维护的全局 scope 状态。
    pub async fn session_scope_get(
        &self,
        _directory: Option<&str>,
        _project_id: Option<&str>,
    ) -> Result<Option<String>, String> {
        self.get_json_with_404("/v1/session/scope", &[]).await
    }

    /// 更新当前全局会话作用域。
    pub async fn session_scope_put(&self, scope: Option<&str>) -> Result<bool, String> {
        let body = GatewaySessionScopeBody { scope: scope.map(|s| s.to_string()) };
        self.put_json("/v1/session/scope", &[], &body).await
    }

    /// 基于会话内容生成候选标题。
    pub async fn session_title_generate(
        &self,
        session_id: &str,
        body: &GatewaySessionTitleGenerateBody,
    ) -> Result<GatewaySessionTitleGenerateResponse, String> {
        self.post_json(&format!("{}/title", session_path(session_id)?), &[], body).await
    }
}

fn session_path(session_id: &str) -> Result<String, String> {
    if session_id.trim().is_empty() {
        return Err("session id must not be empty".to_string());
    }
    Ok(format!("/v1/session/{}", encode_session_path_segment(session_id)))
}

fn required_encoded_id(value: &str, kind: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        return Err(format!("{kind} id must not be empty"));
    }
    Ok(encode_session_path_segment(value))
}

fn session_part_path(session_id: &str, message_id: &str, part_id: &str) -> Result<String, String> {
    Ok(format!(
        "{}/message/{}/part/{}",
        session_path(session_id)?,
        required_encoded_id(message_id, "message")?,
        required_encoded_id(part_id, "part")?
    ))
}

/// 将消息列表查询结构转换为网关接口使用的查询参数。
fn session_message_list_query(query: &GatewaySessionMessageListQuery) -> Vec<(String, String)> {
    let mut pairs = directory_query(query.directory.as_deref());
    if let Some(limit) = query.limit {
        pairs.push(("limit".to_string(), limit.to_string()));
    }
    if let Some(cursor) = query.cursor.as_deref().filter(|value| !value.is_empty()) {
        pairs.push(("cursor".to_string(), cursor.to_string()));
    }
    pairs
}

fn session_message_page_query(query: &GatewaySessionMessagePageQuery) -> Vec<(String, String)> {
    let mut pairs = directory_query(query.directory.as_deref());
    if let Some(cursor) = query.cursor.as_deref().filter(|value| !value.is_empty()) {
        pairs.push(("cursor".to_string(), cursor.to_string()));
    }
    if let Some(limit) = query.limit {
        pairs.push(("limit".to_string(), limit.to_string()));
    }
    pairs.push((
        "direction".to_string(),
        match query.direction {
            vw_api_types::session::SessionMessagePageDirection::Older => "older".to_string(),
        },
    ));
    pairs
}

/// 将 diff 查询结构转换为网关接口使用的查询参数。
fn session_diff_query(query: &GatewaySessionDiffQuery) -> Vec<(String, String)> {
    let mut pairs = directory_query(query.directory.as_deref());
    if let Some(message_id) = query.message_id.as_deref().filter(|value| !value.trim().is_empty()) {
        pairs.push(("messageID".to_string(), message_id.to_string()));
    }
    pairs
}

#[cfg(test)]
#[path = "session_api_tests.rs"]
mod session_api_tests;
