use super::*;

fn sqlite_database() -> (tempfile::TempDir, GlobalDatabase) {
    let home = tempfile::tempdir().expect("temporary database home");
    let database =
        GlobalDatabase::new(home.path().join("global.sqlite3")).expect("SQLite global database");
    (home, database)
}

#[test]
fn new_rejects_sqlite_backend_before_initializing_postgres_schema() {
    let (_home, database) = sqlite_database();

    let error = PostgresGatewaySkeyStore::new(database)
        .expect_err("PostgreSQL store must reject a SQLite backend");

    assert!(
        error.to_string().contains("SQLite global database has no PostgreSQL schema transaction")
    );
}

#[test]
fn load_rejects_sqlite_backend_before_querying_tenant_metadata() {
    let (_home, database) = sqlite_database();
    let store = PostgresGatewaySkeyStore { database };
    let tenant = TenantScope::default();

    let error = store.load(&tenant).expect_err("load must require a PostgreSQL connection");

    assert!(
        error
            .to_string()
            .contains("SQLite global database does not expose a PostgreSQL connection")
    );
}

#[test]
fn replace_all_rejects_sqlite_backend_without_mutating_it() {
    let (_home, database) = sqlite_database();
    let store = PostgresGatewaySkeyStore { database };
    let tenant = TenantScope::default();
    let skey = GatewaySkey {
        enabled: true,
        skey: None,
        skey_hash: "a".repeat(64),
        masked_skey: "masked".to_string(),
        name: "test key".to_string(),
        expires_at: None,
    };

    let error = store
        .replace_all(&tenant, &[skey], 1_234)
        .expect_err("replacement must require a PostgreSQL connection");

    assert!(
        error
            .to_string()
            .contains("SQLite global database does not expose a PostgreSQL connection")
    );
}
