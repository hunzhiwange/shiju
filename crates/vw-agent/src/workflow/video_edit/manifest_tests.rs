use super::manifest::*;
use crate::global_database::GlobalDatabase;

pub(super) fn candidate() -> VideoEditRunManifest {
    VideoEditRunManifest {
        tenant: vw_api_types::TenantScope::default(),
        run_id: "run".into(),
        run_key: "key".into(),
        source: VideoSourceIdentity {
            path: "/source.mp4".into(),
            bytes: 3,
            modified_ns: 1,
            file_id: "inode".into(),
            sha256: "digest".into(),
        },
        template_sha256: "template".into(),
        model_sha256: "model".into(),
        requirements_sha256: "requirements".into(),
        cloud_scope: CloudVideoScope { brand_id: Some("brand".into()), store_id: None },
        project_id: None,
        upload_id: None,
        asset_id: None,
        jobs: vec![],
        asset_jobs: vec![],
        asset_peak: 0,
        asset_cancelled: false,
        transcript: None,
        state: VideoPrepareState::New,
        revision: 0,
        lease_owner: String::new(),
        lease_until_ms: 0,
    }
}
#[test]
fn video_edit_prepare_ledger_claims_without_project_binding() {
    let root = tempfile::tempdir().unwrap();
    let store =
        VideoEditManifestStore::new(GlobalDatabase::new(root.path().join("db.sqlite")).unwrap())
            .unwrap();
    let result = store.claim(candidate(), "owner", 100);
    assert!(result.is_ok(), "an unbound request must acquire a durable tenant run: {result:?}");
}

fn ledger_contract(store: VideoEditManifestStore) {
    let mut a = store.claim(candidate(), "a", 100).unwrap();
    assert!(store.claim(candidate(), "b", 101).is_err());
    a.state = VideoPrepareState::CreatingUpload;
    a.project_id = Some("project".into());
    store.save(&mut a, 102).unwrap();
    let stale = a.clone();
    store.release(&a).unwrap();
    let b = store.claim(candidate(), "b", 103).unwrap();
    assert_eq!(b.state, VideoPrepareState::CreatingUpload);
    assert_eq!(b.project_id.as_deref(), Some("project"));
    assert!(store.save(&mut a, 104).is_err(), "fence stale writer");
    store.release(&stale).unwrap();
    assert!(store.claim(candidate(), "c", 105).is_err(), "stale release cannot drop newer lease");
    let c = store.claim(candidate(), "c", 103 + super::manifest::LEASE_MS).unwrap();
    assert_eq!(c.state, VideoPrepareState::CreatingUpload, "expiry must not reset unknown write");
    let mut other = candidate();
    other.tenant = vw_api_types::TenantScope::new("platform", "different-company").unwrap();
    let other = store.claim(other, "other", 104).unwrap();
    assert_eq!(other.state, VideoPrepareState::New);
    store.release(&c).unwrap();
    store.release(&other).unwrap();
}
#[test]
fn video_edit_prepare_ledger_fences_stale_writers_and_tenants() {
    let dir = tempfile::tempdir().unwrap();
    ledger_contract(
        VideoEditManifestStore::new(GlobalDatabase::new(dir.path().join("db")).unwrap()).unwrap(),
    );
}
#[test]
fn video_edit_prepare_ledger_postgres_contract() {
    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    ledger_contract(VideoEditManifestStore::new(fixture.database().clone()).unwrap());
    fixture.cleanup().unwrap();
}
#[test]
fn video_edit_prepare_ledger_competing_connections_claim_once_and_restart() {
    let dir = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::new(dir.path().join("db")).unwrap();
    let store = VideoEditManifestStore::new(db.clone()).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let outcomes = std::thread::scope(|threads| {
        let handles = (0..8)
            .map(|n| {
                let store = store.clone();
                let barrier = barrier.clone();
                threads.spawn(move || {
                    barrier.wait();
                    store.claim(candidate(), &format!("owner-{n}"), 100)
                })
            })
            .collect::<Vec<_>>();
        handles.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>()
    });
    let mut runs = outcomes.into_iter().filter_map(Result::ok).collect::<Vec<_>>();
    assert_eq!(runs.len(), 1);
    let mut run = runs.pop().unwrap();
    run.state = VideoPrepareState::Uploading;
    run.upload_id = Some("upload-1".into());
    store.save(&mut run, 101).unwrap();
    store.release(&run).unwrap();
    drop(store);
    drop(db);
    let reopened =
        VideoEditManifestStore::new(GlobalDatabase::new(dir.path().join("db")).unwrap()).unwrap();
    let run = reopened.claim(candidate(), "restart", 102).unwrap();
    assert_eq!(run.state, VideoPrepareState::Uploading);
    assert_eq!(run.upload_id.as_deref(), Some("upload-1"));
}
