//! 验证队列所有者轮次控制器的生命周期、取消和关闭状态转换。

use super::*;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::errors::{AcpxErrorOptions, QueueConnectionError};

#[derive(Debug, Default)]
struct OptionsState {
    void_timeouts: Vec<Option<u64>>,
    config_timeouts: Vec<Option<u64>>,
    mode_fallback_calls: Vec<(String, Option<u64>)>,
    model_fallback_calls: Vec<(String, Option<u64>)>,
    config_fallback_calls: Vec<(String, String, Option<u64>)>,
}

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
    fn with_active_prompt() -> Self {
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

fn ok_void() -> QueueControlFuture<()> {
    Box::pin(async { Ok(()) })
}

fn queue_error(message: impl Into<String>) -> QueueConnectionError {
    QueueConnectionError::new(message, AcpxErrorOptions::default())
}

fn options() -> QueueOwnerTurnControllerOptions {
    options_with_state(Arc::new(Mutex::new(OptionsState::default())))
}

fn options_with_state(state: Arc<Mutex<OptionsState>>) -> QueueOwnerTurnControllerOptions {
    QueueOwnerTurnControllerOptions {
        with_timeout: Arc::new({
            let state = Arc::clone(&state);
            move |future, timeout_ms| {
                lock(&state).void_timeouts.push(timeout_ms);
                future
            }
        }),
        with_timeout_config_option: Arc::new({
            let state = Arc::clone(&state);
            move |future, timeout_ms| {
                lock(&state).config_timeouts.push(timeout_ms);
                future
            }
        }),
        set_session_mode_fallback: Arc::new({
            let state = Arc::clone(&state);
            move |mode_id, timeout_ms| {
                lock(&state).mode_fallback_calls.push((mode_id, timeout_ms));
                ok_void()
            }
        }),
        set_session_model_fallback: Arc::new({
            let state = Arc::clone(&state);
            move |model_id, timeout_ms| {
                lock(&state).model_fallback_calls.push((model_id, timeout_ms));
                ok_void()
            }
        }),
        set_session_config_option_fallback: Arc::new({
            let state = Arc::clone(&state);
            move |config_id, value, timeout_ms| {
                lock(&state).config_fallback_calls.push((config_id, value, timeout_ms));
                Box::pin(async { Ok(SetSessionConfigOptionResponse::new(Vec::new())) })
            }
        }),
    }
}

#[test]
fn turn_controller_tracks_lifecycle_state() {
    let mut controller = QueueOwnerTurnController::new(options());

    assert_eq!(controller.lifecycle_state(), QueueOwnerTurnState::Idle);
    controller.begin_turn();
    assert_eq!(controller.lifecycle_state(), QueueOwnerTurnState::Starting);
    controller.mark_prompt_active();
    assert_eq!(controller.lifecycle_state(), QueueOwnerTurnState::Active);
    controller.end_turn();
    assert_eq!(controller.lifecycle_state(), QueueOwnerTurnState::Idle);
    assert!(!controller.has_pending_cancel());
}

#[test]
fn mark_prompt_active_only_changes_running_turns() {
    let mut controller = QueueOwnerTurnController::new(options());

    controller.mark_prompt_active();
    assert_eq!(controller.lifecycle_state(), QueueOwnerTurnState::Idle);

    controller.begin_closing();
    controller.mark_prompt_active();
    assert_eq!(controller.lifecycle_state(), QueueOwnerTurnState::Closing);
}

#[test]
fn mark_prompt_active_is_idempotent_while_active() {
    let mut controller = QueueOwnerTurnController::new(options());

    controller.begin_turn();
    controller.mark_prompt_active();
    assert_eq!(controller.lifecycle_state(), QueueOwnerTurnState::Active);
    controller.mark_prompt_active();
    assert_eq!(controller.lifecycle_state(), QueueOwnerTurnState::Active);
}

#[tokio::test]
async fn request_cancel_marks_pending_while_prompt_active_state() {
    let mut controller = QueueOwnerTurnController::new(options());
    controller.begin_turn();
    controller.mark_prompt_active();
    assert_eq!(controller.lifecycle_state(), QueueOwnerTurnState::Active);

    assert!(controller.request_cancel().await.expect("cancel should be queued"));
    assert!(controller.has_pending_cancel());
}

#[tokio::test]
async fn request_cancel_returns_false_when_closing() {
    let mut controller = QueueOwnerTurnController::new(options());
    controller.begin_closing();

    assert!(!controller.request_cancel().await.expect("cancel should resolve"));
    assert!(!controller.has_pending_cancel());
}

#[tokio::test]
async fn begin_turn_end_turn_and_begin_closing_clear_pending_cancel() {
    let mut controller = QueueOwnerTurnController::new(options());
    controller.begin_turn();
    assert!(controller.request_cancel().await.expect("cancel should be queued"));

    controller.end_turn();
    assert_eq!(controller.lifecycle_state(), QueueOwnerTurnState::Idle);
    assert!(!controller.has_pending_cancel());

    controller.begin_turn();
    assert!(controller.request_cancel().await.expect("cancel should be queued"));
    controller.begin_closing();
    assert_eq!(controller.lifecycle_state(), QueueOwnerTurnState::Closing);
    assert!(!controller.has_pending_cancel());
}

#[tokio::test]
async fn closing_controller_rejects_control_requests() {
    let mut controller = QueueOwnerTurnController::new(options());
    controller.begin_closing();

    let error = controller
        .set_session_mode("plan", Some(10))
        .await
        .expect_err("closing controller rejects control");
    assert_eq!(error.detail_code(), Some("QUEUE_OWNER_SHUTTING_DOWN"));
    assert_eq!(error.retryable(), Some(true));

    assert!(controller.set_session_model("model", Some(10)).await.is_err());
    assert!(controller.set_session_config_option("theme", "dark", Some(10)).await.is_err());
}

#[tokio::test]
async fn request_cancel_returns_false_when_no_turn_is_active() {
    let mut controller = QueueOwnerTurnController::new(options());

    assert!(!controller.request_cancel().await.expect("cancel should resolve"));
    assert!(!controller.has_pending_cancel());
}

#[tokio::test]
async fn request_cancel_ignores_inactive_controller_without_running_turn() {
    let mut controller = QueueOwnerTurnController::new(options());
    let active_controller = TestActiveController::default();
    controller.set_active_controller(Arc::new(active_controller.clone()));

    assert!(!controller.request_cancel().await.expect("cancel should resolve"));
    assert!(!controller.has_pending_cancel());
    assert_eq!(active_controller.state().cancel_calls, 0);
}

#[tokio::test]
async fn request_cancel_marks_pending_until_active_prompt_can_cancel() {
    let mut controller = QueueOwnerTurnController::new(options());
    let active_controller = TestActiveController::default();
    controller.set_active_controller(Arc::new(active_controller.clone()));
    controller.begin_turn();

    assert!(controller.request_cancel().await.expect("cancel should be queued"));
    assert!(controller.has_pending_cancel());
    assert!(!controller.apply_pending_cancel().await.expect("inactive prompt cannot cancel"));
    assert!(controller.has_pending_cancel());

    lock(&active_controller.state).has_active_prompt = true;
    assert!(controller.apply_pending_cancel().await.expect("active prompt should cancel"));
    assert!(!controller.has_pending_cancel());
    assert_eq!(active_controller.state().cancel_calls, 1);
}

#[tokio::test]
async fn request_cancel_uses_active_controller_and_clears_pending_on_cancel() {
    let mut controller = QueueOwnerTurnController::new(options());
    let active_controller = TestActiveController::with_active_prompt();
    controller.begin_turn();
    assert!(controller.request_cancel().await.expect("cancel should be queued"));
    controller.set_active_controller(Arc::new(active_controller.clone()));

    assert!(controller.request_cancel().await.expect("active cancel should resolve"));
    assert!(!controller.has_pending_cancel());
    assert_eq!(active_controller.state().cancel_calls, 1);
}

#[tokio::test]
async fn apply_pending_cancel_returns_false_without_pending_cancel_or_controller() {
    let mut controller = QueueOwnerTurnController::new(options());

    assert!(!controller.apply_pending_cancel().await.expect("no pending cancel"));

    controller.begin_turn();
    assert!(controller.request_cancel().await.expect("cancel should be queued"));
    assert!(!controller.apply_pending_cancel().await.expect("missing controller cannot cancel"));
    assert!(controller.has_pending_cancel());
}

#[tokio::test]
async fn apply_pending_cancel_preserves_pending_when_active_cancel_returns_false() {
    let mut controller = QueueOwnerTurnController::new(options());
    let active_controller = TestActiveController::with_active_prompt();
    lock(&active_controller.state).cancel_result = Ok(false);
    controller.begin_turn();
    assert!(controller.request_cancel().await.expect("cancel should be queued"));
    controller.set_active_controller(Arc::new(active_controller.clone()));

    assert!(!controller.apply_pending_cancel().await.expect("active cancel should resolve"));
    assert!(controller.has_pending_cancel());
    assert_eq!(active_controller.state().cancel_calls, 1);
}

#[tokio::test]
async fn apply_pending_cancel_propagates_active_controller_errors() {
    let mut controller = QueueOwnerTurnController::new(options());
    let active_controller = TestActiveController::with_active_prompt();
    lock(&active_controller.state).cancel_result = Err("cancel failed".to_string());
    controller.begin_turn();
    assert!(controller.request_cancel().await.expect("cancel should be queued"));
    controller.set_active_controller(Arc::new(active_controller));

    let error =
        controller.apply_pending_cancel().await.expect_err("active cancel error should propagate");
    assert_eq!(error.to_string(), "cancel failed");
    assert!(controller.has_pending_cancel());
}

#[tokio::test]
async fn request_cancel_uses_active_controller_and_preserves_pending_on_failed_cancel() {
    let mut controller = QueueOwnerTurnController::new(options());
    let active_controller = TestActiveController::with_active_prompt();
    lock(&active_controller.state).cancel_result = Ok(false);
    controller.begin_turn();
    assert!(controller.request_cancel().await.expect("cancel should be queued"));
    controller.set_active_controller(Arc::new(active_controller.clone()));

    assert!(!controller.request_cancel().await.expect("active cancel should resolve"));
    assert!(controller.has_pending_cancel());
    assert_eq!(active_controller.state().cancel_calls, 1);
}

#[tokio::test]
async fn request_cancel_propagates_active_controller_errors() {
    let mut controller = QueueOwnerTurnController::new(options());
    let active_controller = TestActiveController::with_active_prompt();
    lock(&active_controller.state).cancel_result = Err("cancel failed".to_string());
    controller.set_active_controller(Arc::new(active_controller));

    let error =
        controller.request_cancel().await.expect_err("active cancel error should propagate");
    assert_eq!(error.to_string(), "cancel failed");
}

#[tokio::test]
async fn control_requests_use_fallbacks_without_active_controller() {
    let options_state = Arc::new(Mutex::new(OptionsState::default()));
    let controller = QueueOwnerTurnController::new(options_with_state(Arc::clone(&options_state)));

    controller.set_session_mode("plan", Some(10)).await.expect("mode fallback");
    controller.set_session_model("model-a", Some(20)).await.expect("model fallback");
    controller.set_session_config_option("theme", "dark", Some(30)).await.expect("config fallback");

    let state = lock(&options_state);
    assert_eq!(state.mode_fallback_calls, vec![("plan".to_string(), Some(10))]);
    assert_eq!(state.model_fallback_calls, vec![("model-a".to_string(), Some(20))]);
    assert_eq!(
        state.config_fallback_calls,
        vec![("theme".to_string(), "dark".to_string(), Some(30))]
    );
}

#[tokio::test]
async fn control_requests_use_active_controller_with_timeouts() {
    let options_state = Arc::new(Mutex::new(OptionsState::default()));
    let mut controller =
        QueueOwnerTurnController::new(options_with_state(Arc::clone(&options_state)));
    let active_controller = TestActiveController::with_active_prompt();
    controller.set_active_controller(Arc::new(active_controller.clone()));

    controller.set_session_mode("plan", Some(11)).await.expect("active mode");
    controller.set_session_model("model-a", Some(12)).await.expect("active model");
    controller.set_session_config_option("theme", "dark", Some(13)).await.expect("active config");

    let options_state = lock(&options_state);
    assert_eq!(options_state.void_timeouts, vec![Some(11), Some(12)]);
    assert_eq!(options_state.config_timeouts, vec![Some(13)]);
    drop(options_state);

    let active_state = active_controller.state();
    assert_eq!(active_state.mode_calls, vec!["plan".to_string()]);
    assert_eq!(active_state.model_calls, vec!["model-a".to_string()]);
    assert_eq!(active_state.config_calls, vec![("theme".to_string(), "dark".to_string())]);
}

#[tokio::test]
async fn clear_active_controller_routes_later_control_to_fallback() {
    let options_state = Arc::new(Mutex::new(OptionsState::default()));
    let mut controller =
        QueueOwnerTurnController::new(options_with_state(Arc::clone(&options_state)));
    controller.set_active_controller(Arc::new(TestActiveController::with_active_prompt()));
    controller.clear_active_controller();

    controller.set_session_mode("plan", None).await.expect("mode fallback");

    let state = lock(&options_state);
    assert!(state.void_timeouts.is_empty());
    assert_eq!(state.mode_fallback_calls, vec![("plan".to_string(), None)]);
}
