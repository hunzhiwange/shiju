use super::*;

#[test]
fn session_message_delta_storage_failures_remain_retryable() {
    for code in [rusqlite::ffi::SQLITE_BUSY, rusqlite::ffi::SQLITE_LOCKED] {
        let error = anyhow::Error::new(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(code),
            None,
        ))
        .context("persist session message delta");
        let response = session_message_delta_api_error(error);
        assert_eq!(response.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    }
}

#[test]
fn session_message_delta_domain_errors_preserve_client_status() {
    use axum::http::StatusCode;
    use session_store::SessionMessageDeltaError;

    for (error, expected) in [
        (SessionMessageDeltaError::Invalid("duplicate sequence".into()), StatusCode::BAD_REQUEST),
        (SessionMessageDeltaError::Conflict("stale version".into()), StatusCode::CONFLICT),
        (SessionMessageDeltaError::NotFound, StatusCode::NOT_FOUND),
    ] {
        let error = anyhow::Error::new(error).context("apply session delta");
        assert_eq!(session_message_delta_api_error(error).status, expected);
    }
}
