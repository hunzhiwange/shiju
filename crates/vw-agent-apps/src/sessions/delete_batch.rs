//! Ordered, cooperative bulk deletion through the native capability dispatcher.
use std::{collections::HashMap, io};
use vw_api_types::external_session::{
    DeleteSessionOutcome, DeleteSessionRequest, DeleteSessionResult, ExternalSessionError,
    ExternalSessionErrorCode, ExternalSessionIdentity,
};

use super::{cache::TranscriptCache, delete, source::SourceRegistry};

/// Call on the execution host after authorization and explicit confirmation.
/// `registry_for` supplies server-owned bindings, never client-provided roots.
/// The caller retains the application lifecycle guard used by single deletion;
/// `is_active` must query trusted host state and fail closed.
///
/// Each distinct complete identity is attempted at most once, including failed
/// attempts. Duplicates reuse that outcome without touching storage again.
/// Cancellation is latched between items: an in-flight native deletion finishes,
/// while every remaining input gets a cancelled outcome in its original slot.
pub fn delete_sessions<'a, R, A, C>(
    requests: &[DeleteSessionRequest],
    mut registry_for: R,
    cache: &TranscriptCache,
    mut is_active: A,
    mut is_cancelled: C,
    correlation_id: &str,
) -> Vec<DeleteSessionOutcome>
where
    R: FnMut(&ExternalSessionIdentity) -> io::Result<&'a SourceRegistry>,
    A: FnMut(&ExternalSessionIdentity) -> io::Result<bool>,
    C: FnMut() -> bool,
{
    let mut completed = HashMap::<ExternalSessionIdentity, DeleteSessionResult>::new();
    let mut outcomes = Vec::with_capacity(requests.len());
    let mut cancelled = false;
    for request in requests {
        cancelled = cancelled || is_cancelled();
        let result = if cancelled {
            failed(ExternalSessionErrorCode::Cancelled, "session deletion cancelled", correlation_id)
        } else if let Some(result) = completed.get(&request.identity) {
            result.clone()
        } else {
            let result = match registry_for(&request.identity).and_then(|registry| {
                delete::delete_session(registry, &request.identity, cache, &mut is_active)
            }) {
                Ok(()) => DeleteSessionResult::Deleted,
                Err(error) => deletion_error(error.kind(), correlation_id),
            };
            completed.insert(request.identity.clone(), result.clone());
            result
        };
        outcomes.push(DeleteSessionOutcome { identity: request.identity.clone(), result });
    }
    outcomes
}

fn deletion_error(kind: io::ErrorKind, correlation_id: &str) -> DeleteSessionResult {
    use ExternalSessionErrorCode as Code;
    // Never expose adapter errors, native paths, SQL or nested error payloads.
    // NotFound alone does not prove the session was already absent: an adapter
    // dependency may be missing, so preserve it as a failure rather than success.
    let (code, message) = match kind {
        io::ErrorKind::InvalidInput => (Code::InvalidInput, "invalid session deletion request"),
        io::ErrorKind::NotFound => (Code::NotFound, "session or required source not found"),
        io::ErrorKind::Unsupported => (Code::Unsupported, "session deletion is unsupported"),
        io::ErrorKind::PermissionDenied => (Code::Forbidden, "session deletion denied or source changed"),
        io::ErrorKind::WouldBlock => (Code::Conflict, "session is active or busy"),
        io::ErrorKind::InvalidData => (Code::SourceChanged, "session source is invalid or changed"),
        _ => (Code::StorageUnavailable, "session deletion could not be completed"),
    };
    failed(code, message, correlation_id)
}

fn failed(code: ExternalSessionErrorCode, message: &str, correlation_id: &str) -> DeleteSessionResult {
    DeleteSessionResult::Failed {
        error: ExternalSessionError {
            code,
            message: message.into(),
            correlation_id: correlation_id.into(),
        },
    }
}
