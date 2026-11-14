//! Capability-based deletion; source paths never select arbitrary operations.
use std::io;
use vw_api_types::external_session::{ExternalSessionIdentity, SessionProvider, SessionSource};

use crate::sessions::{cache::TranscriptCache, source::SourceRegistry};

/// Delete only through the original application's supported implementation.
/// Gateway authorization and explicit confirmation must precede this call. The
/// caller holds its application lifecycle guard throughout; `is_active` queries
/// trusted host state and must fail closed when activity cannot be determined.
/// Unknown providers and absent sources cannot construct the wire identity;
/// empty or unregistered locators fail before any deletion or activity query.
pub fn delete_session<F>(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    cache: &TranscriptCache,
    is_active: F,
) -> io::Result<()>
where
    F: FnMut(&ExternalSessionIdentity) -> io::Result<bool>,
{
    // Reject unsupported capabilities before touching the native store. Do not
    // inspect client-provided SessionMeta capabilities to authorize deletion.
    match identity.provider {
        SessionProvider::Mcode => return Err(unsupported("delete this session in MCode")),
        SessionProvider::Dsh => return Err(unsupported("delete this session in DeepSeek Harness")),
        SessionProvider::Agy => return Err(unsupported("delete this session in Antigravity")),
        _ => {}
    }
    super::super::identity::validate_identity(identity).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, "invalid session deletion identity")
    })?;
    match (identity.provider, &identity.source) {
        (SessionProvider::Claude, SessionSource::File { .. }) =>
            super::claude::delete_session(registry, identity, cache, is_active),
        (SessionProvider::Codex, SessionSource::File { .. }) =>
            super::codex::delete_session(registry, identity, cache, is_active),
        (SessionProvider::Gemini, SessionSource::File { .. }) =>
            super::gemini::delete_session(registry, identity, cache, is_active),
        (SessionProvider::Openclaw, SessionSource::File { .. }) =>
            super::openclaw::delete_session(registry, identity, cache, is_active),
        (SessionProvider::Grokbuild, SessionSource::File { .. }) =>
            super::grokbuild::delete_session(registry, identity, cache, is_active),
        (SessionProvider::Pi, SessionSource::File { .. }) =>
            super::pi::delete_session(registry, identity, cache, is_active),
        (SessionProvider::Omp, SessionSource::File { .. }) =>
            super::omp::delete_session(registry, identity, cache, is_active),
        (SessionProvider::Kimi, SessionSource::File { .. }) =>
            super::kimi::delete_session(registry, identity, cache, is_active),
        (SessionProvider::Opencode, SessionSource::File { .. }) =>
            super::opencode::delete_session(registry, identity, cache, is_active),
        (SessionProvider::Opencode, SessionSource::Sqlite { .. }) =>
            super::opencode_sqlite::delete_session(registry, identity, cache, is_active),
        (SessionProvider::Hermes, SessionSource::File { .. }) =>
            super::hermes::delete_session(registry, identity, cache, is_active),
        (SessionProvider::Hermes, SessionSource::Sqlite { .. }) =>
            super::hermes_sqlite::delete_session(registry, identity, cache, is_active),
        _ => Err(unsupported("session deletion is unsupported for this application and source")),
    }
}

fn unsupported(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, message)
}
