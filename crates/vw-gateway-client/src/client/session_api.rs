//! 会话相关 API。
//!
//! 本模块封装聊天会话生命周期管理与 UI 持久化相关接口，覆盖：
//! - 会话增删改查
//! - 消息列表与 diff 查询
//! - 会话 fork、reset、summarize
//! - todo、scope、archived 状态维护
//! - UI 会话快照与预览元数据读写

use serde::de::DeserializeOwned;

use super::GatewayClient;
use crate::http::directory_query;
use crate::session::{
    GatewaySessionCreateBody, GatewaySessionDiffQuery, GatewaySessionForkBody,
    GatewaySessionMessageListQuery, GatewaySessionPatchBody, GatewaySessionResetBody,
    GatewaySessionScopeBody, GatewaySessionSummarizeBody, GatewaySessionTitleGenerateBody,
    GatewaySessionTitleGenerateResponse, GatewaySessionTodoPutBody,
};
use vw_shared::session::ui_types::{ChatSession, ChatSessionMeta};
use vw_shared::todo::Todo;

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

    /// 读取单个会话详情。
    pub async fn session_get<T: DeserializeOwned>(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<T, String> {
        self.get_json(&format!("/v1/session/{session_id}"), &directory_query(directory)).await
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
            },
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
            &format!("/v1/session/{session_id}/message"),
            &session_message_list_query(query),
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
        self.patch_json(&format!("/v1/session/{session_id}"), &directory_query(directory), body)
            .await
    }

    /// 删除会话。
    pub async fn session_delete(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<(), String> {
        self.delete_empty(&format!("/v1/session/{session_id}"), &directory_query(directory)).await
    }

    /// 从现有会话派生一个新会话。
    pub async fn session_fork<T: DeserializeOwned>(
        &self,
        session_id: &str,
        directory: Option<&str>,
        body: &Option<GatewaySessionForkBody>,
    ) -> Result<T, String> {
        self.post_json(&format!("/v1/session/{session_id}/fork"), &directory_query(directory), body)
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
            &format!("/v1/session/{session_id}/reset"),
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
            &format!("/v1/session/{session_id}/summarize"),
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
        self.put_json(&format!("/v1/session/{session_id}/todo"), &directory_query(directory), body)
            .await
    }

    /// 获取会话待办列表。
    pub async fn session_todo_get(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Vec<Todo>, String> {
        self.get_json(&format!("/v1/session/{session_id}/todo"), &directory_query(directory)).await
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
        self.get_json(&format!("/v1/session/{session_id}/diff"), &session_diff_query(query)).await
    }

    /// 读取会话 UI 持久化数据，不存在时返回 None。
    ///
    /// 对 404 响应做了特殊处理，因此调用方无需把“未保存过 UI”视为错误。
    pub async fn session_ui_get(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<Option<ChatSession>, String> {
        self.get_json_with_404(&format!("/v1/session/{session_id}/ui"), &directory_query(directory))
            .await
    }

    /// 无目录限制地读取会话 UI 持久化数据。
    pub async fn session_ui_get_any(
        &self,
        session_id: &str,
    ) -> Result<Option<ChatSession>, String> {
        self.get_json_with_404(&format!("/v1/session/{session_id}/any"), &[]).await
    }

    /// 保存会话 UI 持久化数据。
    pub async fn session_ui_save(
        &self,
        session_id: &str,
        directory: Option<&str>,
        session: &ChatSession,
    ) -> Result<bool, String> {
        self.put_json(&format!("/v1/session/{session_id}/ui"), &directory_query(directory), session)
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
            &format!("/v1/session/{session_id}/preview"),
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
            &format!("/v1/session/{session_id}/path"),
            &directory_query(directory),
        )
        .await
    }

    /// 读取指定会话步骤的原始 LLM 请求/响应载荷。
    pub async fn session_step_llm_raw_get(
        &self,
        session_id: &str,
        step_index: u32,
        directory: Option<&str>,
    ) -> Result<Option<serde_json::Value>, String> {
        self.get_json_with_404(
            &format!("/v1/session/{session_id}/step/{step_index}/llm-raw"),
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
        self.post_json(&format!("/v1/session/{session_id}/title"), &[], body).await
    }
}

/// 将消息列表查询结构转换为网关接口使用的查询参数。
fn session_message_list_query(query: &GatewaySessionMessageListQuery) -> Vec<(String, String)> {
    let mut pairs = directory_query(query.directory.as_deref());
    if let Some(limit) = query.limit {
        pairs.push(("limit".to_string(), limit.to_string()));
    }
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
