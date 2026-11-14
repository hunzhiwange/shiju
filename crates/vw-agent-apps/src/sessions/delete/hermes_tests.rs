use super::hermes::delete_session;
use crate::sessions::{cache::TranscriptCache, providers::hermes::scan_sessions, source::SourceRegistry};
use std::{fs, io, path::PathBuf};
use vw_api_types::{ProjectId, ProjectScope, external_session::*};

fn fixture() -> (tempfile::TempDir, PathBuf, SourceRegistry, ExternalSessionIdentity) {
    let root = tempfile::tempdir().unwrap();
    // Canonicalize the temporary root (on macOS /var can itself be a symlink).
    let directory = root.path().canonicalize().unwrap();
    fs::create_dir(directory.join("sessions")).unwrap();
    let path = directory.join("sessions/log.jsonl");
    fs::write(&path, "{\"type\":\"session\",\"id\":\"native-id\"}\n").unwrap();
    let template = ExternalSessionIdentity {
        scope: ProjectScope::for_default_tenant(ProjectId::from("project")).unwrap(),
        target_host: "host".into(), instance_id: "hermes-instance".into(),
        provider: SessionProvider::Hermes, session_id: ExternalSessionId("placeholder".into()),
        source: SessionSource::File { relative_path: "sessions/log.jsonl".into() },
    };
    let meta = scan_sessions(&directory, &template).unwrap().remove(0);
    let mut registry = SourceRegistry::new(template.scope, "host".into(), "hermes-instance".into(),
        SessionProvider::Hermes, vec![directory]).unwrap();
    registry.register(&meta, path.clone()).unwrap();
    (root, path, registry, meta.identity)
}

#[test]
fn external_history_t41_primary_behavior() {
    let (_root, path, registry, identity) = fixture();
    let sibling = path.with_file_name("other.jsonl");
    let database = path.parent().unwrap().parent().unwrap().join("state.db");
    fs::write(&sibling, "other").unwrap();
    fs::write(&database, "database remains untouched").unwrap();
    let cache = TranscriptCache::default();
    let source = registry.resolve(&identity).unwrap();
    let (transcript, _) = cache.get_or_load(&source, || Ok(Vec::new())).unwrap();
    assert_eq!(std::sync::Arc::strong_count(&transcript), 2);
    delete_session(&registry, &identity, &cache, |_| Ok(false)).unwrap();
    assert!(!path.exists());
    assert_eq!(std::sync::Arc::strong_count(&transcript), 1);
    assert_eq!(fs::read_to_string(sibling).unwrap(), "other");
    assert_eq!(fs::read_to_string(database).unwrap(), "database remains untouched");
}

#[test]
fn external_history_t41_boundary_behavior() {
    let (_root, path, registry, identity) = fixture();
    let cache = TranscriptCache::default();
    assert_eq!(delete_session(&registry, &identity, &cache, |_| Ok(true)).unwrap_err().kind(),
        io::ErrorKind::WouldBlock);
    let mut foreign = identity.clone();
    foreign.target_host = "other-host".into();
    assert!(delete_session(&registry, &foreign, &cache, |_| panic!("unauthorized activity query")).is_err());
    foreign = identity.clone();
    foreign.source = SessionSource::File { relative_path: "../outside.jsonl".into() };
    assert!(delete_session(&registry, &foreign, &cache, |_| Ok(false)).is_err());
    foreign = identity.clone();
    foreign.source = SessionSource::Sqlite { database: "state.db".into(), record_id: "native-id".into() };
    assert_eq!(delete_session(&registry, &foreign, &cache, |_| Ok(false)).unwrap_err().kind(),
        io::ErrorKind::InvalidInput);
    assert!(delete_session(&registry, &identity, &cache, |_| Err(io::Error::other("offline"))).is_err());
    assert!(path.exists());
}

#[test]
fn external_history_t41_rechecks_native_id_after_activity_callback() {
    let (_root, path, registry, identity) = fixture();
    let mut calls = 0;
    let result = delete_session(&registry, &identity, &TranscriptCache::default(), |_| {
        calls += 1;
        if calls == 2 {
            fs::write(&path, "{\"type\":\"session\",\"id\":\"another-id\"}\n")?;
        }
        Ok(false)
    });
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    assert!(path.exists());
}

#[cfg(unix)]
#[test]
fn external_history_t41_rejects_replaced_symlink() {
    let (_root, path, registry, identity) = fixture();
    let outside = tempfile::NamedTempFile::new().unwrap();
    fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(outside.path(), &path).unwrap();
    assert!(delete_session(&registry, &identity, &TranscriptCache::default(), |_| Ok(false)).is_err());
    assert!(outside.path().exists());
}
