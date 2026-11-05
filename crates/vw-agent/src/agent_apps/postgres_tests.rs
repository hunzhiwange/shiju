use super::store::IconLibraryStore;
use std::sync::{Arc, Barrier};
use vw_api_types::{agent_apps::AgentAppsErrorCode, id::TenantScope};
const IMAGES: [&str; 2] = [
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==",
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGP4DwABAQEAsTj2FAAAAABJRU5ErkJggg==",
];

#[test]
fn cc_base_icon_postgres_atomic_concurrent_adds() -> anyhow::Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    let db = fixture.database().clone();
    let store = IconLibraryStore::new(db)?;
    let tenant = TenantScope::new("p", "c")?;
    let barrier = Arc::new(Barrier::new(2));
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for (i, image) in IMAGES.iter().enumerate() {
            let store = store.clone();
            let tenant = tenant.clone();
            let barrier = barrier.clone();
            handles.push(scope.spawn(move || {
                barrier.wait();
                store.add_icon(&tenant, "图标", image, &i.to_string(), 1)
            }));
        }
        for handle in handles {
            assert!(!handle.join().unwrap().unwrap().existed);
        }
    });
    assert_eq!(store.read_icons(&tenant)?.len(), 2);
    Ok(())
}

#[test]
fn cc_base_icon_postgres_atomic_business_failure_and_mutations() -> anyhow::Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    let store = IconLibraryStore::new(fixture.database().clone())?;
    let tenant = TenantScope::new("p", "c")?;
    let original = store.add_icon(&tenant, "原名", IMAGES[0], "id", 1).unwrap().icon;
    assert_eq!(store.rename_icon(&tenant, "id", " "), Err(AgentAppsErrorCode::InvalidInput));
    assert_eq!(store.read_icons(&tenant)?, vec![original]);
    assert_eq!(store.rename_icon(&tenant, "id", "新名").unwrap().name, "新名");
    assert!(store.add_icon(&tenant, "重复", IMAGES[0], "other", 2).unwrap().existed);
    store.delete_icon(&tenant, "id").unwrap();
    store.delete_icon(&tenant, "id").unwrap();
    assert!(store.read_icons(&tenant)?.is_empty());
    Ok(())
}

#[test]
fn cc_base_icon_postgres_atomic_sql_failure_rolls_back() -> anyhow::Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    let db = fixture.database().clone();
    let store = IconLibraryStore::new(db.clone())?;
    let tenant = TenantScope::new("p", "c")?;
    let original = store.add_icon(&tenant, "原名", IMAGES[0], "id", 1).unwrap().icon;
    db.connect_postgres()?.batch_execute("CREATE FUNCTION reject_icons() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'rejected'; END $$; CREATE TRIGGER reject_icons AFTER UPDATE ON gateway_settings FOR EACH ROW EXECUTE FUNCTION reject_icons();")?;
    assert_eq!(
        store.rename_icon(&tenant, "id", "新名"),
        Err(AgentAppsErrorCode::StorageUnavailable)
    );
    assert_eq!(store.delete_icon(&tenant, "id"), Err(AgentAppsErrorCode::StorageUnavailable));
    assert_eq!(
        store.add_icon(&tenant, "新增", IMAGES[1], "new", 2),
        Err(AgentAppsErrorCode::StorageUnavailable)
    );
    assert_eq!(store.read_icons(&tenant)?, vec![original]);
    Ok(())
}

#[test]
fn cc_base_icon_postgres_atomic_matches_sqlite() -> anyhow::Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    let home = tempfile::tempdir()?;
    let stores = [
        IconLibraryStore::new(fixture.database().clone())?,
        IconLibraryStore::new(crate::global_database::GlobalDatabase::for_home(home.path())?)?,
    ];
    let tenant = TenantScope::new("p", "c")?;
    for store in &stores {
        store.add_icon(&tenant, "原名", IMAGES[0], "id", 1).unwrap();
        assert!(store.add_icon(&tenant, "重复", IMAGES[0], "duplicate", 2).unwrap().existed);
        store.rename_icon(&tenant, "id", "新名").unwrap();
        assert_eq!(store.rename_icon(&tenant, "id", " "), Err(AgentAppsErrorCode::InvalidInput));
        store.add_icon(&tenant, "另一个", IMAGES[1], "other", 3).unwrap();
        store.delete_icon(&tenant, "other").unwrap();
        store.delete_icon(&tenant, "missing").unwrap();
    }
    assert_eq!(stores[0].read_icons(&tenant)?, stores[1].read_icons(&tenant)?);
    Ok(())
}

#[test]
fn cc_base_icon_postgres_atomic_drop_and_business_error_remove_placeholder() -> anyhow::Result<()> {
    use super::store::{ICON_KEY, ICON_NAMESPACE};
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    let store = IconLibraryStore::new(fixture.database().clone())?;
    let tenant = TenantScope::new("p", "c")?;
    assert_eq!(store.rename_icon(&tenant, "missing", "name"), Err(AgentAppsErrorCode::NotFound));
    store.delete_icon(&tenant, "missing").unwrap();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: Result<(), AgentAppsErrorCode> = store.mutate_postgres(&tenant, |_| {
            panic!("abort after placeholder insertion and row lock");
        });
    }));
    assert!(panic.is_err());
    let count: i64 = fixture
        .database()
        .connect_postgres()?
        .query_one(
            "SELECT count(*) FROM gateway_settings WHERE namespace=$1 AND config_key=$2",
            &[&ICON_NAMESPACE, &ICON_KEY],
        )?
        .get(0);
    assert_eq!(count, 0);
    // The pooled connection must be usable after transaction drop.
    store.add_icon(&tenant, "图标", IMAGES[0], "id", 1).unwrap();
    assert_eq!(store.read_icons(&tenant)?.len(), 1);
    Ok(())
}

#[test]
fn cc_base_icon_postgres_atomic_other_tenant_is_not_locked() -> anyhow::Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    let store = IconLibraryStore::new(fixture.database().clone())?;
    let tenant = TenantScope::new("p", "locked")?;
    store.add_icon(&tenant, "图标", IMAGES[0], "id", 1).unwrap();
    let mut connection = fixture.database().connect_postgres()?;
    let mut other = fixture.database().connect_postgres()?;
    let pid: i32 = connection.query_one("SELECT pg_backend_pid()", &[])?.get(0);
    let other_pid: i32 = other.query_one("SELECT pg_backend_pid()", &[])?.get(0);
    assert_ne!(pid, other_pid);
    let mut transaction = connection.transaction()?;
    transaction.query_one("SELECT payload_json FROM gateway_settings FOR UPDATE", &[])?;
    let (send, receive) = std::sync::mpsc::channel();
    let result = std::thread::scope(|scope| {
        scope.spawn(|| {
            let result = store.add_icon(
                &TenantScope::new("p", "other").unwrap(),
                "图标",
                IMAGES[1],
                "id",
                1,
            );
            send.send(result).unwrap();
        });
        let result = receive.recv_timeout(std::time::Duration::from_secs(5));
        transaction.rollback().unwrap();
        result
    });
    assert!(!result?.unwrap().existed);
    assert_eq!(store.read_icons(&tenant)?.len(), 1);
    Ok(())
}
