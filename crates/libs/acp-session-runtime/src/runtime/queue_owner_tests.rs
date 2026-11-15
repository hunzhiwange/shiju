//! 验证运行时队列所有者控制桥接、超时错误和不支持操作反馈。
//!
//! 最小会话记录、控制器和异步通道 fixture 用于驱动运行时状态与竞态分支。

use super::*;
use std::sync::{Mutex, MutexGuard};

use crate::errors::{AcpxErrorOptions, QueueConnectionError};
use crate::queue_owner_turn_controller::QueueOwnerActiveSessionController;
use agent_client_protocol::SetSessionConfigOptionResponse;

#[derive(Debug)]
struct ActiveControllerState {
    has_active_prompt: bool,
    cancel_result: Result<bool, String>,
    cancel_calls: usize,
    mode_calls: Vec<String>,
    model_calls: Vec<String>,
    config_calls: Vec<(String, String)>,
}

impl Default for ActiveControllerState {
    fn default() -> Self {
        Self {
            has_active_prompt: false,
            cancel_result: Ok(true),
            cancel_calls: 0,
            mode_calls: Vec::new(),
            model_calls: Vec::new(),
            config_calls: Vec::new(),
        }
    }
}

#[derive(Clone, Default)]
struct TestActiveController {
    state: Arc<Mutex<ActiveControllerState>>,
}

impl TestActiveController {
    fn active() -> Self {
        let controller = Self::default();
        lock(&controller.state).has_active_prompt = true;
        controller
    }

    fn state(&self) -> MutexGuard<'_, ActiveControllerState> {
        lock(&self.state)
    }
}

impl QueueOwnerActiveSessionController for TestActiveController {
    fn has_active_prompt(&self) -> bool {
        lock(&self.state).has_active_prompt
    }

    fn request_cancel_active_prompt(&self) -> QueueControlFuture<bool> {
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            let mut state = lock(&state);
            state.cancel_calls += 1;
            match state.cancel_result.clone() {
                Ok(cancelled) => Ok(cancelled),
                Err(message) => Err(queue_error(message)),
            }
        })
    }

    fn set_session_mode(&self, mode_id: String) -> QueueControlFuture<()> {
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            lock(&state).mode_calls.push(mode_id);
            Ok(())
        })
    }

    fn set_session_model(&self, model_id: String) -> QueueControlFuture<()> {
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            lock(&state).model_calls.push(model_id);
            Ok(())
        })
    }

    fn set_session_config_option(
        &self,
        config_id: String,
        value: String,
    ) -> QueueControlFuture<SetSessionConfigOptionResponse> {
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            lock(&state).config_calls.push((config_id, value));
            Ok(SetSessionConfigOptionResponse::new(Vec::new()))
        })
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn queue_error(message: impl Into<String>) -> QueueConnectionError {
    QueueConnectionError::new(message, AcpxErrorOptions::default())
}

fn passthrough_turn_controller() -> QueueOwnerTurnController {
    QueueOwnerTurnController::new(QueueOwnerTurnControllerOptions {
        with_timeout: Arc::new(|future, _timeout_ms| future),
        with_timeout_config_option: Arc::new(|future, _timeout_ms| future),
        set_session_mode_fallback: Arc::new(|_mode_id, _timeout_ms| {
            unsupported_queue_control_future()
        }),
        set_session_model_fallback: Arc::new(|_model_id, _timeout_ms| {
            unsupported_queue_control_future()
        }),
        set_session_config_option_fallback: Arc::new(|_config_id, _value, _timeout_ms| {
            unsupported_queue_config_option_future()
        }),
    })
}

// 下列行为用例覆盖正常完成、超时中断、回退重试和资源清理。
#[test]
fn normalize_queue_owner_ttl_preserves_zero_and_uses_default() {
    assert_eq!(normalize_queue_owner_ttl_ms(None), DEFAULT_QUEUE_OWNER_TTL_MS);
    assert_eq!(normalize_queue_owner_ttl_ms(Some(0)), 0);
    assert_eq!(normalize_queue_owner_ttl_ms(Some(42)), 42);
}

#[test]
fn timeout_queue_connection_error_marks_retryable_queue_timeout() {
    let error = timeout_queue_connection_error(TimeoutError { timeout_ms: 250 });

    assert_eq!(error.to_string(), "Timed out after 250ms");
    assert_eq!(error.detail_code(), Some("TIMEOUT"));
    assert_eq!(error.origin(), Some(OutputErrorOrigin::Queue));
    assert_eq!(error.retryable(), Some(true));
}

#[tokio::test]
async fn unsupported_queue_control_futures_return_retryable_queue_errors() {
    let error =
        unsupported_queue_control_future().await.expect_err("unsupported control should fail");

    assert_eq!(error.to_string(), "Queue owner control fallback is unavailable");
    assert_eq!(error.origin(), Some(OutputErrorOrigin::Queue));
    assert_eq!(error.retryable(), Some(true));

    let config_error = unsupported_queue_config_option_future()
        .await
        .expect_err("unsupported config control should fail");
    assert_eq!(config_error.to_string(), error.to_string());
}

#[tokio::test]
async fn control_bridge_cancel_returns_false_without_active_cancel_target() {
    let controller = Arc::new(tokio::sync::Mutex::new(passthrough_turn_controller()));
    controller.lock().await.begin_turn();
    let bridge = QueueOwnerControlBridge { controller: Arc::clone(&controller) };

    let cancelled = bridge.cancel_prompt().await.expect("cancel request should resolve");

    assert!(!cancelled);
    assert!(controller.lock().await.has_pending_cancel());
}

#[tokio::test]
async fn control_bridge_cancel_uses_active_controller() {
    let active_controller = TestActiveController::active();
    let mut turn_controller = passthrough_turn_controller();
    turn_controller.set_active_controller(Arc::new(active_controller.clone()));
    let controller = Arc::new(tokio::sync::Mutex::new(turn_controller));
    let bridge = QueueOwnerControlBridge { controller };

    let cancelled = bridge.cancel_prompt().await.expect("active cancel should resolve");

    assert!(cancelled);
    assert_eq!(active_controller.state().cancel_calls, 1);
}

#[tokio::test]
async fn control_bridge_propagates_active_cancel_errors() {
    let active_controller = TestActiveController::active();
    lock(&active_controller.state).cancel_result = Err("cancel failed".to_string());
    let mut turn_controller = passthrough_turn_controller();
    turn_controller.set_active_controller(Arc::new(active_controller));
    let controller = Arc::new(tokio::sync::Mutex::new(turn_controller));
    let bridge = QueueOwnerControlBridge { controller };

    let error = bridge.cancel_prompt().await.expect_err("active cancel error should propagate");

    assert_eq!(error.to_string(), "cancel failed");
}

#[tokio::test]
async fn control_bridge_routes_session_controls_to_controller() {
    let active_controller = TestActiveController::active();
    let mut turn_controller = passthrough_turn_controller();
    turn_controller.set_active_controller(Arc::new(active_controller.clone()));
    let controller = Arc::new(tokio::sync::Mutex::new(turn_controller));
    let bridge = QueueOwnerControlBridge { controller };

    bridge.set_session_mode("plan".to_string(), Some(10)).await.expect("set mode");
    bridge.set_session_model("model-a".to_string(), Some(20)).await.expect("set model");
    bridge
        .set_session_config_option("theme".to_string(), "dark".to_string(), Some(30))
        .await
        .expect("set config option");

    let state = active_controller.state();
    assert_eq!(state.mode_calls, vec!["plan".to_string()]);
    assert_eq!(state.model_calls, vec!["model-a".to_string()]);
    assert_eq!(state.config_calls, vec![("theme".to_string(), "dark".to_string())]);
}

#[tokio::test]
async fn control_bridge_reports_unsupported_session_controls_without_active_controller() {
    let controller = Arc::new(tokio::sync::Mutex::new(passthrough_turn_controller()));
    let bridge = QueueOwnerControlBridge { controller };

    let mode_error = bridge
        .set_session_mode("plan".to_string(), None)
        .await
        .expect_err("mode fallback is unsupported");
    let model_error = bridge
        .set_session_model("model-a".to_string(), None)
        .await
        .expect_err("model fallback is unsupported");
    let config_error = bridge
        .set_session_config_option("theme".to_string(), "dark".to_string(), None)
        .await
        .expect_err("config fallback is unsupported");

    assert_eq!(mode_error.to_string(), "Queue owner control fallback is unavailable");
    assert_eq!(model_error.to_string(), mode_error.to_string());
    assert_eq!(config_error.to_string(), mode_error.to_string());
}
