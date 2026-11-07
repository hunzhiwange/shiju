use super::*;

#[test]
fn cc04_codex_login_preparation_joins_config_and_pointer_in_one_intent() {
    let f = Fixture::new();
    let auth = login("alice", "team", "new");
    f.write("auth.json", &auth);
    let mut guard = lock_app(AppId::Codex, f.live.path()).unwrap();
    let prepared =
        prepare(&guard, f.input(AuthTarget::ThirdParty { preserve: false }, None)).unwrap();
    let config = WholeFile::new(None, Some(b"model='fake-model'".to_vec()));
    let mut changes = prepared.changes().into_iter().collect::<Vec<_>>();
    changes.push(FileChange {
        name: OsStr::new("config.toml"),
        privacy: FilePrivacy::Private,
        patch: &config,
    });
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(|tx, app, at| {
            if at == Phase::Marked {
                assert_eq!(tx.pending(app).unwrap().unwrap().files.len(), 3);
                return Err(WriteError::Io);
            }
            Ok(())
        }))
    });
    assert!(
        f.tx.commit(
            &TenantScope::new("p", "c").unwrap(),
            &mut guard,
            &changes,
            PendingTarget { pointer: Some("relay".into()), mode: None, ..Default::default() }
        )
        .is_err()
    );
    HOOK.with_borrow_mut(|h| *h = None);
    assert_eq!(f.tx.recover(&mut guard).unwrap(), Some(RecoveryOutcome::RolledForward));
    assert!(!f.live.path().join("auth.json").exists());
    assert_eq!(f.read(STASH_FILENAME)["logins"]["sub:alice|account:team"], auth);
    assert_eq!(fs::read(f.live.path().join("config.toml")).unwrap(), b"model='fake-model'");
    assert_eq!(f.tx.state(AppId::Codex).unwrap().pointer.unwrap().id, "relay");
}
#[test]
fn cc04_codex_login_delete_missing_is_noop_and_unsafe_target_is_rejected() {
    for unsafe_target in [false, true] {
        let f = Fixture::new();
        let outside = tempfile::NamedTempFile::new().unwrap();
        if unsafe_target {
            symlink(outside.path(), f.live.path().join("auth.json")).unwrap();
        }
        let mut guard = lock_app(AppId::Codex, f.live.path()).unwrap();
        let delete = WholeFile::new(None, None);
        let result = f.tx.commit(
            &TenantScope::new("p", "c").unwrap(),
            &mut guard,
            &[FileChange {
                name: OsStr::new("auth.json"),
                privacy: FilePrivacy::Private,
                patch: &delete,
            }],
            PendingTarget::default(),
        );
        if unsafe_target {
            assert_eq!(result, Err(WriteError::InvalidResource));
        } else {
            result.unwrap();
        }
        assert!(f.tx.pending(AppId::Codex).unwrap().is_none());
        assert!(outside.path().exists());
    }
}
