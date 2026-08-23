//! 验证客户端错误到权限结果的映射以及统计计数更新。

use std::error::Error as StdError;

use agent_client_protocol as acp;
use parking_lot::Mutex;
use serde_json::json;

use crate::errors::{AcpxErrorOptions, PermissionDeniedError, PermissionPromptUnavailableError};
use crate::types::PermissionStats;

use super::client_error::{cancelled_permission_response, map_client_error};

fn stats() -> std::sync::Arc<Mutex<PermissionStats>> {
    std::sync::Arc::new(Mutex::new(PermissionStats::default()))
}

fn map_error(
    err: impl StdError + Send + Sync + 'static,
    permission_stats: &std::sync::Arc<Mutex<PermissionStats>>,
) -> acp::Error {
    map_client_error(Box::new(err), permission_stats)
}

#[test]
fn permission_denied_error_increments_denied_and_preserves_message() {
    let permission_stats = stats();
    let err = PermissionDeniedError::new("write rejected", AcpxErrorOptions::default());

    let mapped = map_error(err, &permission_stats);

    assert_eq!(permission_stats.lock().denied, 1);
    assert_eq!(permission_stats.lock().cancelled, 0);
    assert_eq!(mapped.code, acp::ErrorCode::InternalError);
    assert_eq!(mapped.data, Some(json!("write rejected")));
}

#[test]
fn permission_prompt_unavailable_error_increments_cancelled() {
    let permission_stats = stats();

    let mapped = map_error(PermissionPromptUnavailableError::new(), &permission_stats);

    assert_eq!(permission_stats.lock().denied, 0);
    assert_eq!(permission_stats.lock().cancelled, 1);
    assert_eq!(mapped.code, acp::ErrorCode::InternalError);
    assert_eq!(mapped.data, Some(json!("Permission prompt unavailable in non-interactive mode")));
}

#[test]
fn unrelated_error_does_not_change_permission_stats() {
    let permission_stats = stats();

    let mapped = map_error(std::io::Error::other("client callback failed"), &permission_stats);

    assert_eq!(*permission_stats.lock(), PermissionStats::default());
    assert_eq!(mapped.code, acp::ErrorCode::InternalError);
    assert_eq!(mapped.data, Some(json!("client callback failed")));
}

#[test]
fn cancelled_permission_response_uses_cancelled_outcome_without_meta() {
    let response = cancelled_permission_response();

    assert!(matches!(response.outcome, acp::RequestPermissionOutcome::Cancelled));
    assert!(response.meta.is_none());
    assert_eq!(
        serde_json::to_value(response).expect("serialize response"),
        json!({
            "outcome": {
                "outcome": "cancelled"
            }
        })
    );
}
