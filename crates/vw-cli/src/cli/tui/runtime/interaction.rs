//! 当前会话权限、提问与待办的 Gateway 交互边界。
//!
//! 本模块只负责请求、会话过滤和传输结果判定；overlay 选择、清理与错误展示
//! 由活动宿主处理，避免网络副作用进入 reducer 或渲染 crate。

use vw_gateway_client::{PendingPermissionReplyDto, PendingPermissionRequestDto};
use vw_shared::{question, todo::Todo};

use super::gateway::{GatewayUiRuntime, block_on_gateway};

/// Gateway 中与一个确定会话关联的待处理交互快照。
#[derive(Debug, Clone)]
pub(crate) struct InteractionSnapshot {
    pub(crate) session_id: String,
    pub(crate) permissions: Vec<PendingPermissionRequestDto>,
    pub(crate) questions: Vec<question::Request>,
    pub(crate) todos: Vec<Todo>,
}

impl GatewayUiRuntime {
    /// 加载权限、提问和待办，并在传输边界丢弃其他会话的数据。
    pub(crate) async fn interaction_snapshot(
        &self,
        session_id: Option<&str>,
    ) -> Result<InteractionSnapshot, String> {
        let session_id = self.resolve_session_id(session_id)?.to_string();
        let directory = self.directory_value();
        let permissions = self
            .client()
            .permission_list(directory.as_deref())
            .await?
            .into_iter()
            .filter(|request| request.session_id == session_id)
            .collect();
        let questions = self
            .client()
            .question_list()
            .await?
            .into_iter()
            .filter(|request| request.session_id == session_id)
            .collect();
        let todos = self
            .client()
            .session_todo_get(session_id.as_str(), directory.as_deref())
            .await?;
        Ok(InteractionSnapshot { session_id, permissions, questions, todos })
    }

    /// 阻塞包装仅供后台工作线程调用，不允许从 reducer 或渲染路径直接使用。
    pub(crate) fn interaction_snapshot_blocking(
        &self,
        session_id: Option<&str>,
    ) -> Result<InteractionSnapshot, String> {
        block_on_gateway(self.interaction_snapshot(session_id))
    }

    /// 回复一个权限请求；Gateway 返回 false 也视为失败，不能本地假定已授权。
    pub(crate) async fn permission_reply(
        &self,
        request_id: &str,
        reply: PendingPermissionReplyDto,
        message: Option<String>,
    ) -> Result<(), String> {
        let directory = self.directory_value();
        if self
            .client()
            .permission_reply(request_id, reply, directory.as_deref(), message)
            .await?
        {
            Ok(())
        } else {
            Err(format!("gateway did not resolve permission request {request_id}"))
        }
    }

    pub(crate) fn permission_reply_blocking(
        &self,
        request_id: &str,
        reply: PendingPermissionReplyDto,
        message: Option<String>,
    ) -> Result<(), String> {
        block_on_gateway(self.permission_reply(request_id, reply, message))
    }

    /// 回答一个提问请求；仅显式 true 才允许宿主清理本地 overlay。
    pub(crate) async fn question_reply(
        &self,
        request_id: &str,
        answers: Vec<Vec<String>>,
    ) -> Result<(), String> {
        if self.client().question_reply(request_id, answers).await? {
            Ok(())
        } else {
            Err(format!("gateway did not resolve question request {request_id}"))
        }
    }

    pub(crate) fn question_reply_blocking(
        &self,
        request_id: &str,
        answers: Vec<Vec<String>>,
    ) -> Result<(), String> {
        block_on_gateway(self.question_reply(request_id, answers))
    }

    /// 显式拒绝一个提问请求；断连或未知响应不会被解释为成功。
    pub(crate) async fn question_reject(&self, request_id: &str) -> Result<(), String> {
        if self.client().question_reject(request_id).await? {
            Ok(())
        } else {
            Err(format!("gateway did not reject question request {request_id}"))
        }
    }

    pub(crate) fn question_reject_blocking(&self, request_id: &str) -> Result<(), String> {
        block_on_gateway(self.question_reject(request_id))
    }
}
