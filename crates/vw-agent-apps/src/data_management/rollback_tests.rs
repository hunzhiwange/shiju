use super::{rollback::*, safety_snapshot::*, source::*};
use rusqlite::Connection;
use std::{cell::RefCell, collections::BTreeMap, fs, os::unix::fs::PermissionsExt};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

fn exercise(fail_restore: bool) {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let scope = SourceScope::new(
        ProjectScope::new(TenantScope::new("fixture", "company").unwrap(), ProjectId::from("project")).unwrap(),
        "host".into(), "user".into(),
    ).unwrap();
    let summary = SourceSummary { format: SourceFormat::Sqlite, version: 24,
        main_sha256: [17; 32], wal_sha256: None, counts: BTreeMap::new() };
    let source = SourceSnapshot { scope: scope.clone(), summary: summary.clone(), records: vec![] };
    let targets = [
        SnapshotTarget { category: SnapshotCategory::Provider, object_id: "owner-provider".into() },
        SnapshotTarget { category: SnapshotCategory::Profile, object_id: "owner-profile".into() },
    ];
    let receipt = create_safety_snapshot(root.path(), &SafetySnapshotScope {
        project: scope.project().clone(), host_id: "host".into(), principal_id: "user".into(),
    }, &targets, || Ok(targets.iter().map(|target| OwnerPreimage {
        target: target.clone(), state: Some(PresentPreimage { revision: "0".into(), bytes: b"old".to_vec() }),
    }).collect())).unwrap();
    // Real isolated storage with a reference; only the designated failure
    // boundaries are injected. This tests dispatch recovery, not domain adapters.
    let db = RefCell::new(Connection::open_in_memory().unwrap());
    db.borrow().execute_batch("PRAGMA foreign_keys=ON;
        CREATE TABLE provider(id TEXT PRIMARY KEY, value TEXT, revision INTEGER);
        CREATE TABLE profile(provider_id TEXT REFERENCES provider(id), value TEXT, revision INTEGER);
        INSERT INTO provider VALUES ('owned', 'old', 0);
        INSERT INTO profile VALUES ('owned', 'old', 0);").unwrap();
    let mut recovery_order = Vec::new();
    let report = apply_with_rollback(root.path(), &receipt, &scope, &source, &summary,
        &[SourceCategory::Providers, SourceCategory::Profiles], &targets,
        |category| {
            let table = match category { SourceCategory::Providers => "provider", SourceCategory::Profiles => "profile", _ => panic!() };
            db.borrow().execute(&format!("UPDATE {table} SET value='new', revision=1"), []).unwrap();
            if category == SourceCategory::Profiles { Err("late failure") } else { Ok(()) }
        },
        |category, images| {
            recovery_order.push(category);
            assert_eq!(images.len(), 1);
            if fail_restore && category == SourceCategory::Profiles { return Err("restore unavailable"); }
            let old = std::str::from_utf8(&images[0].state.as_ref().unwrap().bytes).unwrap();
            let table = if category == SourceCategory::Providers { "provider" } else { "profile" };
            let changed = db.borrow().execute(&format!("UPDATE {table} SET value=?1, revision=2 WHERE revision=1"), [old]).unwrap();
            assert_eq!(changed, 1);
            Ok(())
        }).unwrap();
    assert_eq!(report.receipt.id(), receipt.id());
    assert_eq!(report.apply.completed, [SourceCategory::Providers]);
    assert_eq!(report.apply.failed, Some((SourceCategory::Profiles, "late failure")));
    assert_eq!(recovery_order, [SourceCategory::Profiles, SourceCategory::Providers]);
    assert_eq!(report.recovery_required.len(), usize::from(fail_restore));
    if fail_restore {
        assert_eq!(report.recovery_required[0].0, SourceCategory::Profiles);
        assert_eq!(report.restored, [SourceCategory::Providers]);
    } else {
        assert_eq!(report.restored, recovery_order);
        let values: (String, String) = db.borrow().query_row(
            "SELECT p.value, r.value FROM provider p JOIN profile r ON r.provider_id=p.id", [],
            |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(values, ("old".into(), "old".into()));
    }
}

#[test]
fn cc_data_17_rollback_restores_earlier_categories() { exercise(false); }
#[test]
fn cc_data_17_rollback_failure_reports_recovery_required() { exercise(true); }
