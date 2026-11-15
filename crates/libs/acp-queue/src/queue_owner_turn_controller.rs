//! 队列所有者对活动会话的控制接口定义。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use agent_client_protocol::SetSessionConfigOptionResponse;

use acp_types::errors::{AcpxErrorOptions, QueueConnectionError};
use acp_types::types::OutputErrorOrigin;

pub type QueueControlFuture<T> =
    Pin<Box<dyn Future<Output = Result<T, QueueConnectionError>> + Send + 'static>>;

pub trait QueueOwnerActiveSessionController: Send + Sync {
    fn has_active_prompt(&self) -> bool;
    fn request_cancel_active_prompt(&self) -> QueueControlFuture<bool>;
    fn set_session_mode(&self, mode_id: String) -> QueueControlFuture<()>;
    fn set_session_model(&self, model_id: String) -> QueueControlFuture<()>;
    fn set_session_config_option(
        &self,
        config_id: String,
        value: String,
    ) -> QueueControlFuture<SetSessionConfigOptionResponse>;
}

pub type QueueOwnerVoidTimeoutFn =
    Arc<dyn Fn(QueueControlFuture<()>, Option<u64>) -> QueueControlFuture<()> + Send + Sync>;
pub type QueueOwnerConfigOptionTimeoutFn = Arc<
    dyn Fn(
            QueueControlFuture<SetSessionConfigOptionResponse>,
            Option<u64>,
        ) -> QueueControlFuture<SetSessionConfigOptionResponse>
        + Send
        + Sync,
>;
pub type QueueOwnerModeFallbackFn =
    Arc<dyn Fn(String, Option<u64>) -> QueueControlFuture<()> + Send + Sync>;
pub type QueueOwnerModelFallbackFn =
    Arc<dyn Fn(String, Option<u64>) -> QueueControlFuture<()> + Send + Sync>;
pub type QueueOwnerConfigOptionFallbackFn = Arc<
    dyn Fn(String, String, Option<u64>) -> QueueControlFuture<SetSessionConfigOptionResponse>
        + Send
        + Sync,
>;

#[derive(Clone)]
pub struct QueueOwnerTurnControllerOptions {
    pub with_timeout: QueueOwnerVoidTimeoutFn,
    pub with_timeout_config_option: QueueOwnerConfigOptionTimeoutFn,
    pub set_session_mode_fallback: QueueOwnerModeFallbackFn,
    pub set_session_model_fallback: QueueOwnerModelFallbackFn,
    pub set_session_config_option_fallback: QueueOwnerConfigOptionFallbackFn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueOwnerTurnState {
    Idle,
    Starting,
    Active,
    Closing,
}

pub struct QueueOwnerTurnController {
    options: QueueOwnerTurnControllerOptions,
    state: QueueOwnerTurnState,
    pending_cancel: bool,
    active_controller: Option<Arc<dyn QueueOwnerActiveSessionController>>,
}

impl QueueOwnerTurnController {
    pub fn new(options: QueueOwnerTurnControllerOptions) -> Self {
        Self {
            options,
            state: QueueOwnerTurnState::Idle,
            pending_cancel: false,
            active_controller: None,
        }
    }

    pub fn lifecycle_state(&self) -> QueueOwnerTurnState {
        self.state
    }

    pub fn has_pending_cancel(&self) -> bool {
        self.pending_cancel
    }

    pub fn begin_turn(&mut self) {
        self.state = QueueOwnerTurnState::Starting;
        self.pending_cancel = false;
    }

    pub fn mark_prompt_active(&mut self) {
        if matches!(self.state, QueueOwnerTurnState::Starting | QueueOwnerTurnState::Active) {
            self.state = QueueOwnerTurnState::Active;
        }
    }

    pub fn end_turn(&mut self) {
        self.state = QueueOwnerTurnState::Idle;
        self.pending_cancel = false;
    }

    pub fn begin_closing(&mut self) {
        self.state = QueueOwnerTurnState::Closing;
        self.pending_cancel = false;
        self.active_controller = None;
    }

    pub fn set_active_controller(
        &mut self,
        controller: Arc<dyn QueueOwnerActiveSessionController>,
    ) {
        self.active_controller = Some(controller);
    }

    pub fn clear_active_controller(&mut self) {
        self.active_controller = None;
    }

    #[allow(clippy::result_large_err)]
    fn assert_can_handle_control_request(&self) -> Result<(), QueueConnectionError> {
        if self.state == QueueOwnerTurnState::Closing {
            return Err(QueueConnectionError::new(
                "Queue owner is closing",
                AcpxErrorOptions {
                    detail_code: Some("QUEUE_OWNER_SHUTTING_DOWN".to_string()),
                    origin: Some(OutputErrorOrigin::Queue),
                    retryable: Some(true),
                    ..AcpxErrorOptions::default()
                },
            ));
        }
        Ok(())
    }

    pub async fn request_cancel(&mut self) -> Result<bool, QueueConnectionError> {
        let active_controller = self.active_controller.clone();
        if let Some(active_controller) = active_controller
            && active_controller.has_active_prompt()
        {
            let cancelled = active_controller.request_cancel_active_prompt().await?;
            if cancelled {
                self.pending_cancel = false;
            }
            return Ok(cancelled);
        }

        if matches!(self.state, QueueOwnerTurnState::Starting | QueueOwnerTurnState::Active) {
            self.pending_cancel = true;
            return Ok(true);
        }

        Ok(false)
    }

    pub async fn apply_pending_cancel(&mut self) -> Result<bool, QueueConnectionError> {
        if !self.pending_cancel {
            return Ok(false);
        }

        let Some(active_controller) = self.active_controller.clone() else {
            return Ok(false);
        };
        if !active_controller.has_active_prompt() {
            return Ok(false);
        }

        let cancelled = active_controller.request_cancel_active_prompt().await?;
        if cancelled {
            self.pending_cancel = false;
        }
        Ok(cancelled)
    }

    pub async fn set_session_mode(
        &self,
        mode_id: impl Into<String>,
        timeout_ms: Option<u64>,
    ) -> Result<(), QueueConnectionError> {
        self.assert_can_handle_control_request()?;

        let mode_id = mode_id.into();
        if let Some(active_controller) = self.active_controller.clone() {
            return (self.options.with_timeout)(
                active_controller.set_session_mode(mode_id),
                timeout_ms,
            )
            .await;
        }

        (self.options.set_session_mode_fallback)(mode_id, timeout_ms).await
    }

    pub async fn set_session_model(
        &self,
        model_id: impl Into<String>,
        timeout_ms: Option<u64>,
    ) -> Result<(), QueueConnectionError> {
        self.assert_can_handle_control_request()?;

        let model_id = model_id.into();
        if let Some(active_controller) = self.active_controller.clone() {
            return (self.options.with_timeout)(
                active_controller.set_session_model(model_id),
                timeout_ms,
            )
            .await;
        }

        (self.options.set_session_model_fallback)(model_id, timeout_ms).await
    }

    pub async fn set_session_config_option(
        &self,
        config_id: impl Into<String>,
        value: impl Into<String>,
        timeout_ms: Option<u64>,
    ) -> Result<SetSessionConfigOptionResponse, QueueConnectionError> {
        self.assert_can_handle_control_request()?;

        let config_id = config_id.into();
        let value = value.into();
        if let Some(active_controller) = self.active_controller.clone() {
            return (self.options.with_timeout_config_option)(
                active_controller.set_session_config_option(config_id, value),
                timeout_ms,
            )
            .await;
        }

        (self.options.set_session_config_option_fallback)(config_id, value, timeout_ms).await
    }
}

#[cfg(test)]
#[path = "queue_owner_turn_controller_tests.rs"]
mod queue_owner_turn_controller_tests;
