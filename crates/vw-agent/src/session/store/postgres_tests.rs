use super::*;

fn sqlite_database() -> (tempfile::TempDir, GlobalDatabase) {
    let home = tempfile::tempdir().expect("temporary database home");
    let database =
        GlobalDatabase::new(home.path().join("global.sqlite3")).expect("SQLite global database");
    (home, database)
}

fn project_scope() -> ProjectScope {
    ProjectScope::new(TenantScope::default(), ProjectId::from("postgres-session-tests"))
        .expect("valid project scope")
}

fn empty_session() -> ChatSession {
    ChatSession {
        id: "session-id".to_string(),
        title: "Session".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    }
}

fn assert_requires_postgres<T>(result: Result<T>) {
    let error = match result {
        Ok(_) => panic!("operation must reject a SQLite backend"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("SQLite global database does not expose a PostgreSQL connection"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn new_rejects_sqlite_backend_before_initializing_postgres_schema() {
    let (_home, database) = sqlite_database();

    let error = PostgresSessionStore::new(database)
        .expect_err("PostgreSQL store must reject a SQLite backend");

    assert!(
        error.to_string().contains("SQLite global database has no PostgreSQL schema transaction")
    );
}

#[test]
fn session_operations_reject_sqlite_backend_before_accessing_session_data() {
    let (_home, database) = sqlite_database();
    let store = PostgresSessionStore { database };
    let scope = project_scope();
    let session = empty_session();

    assert_requires_postgres(store.save_session(&scope, &session));
    assert_requires_postgres(store.load_session(&scope, &session.id));
    assert_requires_postgres(store.load_session_any(scope.tenant_scope(), &session.id));
    assert_requires_postgres(store.list_sessions(&scope));
    assert_requires_postgres(store.load_session_meta(&scope, &session.id));
    assert_requires_postgres(store.replace_session_index(&scope, &[]));
    assert_requires_postgres(store.delete_session(&scope, &session.id));
    assert_requires_postgres(store.save_session_todos(&scope, &session.id, &[]));
    assert_requires_postgres(store.load_session_todos(&scope, &session.id));
}

#[test]
fn agent_session_operations_reject_sqlite_backend_before_querying_rows() {
    let (_home, database) = sqlite_database();
    let store = PostgresSessionStore { database };
    let scope = project_scope();
    let tenant = scope.tenant_scope();

    assert_requires_postgres(store.load_agent_session(&scope, "agent-session"));
    assert_requires_postgres(store.load_agent_sessions(&scope));
    assert_requires_postgres(store.load_agent_sessions_all(tenant));
    assert_requires_postgres(store.load_agent_session_any(tenant, "agent-session"));
    assert_requires_postgres(store.delete_agent_session(&scope, "agent-session"));
}
