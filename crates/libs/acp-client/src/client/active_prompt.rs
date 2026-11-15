//! 活动 prompt 的取消状态管理。
//!
//! 本模块只维护当前运行中的 prompt 控制通道。actor 在 prompt 生命周期中注册、
//! 清理和更新 session id；外部调用方通过公共方法请求取消。

use super::*;

impl AcpClient {
    /// 判断当前是否存在仍在运行的提示词请求。
    pub fn has_active_prompt(&self) -> bool {
        self.active_prompt.lock().is_some()
    }

    /// 请求取消指定会话上的活动提示词。
    ///
    /// 返回 `Ok(true)` 表示已向活动提示词发送取消信号，`Ok(false)` 表示没有匹配
    /// 的活动提示词。底层取消通道发送失败时返回 [`AcpError::Cancel`]。
    pub async fn cancel(&self, session_id: impl AsRef<str>) -> Result<bool, AcpError> {
        let session_id = session_id.as_ref();
        let active_prompt = self.active_prompt.lock().clone();
        let Some(active_prompt) = active_prompt else {
            return Ok(false);
        };
        if active_prompt.session_id != session_id {
            return Ok(false);
        }

        self.cancelling_session_ids.lock().insert(session_id.to_string());
        active_prompt.cancel_tx.send(true).map_err(|err| AcpError::Cancel(err.to_string()))?;
        Ok(true)
    }

    /// 请求取消当前活动提示词，但不等待其完成。
    pub async fn request_cancel_active_prompt(&self) -> Result<bool, AcpError> {
        let active_prompt = self.active_prompt.lock().clone();
        let Some(active_prompt) = active_prompt else {
            return Ok(false);
        };
        self.cancel(active_prompt.session_id).await
    }

    /// 请求取消当前活动提示词，并可等待完成通知。
    pub async fn cancel_active_prompt(&self, wait_ms: u64) -> Result<bool, AcpError> {
        let active_prompt = self.active_prompt.lock().clone();
        let Some(active_prompt) = active_prompt else {
            return Ok(false);
        };

        let requested = self.cancel(&active_prompt.session_id).await?;
        if !requested || wait_ms == 0 {
            return Ok(requested);
        }

        let mut completed_rx = active_prompt.completed_rx.clone();
        if *completed_rx.borrow() {
            return Ok(true);
        }

        match timeout(Duration::from_millis(wait_ms), completed_rx.changed()).await {
            Ok(Ok(_)) | Ok(Err(_)) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub(super) fn register_active_prompt(
        &self,
        session_id: String,
        cancel_tx: watch::Sender<bool>,
        completed_rx: watch::Receiver<bool>,
    ) {
        *self.active_prompt.lock() =
            Some(ActivePromptControl { session_id, cancel_tx, completed_rx });
    }

    pub(super) fn clear_active_prompt(&self, session_id: &str) {
        let should_clear = self
            .active_prompt
            .lock()
            .as_ref()
            .is_some_and(|active_prompt| active_prompt.session_id == session_id);
        if should_clear {
            *self.active_prompt.lock() = None;
        }
    }

    pub(super) fn update_active_prompt_session(&self, expected: &str, actual: String) {
        if let Some(active_prompt) = self.active_prompt.lock().as_mut()
            && active_prompt.session_id == expected
        {
            active_prompt.session_id = actual;
        }
    }
}
