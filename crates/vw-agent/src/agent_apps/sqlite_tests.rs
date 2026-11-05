use super::store::IconLibraryStore;
use crate::global_database::GlobalDatabase;
use std::sync::{Arc, Barrier};
use vw_api_types::{agent_apps::AgentAppsErrorCode, id::TenantScope};
const IMAGES: [&str; 2] = [
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==",
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGP4DwABAQEAsTj2FAAAAABJRU5ErkJggg==",
];

#[test]
fn cc_base_icon_sqlite_atomic_concurrent_adds() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let db = GlobalDatabase::for_home(home.path())?;
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
fn cc_base_icon_sqlite_atomic_business_failure_and_mutations() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let store = IconLibraryStore::new(GlobalDatabase::for_home(home.path())?)?;
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
fn cc_base_icon_sqlite_atomic_sql_failure_rolls_back() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let db = GlobalDatabase::for_home(home.path())?;
    let store = IconLibraryStore::new(db.clone())?;
    let tenant = TenantScope::new("p", "c")?;
    let original = store.add_icon(&tenant, "原名", IMAGES[0], "id", 1).unwrap().icon;
    db.connect_sqlite()?.execute_batch("CREATE TRIGGER reject_icons AFTER UPDATE ON gateway_settings BEGIN SELECT RAISE(ABORT, 'rejected'); END;")?;
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
