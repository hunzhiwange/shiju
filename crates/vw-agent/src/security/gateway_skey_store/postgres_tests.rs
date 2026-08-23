//! 验证网关密钥 PostgreSQL 存储拒绝错误数据库后端。

use super::*;

fn sqlite_database() -> (tempfile::TempDir, GlobalDatabase) {
    let home = tempfile::tempdir().expect("temporary database home");
    let database =
        GlobalDatabase::new(home.path().join("global.sqlite3")).expect("SQLite global database");
    (home, database)
}

#[test]
// 后端类型检查必须早于建表，避免在错误数据库上留下部分结构。
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
// 批量替换在后端不匹配时应零写入。
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
