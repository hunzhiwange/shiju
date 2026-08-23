//! 活动 TUI 宿主需要的 Gateway 会话创建、恢复与快照保存操作。
//!
//! 异步方法集中处理传输错误和会话 ID 校验；同步包装仅供当前终端宿主
//! 在非异步事件循环边界调用，不向状态模型泄漏 Gateway 客户端。

use std::collections::HashSet;

use serde::Deserialize;
use vw_gateway_client::{
    GatewaySessionCreateBody, GatewaySessionForkBody, GatewaySessionPatchBody,
    GatewaySessionResetBody, GatewaySessionSummarizeBody,
};
use vw_shared::session::ui_types::{ChatSession, ChatSessionMeta};

use super::gateway::{
    GatewayUiRuntime, annotate_gateway_transport_error, block_on_gateway,
    normalize_optional_str_ref,
};

/// Gateway 创建会话后，TUI 初始化所需的最小返回值。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct UiSessionCreateInfo {
    pub(crate) id: String,
    pub(crate) title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct UiSessionMutationInfo {
    id: String,
}

impl GatewayUiRuntime {
    async fn session_create(&self, title: Option<&str>) -> Result<UiSessionCreateInfo, String> {
        let directory = self
            .directory_value()
            .unwrap_or_else(|| self.directory().to_string_lossy().to_string());
        let body = Some(GatewaySessionCreateBody {
            parent_id: None,
            title: normalize_optional_str_ref(title).map(ToOwned::to_owned),
        });
        self.client()
            .session_create(directory.as_str(), &body)
            .await
            .map_err(|error| annotate_gateway_transport_error(error, self.endpoint()))
    }

    async fn session_ui_load_any(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<ChatSession>, String> {
        let session_id = self.resolve_session_id(session_id)?;
        self.client().session_ui_get_any(session_id).await
    }

    async fn session_ui_load(&self, session_id: &str) -> Result<ChatSession, String> {
        let session_id = normalize_optional_str_ref(Some(session_id))
            .ok_or_else(|| "session restore requires a non-empty session id".to_string())?;
        let directory = self.directory_value();
        let snapshot = self
            .client()
            .session_ui_get(session_id, directory.as_deref())
            .await?
            .ok_or_else(|| format!("session {session_id} has no UI snapshot"))?;
        if snapshot.id != session_id {
            return Err(format!(
                "session restore returned mismatched id: expected {session_id}, got {}",
                snapshot.id
            ));
        }
        Ok(snapshot)
    }

    async fn session_ui_save(&self, session: &ChatSession) -> Result<(), String> {
        let session_id = normalize_optional_str_ref(Some(session.id.as_str()))
            .ok_or_else(|| "session_ui save requires a non-empty session id".to_string())?;
        let directory = self.directory_value();
        self.client().session_ui_save(session_id, directory.as_deref(), session).await.map(|_| ())
    }

    async fn session_ui_previews(&self) -> Result<Vec<ChatSessionMeta>, String> {
        let directory = self.directory_value();
        self.client().session_ui_previews(directory.as_deref()).await
    }

    async fn session_resume_candidates(&self) -> Result<Vec<ChatSessionMeta>, String> {
        let directory = self.directory_value();
        let mut previews = self.client().session_ui_previews(directory.as_deref()).await?;
        let archived = self.client().session_archived_get(directory.as_deref()).await?;
        let archived = archived.into_iter().collect::<HashSet<_>>();
        previews.retain(|preview| !archived.contains(preview.id.as_str()));
        previews.sort_by(|left, right| {
            right
                .updated_ms
                .cmp(&left.updated_ms)
                .then(right.message_count.cmp(&left.message_count))
                .then(right.call_count.cmp(&left.call_count))
                .then(left.id.cmp(&right.id))
        });
        Ok(previews)
    }

    async fn session_scope_get(&self) -> Result<Option<String>, String> {
        self.client().session_scope_get(None, None).await
    }

    async fn session_rename(&self, title: &str) -> Result<(), String> {
        let session_id = self.resolve_session_id(None)?;
        let title = normalize_optional_str_ref(Some(title))
            .ok_or_else(|| "session rename requires a non-empty title".to_string())?;
        let body = GatewaySessionPatchBody { title: Some(title.to_string()), time: None };
        let directory = self.directory_value();
        let info: UiSessionMutationInfo = self
            .client()
            .session_update(session_id, directory.as_deref(), &body)
            .await?;
        ensure_mutation_session_id("rename", session_id, info.id.as_str())
    }

    async fn session_fork_from(&self, message_id: Option<&str>) -> Result<String, String> {
        let session_id = self.resolve_session_id(None)?;
        let body = Some(GatewaySessionForkBody {
            message_id: normalize_optional_str_ref(message_id).map(ToOwned::to_owned),
        });
        let directory = self.directory_value();
        let info: UiSessionMutationInfo = self
            .client()
            .session_fork(session_id, directory.as_deref(), &body)
            .await?;
        let forked_id = normalize_optional_str_ref(Some(info.id.as_str()))
            .ok_or_else(|| "session fork returned an empty session id".to_string())?;
        if forked_id == session_id {
            return Err("session fork returned the source session id".to_string());
        }
        Ok(forked_id.to_string())
    }

    async fn session_reset_to(&self, message_id: &str) -> Result<(), String> {
        let session_id = self.resolve_session_id(None)?;
        let message_id = required_message_id("reset", message_id)?;
        let body = GatewaySessionResetBody { message_id: message_id.to_string(), revert_code: true };
        let directory = self.directory_value();
        let info: UiSessionMutationInfo = self
            .client()
            .session_reset(session_id, directory.as_deref(), &body)
            .await?;
        ensure_mutation_session_id("reset", session_id, info.id.as_str())
    }

    async fn session_compact_from(&self, message_id: &str) -> Result<bool, String> {
        let session_id = self.resolve_session_id(None)?;
        let message_id = required_message_id("compact", message_id)?;
        let body = GatewaySessionSummarizeBody { message_id: message_id.to_string() };
        let directory = self.directory_value();
        self.client()
            .session_summarize(session_id, directory.as_deref(), &body)
            .await
    }

    async fn session_archive_current(&self) -> Result<(), String> {
        let session_id = self.resolve_session_id(None)?;
        let directory = self.directory_value();
        let mut archived = self.client().session_archived_get(directory.as_deref()).await?;
        if !archived.iter().any(|candidate| candidate == session_id) {
            archived.push(session_id.to_string());
        }
        if !self.client().session_archived_put(directory.as_deref(), &archived).await? {
            return Err(format!("gateway did not archive session {session_id}"));
        }
        Ok(())
    }

    /// 同步创建会话，传入空白标题时交由 Gateway 选择默认值。
    pub(crate) fn session_create_blocking(
        &self,
        title: Option<&str>,
    ) -> Result<UiSessionCreateInfo, String> {
        block_on_gateway(self.session_create(title))
    }

    /// 按显式 ID 或运行时绑定 ID 加载任意格式的 UI 会话快照。
    pub(crate) fn session_ui_load_any_blocking(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<ChatSession>, String> {
        block_on_gateway(self.session_ui_load_any(session_id))
    }

    /// 在当前目录约束下加载完整会话快照，并拒绝空 ID、缺失快照或响应 ID 失配。
    pub(crate) fn session_ui_load_blocking(&self, session_id: &str) -> Result<ChatSession, String> {
        block_on_gateway(self.session_ui_load(session_id))
    }

    /// 将完整 UI 会话快照同步保存到 Gateway。
    pub(crate) fn session_ui_save_blocking(&self, session: &ChatSession) -> Result<(), String> {
        block_on_gateway(self.session_ui_save(session))
    }

    /// 加载当前工作目录可见的会话摘要列表。
    pub(crate) fn session_ui_previews_blocking(&self) -> Result<Vec<ChatSessionMeta>, String> {
        block_on_gateway(self.session_ui_previews())
    }

    /// 返回当前目录下按最近更新时间倒序排列的非归档会话。
    pub(crate) fn session_resume_candidates_blocking(
        &self,
    ) -> Result<Vec<ChatSessionMeta>, String> {
        block_on_gateway(self.session_resume_candidates())
    }

    /// 读取 Gateway 记录的当前会话作用域。
    pub(crate) fn session_scope_get_blocking(&self) -> Result<Option<String>, String> {
        block_on_gateway(self.session_scope_get())
    }

    /// 重命名当前绑定会话；响应 ID 必须仍指向当前会话。
    pub(crate) fn session_rename_blocking(&self, title: &str) -> Result<(), String> {
        block_on_gateway(self.session_rename(title))
    }

    /// 从当前会话末尾或指定持久化消息分叉，并返回 Gateway 分配的新会话 ID。
    pub(crate) fn session_fork_blocking(
        &self,
        message_id: Option<&str>,
    ) -> Result<String, String> {
        block_on_gateway(self.session_fork_from(message_id))
    }

    /// 将当前会话及代码变更回退到指定持久化消息。
    pub(crate) fn session_reset_blocking(&self, message_id: &str) -> Result<(), String> {
        block_on_gateway(self.session_reset_to(message_id))
    }

    /// 从指定持久化消息触发上下文压缩，返回 Gateway 是否执行了压缩。
    pub(crate) fn session_compact_blocking(&self, message_id: &str) -> Result<bool, String> {
        block_on_gateway(self.session_compact_from(message_id))
    }

    /// 在当前目录作用域的归档集合中加入当前会话。
    pub(crate) fn session_archive_blocking(&self) -> Result<(), String> {
        block_on_gateway(self.session_archive_current())
    }
}

fn required_message_id<'a>(operation: &str, message_id: &'a str) -> Result<&'a str, String> {
    normalize_optional_str_ref(Some(message_id))
        .ok_or_else(|| format!("session {operation} requires a non-empty persisted message id"))
}

fn ensure_mutation_session_id(
    operation: &str,
    expected: &str,
    actual: &str,
) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "session {operation} returned mismatched id: expected {expected}, got {actual}"
        ))
    }
}
