//! Gemini transcript deletion, including the legacy JSON left by migration.
use crate::sessions::{cache::TranscriptCache, providers, source::SourceRegistry};
use std::{fs, io};
use vw_api_types::external_session::{ExternalSessionIdentity, SessionProvider, SessionSource};

/// The gateway must authorize deletion and confirm it explicitly. The caller
/// holds its application lifecycle guard for the whole operation; `is_active`
/// queries trusted host runtime state under that guard and fails closed on error.
/// Cleanup can partially succeed, so cache invalidation precedes all mutations.
pub fn delete_session<F>(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    cache: &TranscriptCache,
    mut is_active: F,
) -> io::Result<()>
where
    F: FnMut(&ExternalSessionIdentity) -> io::Result<bool>,
{
    if identity.provider != SessionProvider::Gemini
        || !matches!(&identity.source, SessionSource::File { .. })
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected a Gemini file session"));
    }
    let source = registry.resolve(identity)?;
    if is_active(identity)? {
        return Err(active());
    }
    verify(source.path(), identity)?;
    let sibling = source.path().with_extension(
        if source.path().extension().and_then(|v| v.to_str()) == Some("jsonl") {
            "json"
        } else {
            "jsonl"
        },
    );
    // Only a migrated JSONL may remove its legacy JSON. A stale JSON locator
    // must never delete a newer JSONL transcript.
    let migrated = source.path().extension().and_then(|v| v.to_str()) == Some("jsonl");
    // A sibling is derived only from the verified physical source, never from
    // a client path. Bind it through the same complete identity/root checks.
    let mut sibling_registry = SourceRegistry::new(
        identity.scope.clone(),
        identity.target_host.clone(),
        identity.instance_id.clone(),
        SessionProvider::Gemini,
        vec![source.root().to_path_buf()],
    )?;
    let mut sibling_identity = identity.clone();
    sibling_identity.source = SessionSource::File {
        relative_path: sibling
            .strip_prefix(source.root())
            .map_err(|_| forbidden())?
            .to_str()
            .ok_or_else(forbidden)?
            .to_owned(),
    };
    let has_sibling = match fs::symlink_metadata(&sibling) {
        Ok(_) => {
            // register validates symlinks and containment before any read.
            if !migrated {
                return Err(forbidden());
            }
            let meta = vw_api_types::external_session::SessionMeta {
                identity: sibling_identity.clone(),
                title: None,
                summary: None,
                project_dir: None,
                created_at: None,
                last_active_at: None,
                source_path: None,
                capabilities: vw_api_types::external_session::SessionCapabilities {
                    read: true,
                    delete: true,
                    resume: true,
                    export_markdown: true,
                },
            };
            sibling_registry.register(&meta, sibling.clone())?;
            sibling_registry.resolve(&sibling_identity)?;
            verify(&sibling, &sibling_identity)?;
            true
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => false,
        Err(e) => return Err(storage(e)),
    };
    let current = registry.resolve(identity)?;
    if current.path() != source.path() || current.root() != source.root() {
        return Err(forbidden());
    }
    verify(current.path(), identity)?;
    if has_sibling {
        let current = sibling_registry.resolve(&sibling_identity)?;
        verify(current.path(), &sibling_identity)?;
    }
    if is_active(identity)? {
        return Err(active());
    }
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))?;
    // Invalidate the alternate locator too: both forms identify the same data.
    cache.remove(&sibling_identity).map_err(|_| io::Error::other("session cache unavailable"))?;
    if has_sibling {
        sibling_registry.resolve(&sibling_identity)?;
        fs::remove_file(&sibling).map_err(storage)?;
    } else {
        match fs::symlink_metadata(&sibling) {
            Ok(_) => return Err(forbidden()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(storage(e)),
        }
    }
    registry.resolve(identity)?;
    fs::remove_file(source.path()).map_err(storage)?;
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))
}

fn verify(path: &std::path::Path, identity: &ExternalSessionIdentity) -> io::Result<()> {
    // The registry binds the full host/instance/project identity; additionally
    // require the native Gemini layout and the ID stored inside the transcript.
    let SessionSource::File { relative_path } = &identity.source else {
        return Err(forbidden());
    };
    let parts: Vec<_> = std::path::Path::new(relative_path).components().collect();
    if parts.len() != 4
        || parts[0].as_os_str() != "tmp"
        || parts[2].as_os_str() != "chats"
        || !matches!(path.extension().and_then(|v| v.to_str()), Some("json" | "jsonl"))
    {
        return Err(forbidden());
    }
    let data = fs::read_to_string(path).map_err(storage)?;
    let document = providers::gemini::parse_session_document(&data).ok_or_else(forbidden)?;
    if document.get("sessionId").and_then(serde_json::Value::as_str)
        != Some(identity.session_id.0.as_str())
    {
        return Err(forbidden());
    }
    Ok(())
}
fn active() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "active session cannot be deleted")
}
fn forbidden() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Gemini session source mismatch or unsafe sibling",
    )
}
fn storage(error: io::Error) -> io::Error {
    io::Error::new(error.kind(), "Gemini session storage operation failed")
}
