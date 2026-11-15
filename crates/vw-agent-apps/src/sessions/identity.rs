//! Native resume checks over an authorized source.
use vw_api_external_session::{ExternalSessionErrorCode, ExternalSessionIdentity};
use super::source::ValidatedSource;
pub use app_session_core::identity::*;

/// Select only the exact original application's verified native identity.
/// A transcript, ShiJu SessionId or ACP display link cannot satisfy this API.
/// Callers must still reauthorize and apply the provider's resume capabilities.
pub fn native_resume_identity<'a>(
    source: &'a ValidatedSource,
    requested: &ExternalSessionIdentity,
) -> Result<&'a ExternalSessionIdentity, ExternalSessionErrorCode> {
    validate_identity(requested)?;
    if source.identity() != requested {
        return Err(ExternalSessionErrorCode::Forbidden);
    }
    Ok(source.identity())
}
