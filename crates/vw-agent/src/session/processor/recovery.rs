//! 模型恢复：服务错误使用有限预算，安全的网络重连持续等待至恢复或取消。
use super::llm_runner::provider_retry_backoff_ms;
use super::types::{ModelRecovery, RecoveryPhase, StreamEvent};
use crate::session::message::AssistantError;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

pub(crate) fn retry_allowed(error: &str, is_acp: bool) -> bool {
    if error == "模型响应超时" {
        return true;
    }
    match serde_json::from_str::<AssistantError>(error) {
        Ok(AssistantError::APIError { metadata: Some(metadata), .. })
            if metadata.get("replay_safe").is_some_and(|value| value == "false") =>
        {
            false
        }
        Ok(AssistantError::APIError { is_retryable, .. }) if !is_acp => is_retryable,
        Ok(AssistantError::Unknown { message }) if is_acp => {
            super::llm_runner::is_acp_retryable_error(&message)
        }
        Ok(AssistantError::APIError { message, .. }) if is_acp => {
            super::llm_runner::is_acp_retryable_error(&message)
        }
        _ => false,
    }
}

pub(crate) struct RetryBudget {
    failures: u64,
    current_attempt: u64,
    max_attempts: u64,
    backoff_ms: u64,
}

impl RetryBudget {
    pub(crate) fn new(additional_retries: u32, backoff_ms: u64) -> Self {
        Self {
            failures: 0,
            current_attempt: 0,
            max_attempts: u64::from(additional_retries) + 1,
            backoff_ms,
        }
    }

    fn emit(
        &self,
        sink: &mut dyn FnMut(StreamEvent) -> bool,
        phase: RecoveryPhase,
        wait_ms: u64,
        reason: &str,
        next_action: &str,
    ) -> bool {
        sink(StreamEvent::ModelRecovery(ModelRecovery {
            attempt: self.current_attempt,
            max_attempts: self.max_attempts,
            wait_ms,
            reason: reason.into(),
            phase,
            next_action: next_action.into(),
        }))
    }

    pub(crate) fn run<T>(
        &mut self,
        is_acp: bool,
        cancellation: Option<&CancellationToken>,
        sink: &mut dyn FnMut(StreamEvent) -> bool,
        mut operation: impl FnMut(&mut dyn FnMut(StreamEvent) -> bool) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut recovering = false;
        let mut connection_failures = 0u64;
        loop {
            if cancellation.is_some_and(CancellationToken::is_cancelled) {
                self.emit(sink, RecoveryPhase::Cancelled, 0, "执行已取消", "需要时重新发送请求");
                return Err("执行已取消".into());
            }
            self.current_attempt = self.failures + 1;
            let mut output_started = false;
            let mut consumer_closed = false;
            let result = operation(&mut |event| {
                output_started |= matches!(&event, StreamEvent::Delta(text) if !text.is_empty())
                    || matches!(&event, StreamEvent::Tool(_));
                let accepted = sink(event);
                consumer_closed |= !accepted;
                accepted
            });
            if consumer_closed {
                return Err("stream consumer stopped".into());
            }
            if cancellation.is_some_and(CancellationToken::is_cancelled) {
                self.emit(sink, RecoveryPhase::Cancelled, 0, "执行已取消", "需要时重新发送请求");
                return Err("执行已取消".into());
            }
            match result {
                Ok(value) => {
                    if recovering
                        && !self.emit(
                            sink,
                            RecoveryPhase::Recovered,
                            0,
                            "模型请求已恢复",
                            "继续当前请求",
                        )
                    {
                        return Err("stream consumer stopped".into());
                    }
                    return Ok(value);
                }
                Err(error) => {
                    if error == "stream consumer stopped" {
                        return Err(error);
                    }
                    let transient = retry_allowed(&error, is_acp);
                    let parsed = serde_json::from_str::<AssistantError>(&error).ok();
                    // 仅原生传输层明确标记的故障可无限等待；ACP 远端操作不可盲目重放。
                    let connection_failure = !is_acp
                        && transient
                        && matches!(
                            &parsed,
                            Some(AssistantError::APIError { status_code: None, metadata: Some(metadata), .. })
                                if metadata.get("connection_failure").is_some_and(|value| value == "true")
                        );
                    let exhausted = !connection_failure && self.failures + 1 >= self.max_attempts;
                    let message = serde_json::from_str::<AssistantError>(&error)
                        .ok()
                        .and_then(|err| serde_json::to_value(err).ok())
                        .and_then(|value| {
                            value
                                .get("message")
                                .and_then(serde_json::Value::as_str)
                                .map(str::to_owned)
                        })
                        .unwrap_or_else(|| error.clone());
                    let reason = crate::util::truncate_with_ellipsis(
                        &crate::agent::loop_::scrub_credentials(&message),
                        240,
                    );
                    if output_started || !transient || exhausted {
                        let action = if output_started {
                            "已保留部分输出；请检查结果后重新发送，避免重复工具操作"
                        } else if exhausted {
                            "重试预算已耗尽；请检查网络与模型服务后重新发送"
                        } else {
                            "请检查模型是否启用、配置及鉴权后重新发送"
                        };
                        if !self.emit(sink, RecoveryPhase::Stopped, 0, &reason, action) {
                            return Err("stream consumer stopped".into());
                        }
                        if output_started
                            && (transient
                                || error == "模型未返回内容"
                                || super::llm_runner::is_prompt_too_long_error(&error)
                                || super::llm_runner::input_length_range_max(&error).is_some())
                        {
                            let mut error = serde_json::from_str::<AssistantError>(&error)
                                .unwrap_or(AssistantError::Unknown { message: error });
                            crate::session::llm::prevent_error_replay(&mut error);
                            return Err(
                                serde_json::to_string(&error).expect("assistant error serializes")
                            );
                        }
                        return Err(error);
                    }
                    let wait_ms = if connection_failure {
                        let delay = 5_000u64
                            .saturating_mul(
                                1u64.checked_shl(connection_failures.min(32) as u32)
                                    .unwrap_or(u64::MAX),
                            )
                            .min(60_000);
                        connection_failures = connection_failures.saturating_add(1);
                        delay
                    } else {
                        let local = provider_retry_backoff_ms(
                            self.backoff_ms,
                            self.failures.min(u64::from(u32::MAX)) as u32,
                        );
                        parsed
                            .as_ref()
                            .and_then(|error| match error {
                                AssistantError::APIError { response_headers: Some(_), .. } => Some(
                                    crate::session::retry::delay(self.failures + 1, Some(error)),
                                ),
                                _ => None,
                            })
                            .unwrap_or(local)
                            .min(crate::session::retry::RETRY_MAX_DELAY)
                    };
                    if !self.emit(
                        sink,
                        RecoveryPhase::Waiting,
                        wait_ms,
                        &reason,
                        if connection_failure {
                            "等待网络恢复后重连；可随时取消"
                        } else {
                            "等待后重试同一模型"
                        },
                    ) {
                        return Err("stream consumer stopped".into());
                    }
                    if !connection_failure {
                        self.failures = self.failures.saturating_add(1);
                    }
                    recovering = true;
                    let deadline = Instant::now() + Duration::from_millis(wait_ms);
                    while Instant::now() < deadline {
                        if cancellation.is_some_and(CancellationToken::is_cancelled) {
                            break;
                        }
                        std::thread::sleep(
                            deadline
                                .saturating_duration_since(Instant::now())
                                .min(Duration::from_millis(50)),
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "recovery_tests.rs"]
mod tests;
