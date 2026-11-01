//! 用户提问协调模块。
//!
//! 本模块维护运行时内尚未回答的问题请求，将问题发布到事件总线，并通过
//! oneshot 通道把前端或外部调用方的回答返回给等待中的会话流程。它只保存
//! 当前进程内的临时状态，不负责持久化问题内容。

/// 重新导出共享层的问题数据结构，保持调用方使用统一的 question API。
pub use vw_shared::question::*;

use crate::app::agent::bus;
use crate::app::agent::id;
use crate::app::agent::util::log;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fmt;
use std::sync::LazyLock;
use std::sync::Mutex;
use tokio::sync::oneshot;

static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    let mut tags = Map::new();
    tags.insert("service".to_string(), Value::String("question".to_string()));
    log::create(Some(tags))
});

/// 问题生命周期事件定义。
///
/// 这些事件用于通知 UI 或其他集成方有新问题、已回答或已拒绝。事件载荷由
/// 本模块在 `ask`、`reply` 与 `reject` 中统一构造。
pub mod event {
    use crate::app::agent::bus;

    /// 已创建等待回答的问题请求。
    pub const ASKED: bus::Definition = bus::Definition { r#type: "question.asked" };
    /// 问题请求已经收到回答。
    pub const REPLIED: bus::Definition = bus::Definition { r#type: "question.replied" };
    /// 问题请求被拒绝或等待方已经断开。
    pub const REJECTED: bus::Definition = bus::Definition { r#type: "question.rejected" };
}

/// 问题流程可能返回的错误。
#[derive(Debug)]
pub enum Error {
    /// 生成问题 ID 失败。
    Id(id::Error),
    /// 请求被显式拒绝，或等待回答的通道被关闭。
    Rejected(RejectedError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Id(e) => write!(f, "{}", e),
            Error::Rejected(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<id::Error> for Error {
    fn from(value: id::Error) -> Self {
        Error::Id(value)
    }
}

struct Pending {
    tenant: vw_api_types::id::TenantScope,
    info: Request,
    tx: oneshot::Sender<Result<Vec<Answer>, RejectedError>>,
}

#[derive(Default)]
struct State {
    pending: HashMap<String, Pending>,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));

struct PendingGuard(String);
impl Drop for PendingGuard {
    fn drop(&mut self) {
        let pending = STATE.lock().unwrap_or_else(|e| e.into_inner()).pending.remove(&self.0);
        if let Some(pending) = pending {
            let _ = bus::publish(
                event::REJECTED,
                serde_json::json!({
                    "sessionID": pending.info.session_id,
                    "requestID": pending.info.id,
                }),
                None,
            );
        }
    }
}

/// 创建一个问题请求并等待回答。
///
/// 参数 `input` 包含会话、问题列表以及可选工具来源。本函数会为请求生成 ID，
/// 写入进程内 pending 表，发布 `question.asked` 事件，然后等待 `reply` 或
/// `reject` 通过 oneshot 通道完成。
///
/// 返回回答列表；如果 ID 生成失败会返回 `Error::Id`，如果请求被拒绝或等待
/// 通道被关闭会返回 `Error::Rejected`。
pub async fn ask(input: AskInput) -> Result<Vec<Answer>, Error> {
    let id = id::ascending(id::Prefix::Question, None)?;
    LOGGER.info(
        "asking",
        Some(extra([
            ("id", Value::String(id.clone())),
            ("questions", Value::Number((input.questions.len() as i64).into())),
        ])),
    );

    let (tx, rx) = oneshot::channel::<Result<Vec<Answer>, RejectedError>>();
    let req = Request {
        id: id.clone(),
        session_id: input.session_id,
        questions: input.questions,
        tool: input.tool,
    };

    {
        // 先登记再发布事件，避免事件消费者立即回答时找不到 pending 请求。
        let mut lock = STATE.lock().unwrap_or_else(|e| e.into_inner());
        lock.pending.insert(
            id.clone(),
            Pending {
                tenant: crate::session::ui_store::current_tenant_scope(),
                info: req.clone(),
                tx,
            },
        );
    }

    let _pending_guard = PendingGuard(id);
    let _ = bus::publish(event::ASKED, &req, None);

    match rx.await {
        Ok(Ok(answers)) => Ok(answers),
        Ok(Err(e)) => Err(Error::Rejected(e)),
        Err(_) => Err(Error::Rejected(RejectedError)),
    }
}

/// 回答一个待处理的问题请求。
///
/// 参数 `input` 指定请求 ID 与回答内容。未知请求会被记录为警告并直接忽略；
/// 这让重复点击、过期 UI 事件或已经超时的调用不会影响其他 pending 请求。
pub fn reply(input: ReplyInput) {
    let pending = {
        let mut lock = STATE.lock().unwrap_or_else(|e| e.into_inner());
        if lock
            .pending
            .get(&input.request_id)
            .is_some_and(|p| p.tenant == crate::session::ui_store::current_tenant_scope())
        {
            lock.pending.remove(&input.request_id)
        } else {
            None
        }
    };

    let Some(existing) = pending else {
        LOGGER.warn(
            "reply for unknown request",
            Some(extra([("requestID", Value::String(input.request_id))])),
        );
        return;
    };

    LOGGER.info(
        "replied",
        Some(extra([
            ("requestID", Value::String(existing.info.id.clone())),
            ("answer_count", Value::from(input.answers.len())),
        ])),
    );

    let _ = bus::publish(
        event::REPLIED,
        serde_json::json!({
            "sessionID": existing.info.session_id,
            "requestID": existing.info.id,
            "answers": input.answers,
        }),
        None,
    );

    let _ = existing.tx.send(Ok(input.answers));
}

/// 拒绝一个待处理的问题请求。
///
/// 参数 `request_id` 是需要拒绝的问题 ID。未知请求会被记录为警告并忽略。
/// 成功拒绝时会发布 `question.rejected` 事件，并让等待中的 `ask` 返回
/// `Error::Rejected`。
pub fn reject(request_id: impl Into<String>) {
    let request_id = request_id.into();
    let pending = {
        let mut lock = STATE.lock().unwrap_or_else(|e| e.into_inner());
        if lock
            .pending
            .get(&request_id)
            .is_some_and(|p| p.tenant == crate::session::ui_store::current_tenant_scope())
        {
            lock.pending.remove(&request_id)
        } else {
            None
        }
    };

    let Some(existing) = pending else {
        LOGGER.warn(
            "reject for unknown request",
            Some(extra([("requestID", Value::String(request_id))])),
        );
        return;
    };

    LOGGER.info("rejected", Some(extra([("requestID", Value::String(existing.info.id.clone()))])));

    let _ = bus::publish(
        event::REJECTED,
        serde_json::json!({
            "sessionID": existing.info.session_id,
            "requestID": existing.info.id,
        }),
        None,
    );

    let _ = existing.tx.send(Err(RejectedError));
}

/// 列出当前进程内仍在等待回答的问题请求。
///
/// 返回值是 pending 表中请求信息的快照；调用方不能通过该快照修改内部状态。
pub fn list() -> Vec<Request> {
    let lock = STATE.lock().unwrap_or_else(|e| e.into_inner());
    lock.pending
        .values()
        .filter(|p| p.tenant == crate::session::ui_store::current_tenant_scope())
        .map(|p| p.info.clone())
        .collect()
}

fn extra<const N: usize>(pairs: [(&'static str, Value); N]) -> Map<String, Value> {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    m
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
