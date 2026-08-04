//! session_ui 与会话元数据访问封装。
//!
//! 本模块只负责把 `GatewayUiRuntime` 扩展为新 TUI 可直接消费的
//! session_ui 访问层，覆盖：
//! - session_ui 快照读写
//! - session 列表预览与单条预览元数据
//! - 会话持久化路径读取
//! - 全局 session scope 读写
//!
//! 这里不引入状态层或缓存逻辑，只保留稳定、可复用的 gateway 调用入口。

use serde::Deserialize;
use std::path::PathBuf;
use vw_gateway_client::{
    GatewaySessionCreateBody, GatewaySessionForkBody, GatewaySessionPatchBody,
    GatewaySessionResetBody, GatewaySessionSummarizeBody,
};
use vw_shared::session::ui_types::{ChatSession, ChatSessionMeta};

use super::super::commands::session::{SessionCommand, SessionCommandRuntime, SessionReplacement};
use super::super::state::TuiState;
#[cfg(not(target_arch = "wasm32"))]
use super::gateway::block_on_gateway;
use super::gateway::{
    GatewayUiRuntime, annotate_gateway_transport_error, normalize_optional_str_ref,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct UiSessionCreateInfo {
    pub(crate) id: String,
    pub(crate) title: String,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct GatewaySessionCommandRuntime<'a> {
    runtime: &'a mut GatewayUiRuntime,
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a> GatewaySessionCommandRuntime<'a> {
    pub(crate) fn new(runtime: &'a mut GatewayUiRuntime) -> Self {
        Self { runtime }
    }

    fn replacement_from_snapshot(
        snapshot: ChatSession,
        scope: Option<String>,
    ) -> SessionReplacement {
        let restored = TuiState::from_chat_session(&snapshot);
        SessionReplacement {
            session_id: snapshot.id,
            title: snapshot.title,
            messages: restored.messages,
            scope,
        }
    }

    fn load_required(
        &self,
        session_id: &str,
        scope: Option<String>,
    ) -> Result<SessionReplacement, String> {
        let snapshot = self
            .runtime
            .session_ui_load_any_blocking(Some(session_id))?
            .ok_or_else(|| format!("session {session_id} has no UI snapshot"))?;
        Ok(Self::replacement_from_snapshot(snapshot, scope))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl SessionCommandRuntime for GatewaySessionCommandRuntime<'_> {
    fn persist_current(&mut self, state: &TuiState) -> Result<(), String> {
        let snapshot = state.to_chat_session();
        if snapshot.id.trim().is_empty() {
            return Err("active session id is required before persistence".to_string());
        }
        self.runtime.session_ui_save_blocking(&snapshot)
    }

    fn apply(
        &mut self,
        command: &SessionCommand,
        state: &TuiState,
    ) -> Result<SessionReplacement, String> {
        let current_session_id = state.session.session_id.as_deref();
        let scope = state.session.scope.clone();
        match command {
            SessionCommand::Resume { session_id } => self.load_required(session_id.as_str(), scope),
            SessionCommand::Rename { title } => {
                let info = self
                    .runtime
                    .session_update_title_blocking(current_session_id, title.as_str())?;
                Ok(SessionReplacement {
                    session_id: info.id,
                    title: info.title,
                    messages: state.messages.clone(),
                    scope,
                })
            }
            SessionCommand::Fork { message_id } => {
                let info = self
                    .runtime
                    .session_fork_from_blocking(current_session_id, message_id.as_deref())?;
                match self.runtime.session_ui_load_any_blocking(Some(info.id.as_str()))? {
                    Some(snapshot) => Ok(Self::replacement_from_snapshot(snapshot, scope)),
                    None => Ok(SessionReplacement {
                        session_id: info.id,
                        title: info.title,
                        messages: state.messages.clone(),
                        scope,
                    }),
                }
            }
            SessionCommand::Reset { message_id } => {
                let info = self.runtime.session_reset_to_blocking(
                    current_session_id,
                    message_id.as_str(),
                    true,
                )?;
                self.load_required(info.id.as_str(), scope)
            }
            SessionCommand::Compact { message_id } => {
                if !self
                    .runtime
                    .session_summarize_from_blocking(current_session_id, message_id.as_str())?
                {
                    return Err("gateway did not confirm session summarization".to_string());
                }
                let session_id = current_session_id
                    .ok_or_else(|| "active session id is required".to_string())?;
                self.load_required(session_id, scope)
            }
            SessionCommand::Clear => {
                let created =
                    self.runtime.session_create_blocking(Some(state.session.title.as_str()))?;
                Ok(SessionReplacement {
                    session_id: created.id,
                    title: created.title,
                    messages: Vec::new(),
                    scope,
                })
            }
        }
    }
}

impl GatewayUiRuntime {
    /// 在当前工作区上下文下创建一个新 session。
    pub(crate) async fn session_create(
        &self,
        title: Option<&str>,
    ) -> Result<UiSessionCreateInfo, String> {
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
            .map_err(|err| annotate_gateway_transport_error(err, self.endpoint()))
    }

    pub(crate) async fn session_update_title(
        &self,
        session_id: Option<&str>,
        title: &str,
    ) -> Result<UiSessionCreateInfo, String> {
        let session_id = self.resolve_session_id(session_id)?;
        let title = normalize_optional_str_ref(Some(title))
            .ok_or_else(|| "session rename requires a non-empty title".to_string())?;
        let directory = self.directory_value();
        self.client()
            .session_update(
                session_id,
                directory.as_deref(),
                &GatewaySessionPatchBody { title: Some(title.to_string()), time: None },
            )
            .await
            .map_err(|error| annotate_gateway_transport_error(error, self.endpoint()))
    }

    pub(crate) async fn session_fork_from(
        &self,
        session_id: Option<&str>,
        message_id: Option<&str>,
    ) -> Result<UiSessionCreateInfo, String> {
        let session_id = self.resolve_session_id(session_id)?;
        let directory = self.directory_value();
        let body = Some(GatewaySessionForkBody {
            message_id: normalize_optional_str_ref(message_id).map(str::to_string),
        });
        self.client()
            .session_fork(session_id, directory.as_deref(), &body)
            .await
            .map_err(|error| annotate_gateway_transport_error(error, self.endpoint()))
    }

    pub(crate) async fn session_reset_to(
        &self,
        session_id: Option<&str>,
        message_id: &str,
        revert_code: bool,
    ) -> Result<UiSessionCreateInfo, String> {
        let session_id = self.resolve_session_id(session_id)?;
        let message_id = normalize_optional_str_ref(Some(message_id))
            .ok_or_else(|| "session reset requires a non-empty message id".to_string())?;
        let directory = self.directory_value();
        self.client()
            .session_reset(
                session_id,
                directory.as_deref(),
                &GatewaySessionResetBody { message_id: message_id.to_string(), revert_code },
            )
            .await
            .map_err(|error| annotate_gateway_transport_error(error, self.endpoint()))
    }

    pub(crate) async fn session_summarize_from(
        &self,
        session_id: Option<&str>,
        message_id: &str,
    ) -> Result<bool, String> {
        let session_id = self.resolve_session_id(session_id)?;
        let message_id = normalize_optional_str_ref(Some(message_id))
            .ok_or_else(|| "session summarize requires a non-empty message id".to_string())?;
        let directory = self.directory_value();
        self.client()
            .session_summarize(
                session_id,
                directory.as_deref(),
                &GatewaySessionSummarizeBody { message_id: message_id.to_string() },
            )
            .await
            .map_err(|error| annotate_gateway_transport_error(error, self.endpoint()))
    }

    /// 读取当前 runtime 作用域下的 session_ui 快照。
    ///
    /// 当未显式传入 `session_id` 时，会回退到 runtime 自身绑定的当前会话。
    pub(crate) async fn session_ui_load(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<ChatSession>, String> {
        let session_id = self.resolve_session_id(session_id)?;
        let directory = self.directory_value();
        self.client().session_ui_get(session_id, directory.as_deref()).await
    }

    /// 无目录限制地读取 session_ui 快照。
    pub(crate) async fn session_ui_load_any(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<ChatSession>, String> {
        let session_id = self.resolve_session_id(session_id)?;
        self.client().session_ui_get_any(session_id).await
    }

    /// 保存当前 session_ui 快照。
    ///
    /// 保存时会透传 runtime 的目录上下文，让 gateway 根据工作区解析 scope。
    pub(crate) async fn session_ui_save(&self, session: &ChatSession) -> Result<(), String> {
        let session_id = normalize_optional_str_ref(Some(session.id.as_str()))
            .ok_or_else(|| "session_ui save requires a non-empty session id".to_string())?;
        let directory = self.directory_value();
        self.client().session_ui_save(session_id, directory.as_deref(), session).await.map(|_| ())
    }

    /// 读取当前 runtime 作用域下的 session_ui 预览列表。
    pub(crate) async fn session_ui_previews(&self) -> Result<Vec<ChatSessionMeta>, String> {
        let directory = self.directory_value();
        self.client().session_ui_previews(directory.as_deref()).await
    }

    /// 读取单个会话的预览元数据。
    pub(crate) async fn session_preview_meta(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<ChatSessionMeta>, String> {
        let session_id = self.resolve_session_id(session_id)?;
        let directory = self.directory_value();
        self.client().session_preview_meta_get(session_id, directory.as_deref()).await
    }

    /// 读取单个会话对应的持久化文件路径。
    pub(crate) async fn session_path(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<PathBuf>, String> {
        let session_id = self.resolve_session_id(session_id)?;
        let directory = self.directory_value();
        self.client()
            .session_path_get(session_id, directory.as_deref())
            .await
            .map(|path| path.map(PathBuf::from))
    }

    /// 读取当前全局 session scope。
    pub(crate) async fn session_scope_get(&self) -> Result<Option<String>, String> {
        self.client().session_scope_get(None, None).await
    }

    /// 更新当前全局 session scope。
    pub(crate) async fn session_scope_put(&self, scope: Option<&str>) -> Result<(), String> {
        self.client().session_scope_put(normalize_optional_str_ref(scope)).await.map(|_| ())
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// 以阻塞方式在当前工作区上下文下创建一个新 session。
    pub(crate) fn session_create_blocking(
        &self,
        title: Option<&str>,
    ) -> Result<UiSessionCreateInfo, String> {
        block_on_gateway(self.session_create(title))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn session_update_title_blocking(
        &self,
        session_id: Option<&str>,
        title: &str,
    ) -> Result<UiSessionCreateInfo, String> {
        block_on_gateway(self.session_update_title(session_id, title))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn session_fork_from_blocking(
        &self,
        session_id: Option<&str>,
        message_id: Option<&str>,
    ) -> Result<UiSessionCreateInfo, String> {
        block_on_gateway(self.session_fork_from(session_id, message_id))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn session_reset_to_blocking(
        &self,
        session_id: Option<&str>,
        message_id: &str,
        revert_code: bool,
    ) -> Result<UiSessionCreateInfo, String> {
        block_on_gateway(self.session_reset_to(session_id, message_id, revert_code))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn session_summarize_from_blocking(
        &self,
        session_id: Option<&str>,
        message_id: &str,
    ) -> Result<bool, String> {
        block_on_gateway(self.session_summarize_from(session_id, message_id))
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// 以阻塞方式读取当前 runtime 作用域下的 session_ui 快照。
    pub(crate) fn session_ui_load_blocking(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<ChatSession>, String> {
        block_on_gateway(self.session_ui_load(session_id))
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// 以阻塞方式无目录限制地读取 session_ui 快照。
    pub(crate) fn session_ui_load_any_blocking(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<ChatSession>, String> {
        block_on_gateway(self.session_ui_load_any(session_id))
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// 以阻塞方式保存 session_ui 快照。
    pub(crate) fn session_ui_save_blocking(&self, session: &ChatSession) -> Result<(), String> {
        block_on_gateway(self.session_ui_save(session))
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// 以阻塞方式读取 session_ui 预览列表。
    pub(crate) fn session_ui_previews_blocking(&self) -> Result<Vec<ChatSessionMeta>, String> {
        block_on_gateway(self.session_ui_previews())
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// 以阻塞方式读取单个会话的预览元数据。
    pub(crate) fn session_preview_meta_blocking(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<ChatSessionMeta>, String> {
        block_on_gateway(self.session_preview_meta(session_id))
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// 以阻塞方式读取会话持久化文件路径。
    pub(crate) fn session_path_blocking(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<PathBuf>, String> {
        block_on_gateway(self.session_path(session_id))
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// 以阻塞方式读取当前全局 session scope。
    pub(crate) fn session_scope_get_blocking(&self) -> Result<Option<String>, String> {
        block_on_gateway(self.session_scope_get())
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// 以阻塞方式更新当前全局 session scope。
    pub(crate) fn session_scope_put_blocking(&self, scope: Option<&str>) -> Result<(), String> {
        block_on_gateway(self.session_scope_put(scope))
    }
}
