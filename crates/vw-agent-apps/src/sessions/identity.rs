//! External identities never become ShiJu or ACP restore state.
use std::path::{Component, Path};
use vw_api_types::external_session::{ExternalSessionErrorCode, ExternalSessionIdentity, SessionSource};
use vw_api_types::SessionId;

use super::source::ValidatedSource;

/// An observed ACP ID is deliberately distinct from both native and ShiJu IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpDisplayId(pub String);

/// Optional navigation hints supplied by an authorized association lookup.
/// These are display data, never inputs to source resolution or native resume.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionDisplayLinks {
    pub shiju_session_id: Option<SessionId>,
    pub acp_session_id: Option<AcpDisplayId>,
}

/// Validate the complete key before indexing or resolving it. No normalization:
/// different host, instance, provider, source or native ID means another session.
pub fn validate_identity(identity: &ExternalSessionIdentity) -> Result<(), ExternalSessionErrorCode> {
    if [&identity.target_host, &identity.instance_id, &identity.session_id.0]
        .into_iter().any(|value| value.trim().is_empty() || value.contains('\0'))
    {
        return Err(ExternalSessionErrorCode::InvalidInput);
    }
    let relative = match &identity.source {
        SessionSource::File { relative_path } => relative_path,
        SessionSource::Sqlite { database, record_id } => {
            if record_id != &identity.session_id.0 {
                return Err(ExternalSessionErrorCode::InvalidInput);
            }
            database
        }
    };
    if relative.is_empty() || relative.contains('\0')
        || Path::new(relative).components().any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(ExternalSessionErrorCode::InvalidInput);
    }
    Ok(())
}

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
