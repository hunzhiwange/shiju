//! 活动 TUI 宿主需要的 Gateway 会话恢复、发布读取与快照保存操作。
//!
//! 异步方法集中处理传输错误和会话 ID 校验；同步包装仅供当前终端宿主
//! 在非异步事件循环边界调用，不向状态模型泄漏 Gateway 客户端。

use std::collections::HashSet;

use serde::Deserialize;
use vw_gateway_client::{
    GatewaySessionForkBody, GatewaySessionPatchBody, GatewaySessionResetBody,
    GatewaySessionSummarizeBody,
};
use vw_shared::session::info::Info;
use vw_shared::session::ui_types::{ChatSession, ChatSessionMeta};

use super::gateway::{GatewayUiRuntime, block_on_gateway, normalize_optional_str_ref};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct UiSessionMutationInfo {
    id: String,
}

impl GatewayUiRuntime {
    async fn session_ui_load(&self, session_id: &str) -> Result<ChatSession, String> {
        let session_id = normalize_optional_str_ref(Some(session_id))
            .ok_or_else(|| "session restore requires a non-empty session id".to_string())?;
        let directory = self.directory_value();
        let snapshot = self
            .client()
            .session_ui_get_without_raw(session_id, directory.as_deref())
            .await?
            .map(|loaded| loaded.session)
            .ok_or_else(|| format!("session {session_id} has no UI snapshot"))?;
        if snapshot.id != session_id {
            return Err(format!(
                "session restore returned mismatched id: expected {session_id}, got {}",
                snapshot.id
            ));
        }
        Ok(snapshot)
    }

    async fn session_ui_resume(&self, session_id: &str) -> Result<ChatSession, String> {
        let session_id = normalize_optional_str_ref(Some(session_id))
            .ok_or_else(|| "session restore requires a non-empty session id".to_string())?;
        let directory = self.directory_value();
        let info: Info = self.client().session_get(session_id, directory.as_deref()).await?;
        ensure_mutation_session_id("resume", session_id, &info.id)?;
        if !info.is_user_navigable() {
            return Err(format!("session {session_id} cannot be opened in the TUI"));
        }
        self.session_ui_load(session_id).await
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
        if !previews.is_empty() {
            let sessions: Vec<Info> = self.client().session_list(directory.as_deref()).await?;
            let navigable: HashSet<_> = sessions
                .iter()
                .filter(|session| session.is_user_navigable())
                .map(|session| session.id.as_str())
                .collect();
            previews.retain(|preview| navigable.contains(preview.id.as_str()));
        }
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

    async fn session_rename(&self, title: &str) -> Result<(), String> {
        let session_id = self.resolve_session_id(None)?;
        let title = normalize_optional_str_ref(Some(title))
            .ok_or_else(|| "session rename requires a non-empty title".to_string())?;
        let body = GatewaySessionPatchBody { title: Some(title.to_string()), time: None };
        let directory = self.directory_value();
        let info: UiSessionMutationInfo =
            self.client().session_update(session_id, directory.as_deref(), &body).await?;
        ensure_mutation_session_id("rename", session_id, info.id.as_str())
    }

    async fn session_fork_from(&self, message_id: Option<&str>) -> Result<String, String> {
        let session_id = self.resolve_session_id(None)?;
        let body = Some(GatewaySessionForkBody {
            message_id: normalize_optional_str_ref(message_id).map(ToOwned::to_owned),
            ..Default::default()
        });
        let directory = self.directory_value();
        let info: UiSessionMutationInfo =
            self.client().session_fork(session_id, directory.as_deref(), &body).await?;
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
        let body = GatewaySessionResetBody { message_id: message_id.to_string() };
        let directory = self.directory_value();
        let info: UiSessionMutationInfo =
            self.client().session_reset(session_id, directory.as_deref(), &body).await?;
        ensure_mutation_session_id("reset", session_id, info.id.as_str())
    }

    async fn session_compact_from(&self, message_id: &str) -> Result<bool, String> {
        let session_id = self.resolve_session_id(None)?;
        let message_id = required_message_id("compact", message_id)?;
        let body = GatewaySessionSummarizeBody { message_id: message_id.to_string() };
        let directory = self.directory_value();
        self.client().session_summarize(session_id, directory.as_deref(), &body).await
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

    /// Read the published preview in this directory; pending sessions have no public preview.
    pub(crate) fn session_publication_blocking(&self) -> Result<Option<ChatSessionMeta>, String> {
        let session_id = self.resolve_session_id(None)?;
        let directory = self.directory_value();
        let preview = block_on_gateway(
            self.client().session_preview_meta_get(session_id, directory.as_deref()),
        )?;
        if let Some(preview) = preview.as_ref() {
            ensure_mutation_session_id("preview", session_id, &preview.id)?;
        }
        Ok(preview)
    }

    /// 在当前目录约束下加载完整会话快照，并拒绝空 ID、缺失快照或响应 ID 失配。
    pub(crate) fn session_ui_load_blocking(&self, session_id: &str) -> Result<ChatSession, String> {
        block_on_gateway(self.session_ui_load(session_id))
    }

    /// Restore a user-selected session after verifying its persisted source.
    pub(crate) fn session_ui_resume_blocking(
        &self,
        session_id: &str,
    ) -> Result<ChatSession, String> {
        block_on_gateway(self.session_ui_resume(session_id))
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

    /// 重命名当前绑定会话；响应 ID 必须仍指向当前会话。
    pub(crate) fn session_rename_blocking(&self, title: &str) -> Result<(), String> {
        block_on_gateway(self.session_rename(title))
    }

    /// 从当前会话末尾或指定持久化消息分叉，并返回 Gateway 分配的新会话 ID。
    pub(crate) fn session_fork_blocking(&self, message_id: Option<&str>) -> Result<String, String> {
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

fn ensure_mutation_session_id(operation: &str, expected: &str, actual: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "session {operation} returned mismatched id: expected {expected}, got {actual}"
        ))
    }
}
