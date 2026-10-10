//! 验证会话存储迁移、事务一致性、作用域隔离与往返持久化。
//! 数据库夹具覆盖迁移、回滚、级联删除和多租户同名数据，验证事务失败不破坏旧值。

use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::json;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};
use vw_shared::session::info::{
    Info as AgentSessionInfo, RevertInfo, ShareInfo, Summary, TimeInfo,
};
use vw_shared::session::ui_types::{
    ChatMessage, ChatRole, ChatSession, ChatSessionMeta, ChatSessionStep, ProcessingTiming,
    SessionTodoItem, ThinkTiming, TokenUsage, ToolTiming,
};

use super::store::{LlmRawStepKey, SessionStore};
use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;
use crate::session::llm_raw::LlmRawArtifact;

macro_rules! assert_json_eq {
    ($left:expr, $right:expr) => {
        assert_eq!(serde_json::to_value($left)?, serde_json::to_value($right)?);
    };
}

const SESSION_TABLES: [&str; 9] = [
    "session_agent_sessions",
    "session_calls",
    "session_index",
    "session_llm_raw_rounds",
    "session_llm_raw_turns",
    "session_messages",
    "session_step_llm_raw",
    "session_steps",
    "session_todos",
];

// 先锁定表结构与作用域外键，再复用同一夹具验证各存储提供方的行为契约。
#[test]
fn session_store_schema_uses_project_scoped_session_tables() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let database_path = temp_dir.path().join("database").join("_index.sqlite3");
    let database = GlobalDatabase::new(&database_path)?;
    ProjectStore::new(database.clone())?;

    SessionStore::new(database)?;

    let connection = Connection::open(database_path)?;
    let tables = sqlite_object_names(&connection, "table", "session_%")?;
    assert_eq!(tables, SESSION_TABLES.iter().map(ToString::to_string).collect::<Vec<_>>());
    assert_eq!(
        sqlite_object_names(&connection, "index", "idx_session_%")?,
        vec![
            "idx_session_agent_sessions_parent",
            "idx_session_agent_sessions_project_updated",
            "idx_session_agent_sessions_tenant_updated",
            "idx_session_calls_project_seq",
            "idx_session_index_project_updated",
            "idx_session_index_tenant_updated",
            "idx_session_llm_raw_rounds_project_turn_step",
            "idx_session_llm_raw_turns_project_session",
            "idx_session_messages_project_seq",
            "idx_session_step_llm_raw_project_step",
            "idx_session_steps_project_index",
            "idx_session_steps_project_turn_index",
            "idx_session_todos_project_order",
        ]
    );
    assert!(!sqlite_object_exists(&connection, "table", "agent_sessions")?);
    assert!(!sqlite_object_exists(&connection, "index", "idx_agent_sessions_project_updated")?);
    assert!(sqlite_table_columns(&connection, "session_messages")?.contains(&"model".to_string()));

    for table in SESSION_TABLES {
        assert_scope_columns(&connection, table)?;
        assert_project_foreign_key(&connection, table)?;
    }
    for table in ["session_messages", "session_calls", "session_steps", "session_todos"] {
        assert_session_cascade_foreign_key(&connection, table)?;
    }
    Ok(())
}

#[test]
fn session_store_migrates_legacy_message_model_column() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let database_path = temp_dir.path().join("database").join("_index.sqlite3");
    let database = GlobalDatabase::new(&database_path)?;
    ProjectStore::new(database.clone())?;
    database.connect_sqlite()?.execute_batch(
        "CREATE TABLE session_messages (
            platform_id TEXT NOT NULL,
            company_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            seq_no INTEGER NOT NULL,
            role TEXT NOT NULL,
            message_id TEXT,
            content TEXT NOT NULL,
            think_timing_json TEXT NOT NULL,
            PRIMARY KEY (platform_id, company_id, project_id, session_id, seq_no)
        );",
    )?;

    SessionStore::new(database)?;

    let connection = Connection::open(database_path)?;
    assert!(sqlite_table_columns(&connection, "session_messages")?.contains(&"model".to_string()));
    Ok(())
}

#[test]
fn session_store_migrates_legacy_processing_timing_columns() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let database_path = temp_dir.path().join("database").join("_index.sqlite3");
    let database = GlobalDatabase::new(&database_path)?;
    ProjectStore::new(database.clone())?;
    database.connect_sqlite()?.execute_batch(
        "CREATE TABLE session_messages (
            platform_id TEXT NOT NULL,
            company_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            seq_no INTEGER NOT NULL,
            role TEXT NOT NULL,
            message_id TEXT,
            content TEXT NOT NULL,
            model TEXT,
            think_timing_json TEXT NOT NULL,
            PRIMARY KEY (platform_id, company_id, project_id, session_id, seq_no)
        );",
    )?;

    SessionStore::new(database)?;

    let connection = Connection::open(database_path)?;
    let columns = sqlite_table_columns(&connection, "session_messages")?;
    assert!(columns.contains(&"processing_started_ms".to_string()));
    assert!(columns.contains(&"processing_completed_ms".to_string()));
    assert!(columns.contains(&"processing_elapsed_ms".to_string()));
    assert!(columns.contains(&"processing_llm_elapsed_ms".to_string()));
    Ok(())
}

#[test]
fn session_store_processing_elapsed_round_trips() -> Result<()> {
    for_each_session_store_provider(|fixture| {
        let scope = fixture.create_project(&fixture.tenant_a, "processing-elapsed")?;
        let session = sample_session("processing-elapsed", "Processing elapsed", 200);

        fixture.store.save_session(&scope, &session)?;
        let loaded = fixture.store.load_session(&scope, &session.id)?.expect("stored session");

        assert_eq!(
            loaded.messages[1]
                .processing_timing
                .as_ref()
                .and_then(|timing| timing.processing_elapsed_ms),
            Some(90)
        );
        assert_eq!(
            loaded.messages[1].processing_timing.as_ref().and_then(|timing| timing.llm_elapsed_ms),
            Some(75)
        );
        Ok(())
    })?;
    Ok(())
}

#[test]
fn session_store_tool_timings_round_trip() -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let database = GlobalDatabase::new(sqlite_home.path().join("database/_index.sqlite3"))?;
    let fixture = StoreFixture::new(database)?;
    let scope = fixture.create_project(&fixture.tenant_a, "tool-timings")?;
    let session = sample_session("tool-timings", "Tool timings", 200);

    fixture.store.save_session(&scope, &session)?;
    let loaded = fixture.store.load_session(&scope, &session.id)?.expect("stored session");

    assert_eq!(loaded.steps[0].tool_timings, session.steps[0].tool_timings);
    Ok(())
}

#[test]
fn session_store_lists_llm_raw_step_indices_without_session_steps() -> Result<()> {
    for_each_session_store_provider(|fixture| {
        let scope = fixture.create_project(&fixture.tenant_a, "raw-step-indices")?;
        let mut session = sample_session("raw-step-indices", "Raw step indices", 200);
        session.steps.clear();
        fixture.store.save_session(&scope, &session)?;
        fixture.store.save_session_step_llm_raw(&scope, &session.id, 9, &json!({"step": 9}))?;
        fixture.store.save_session_step_llm_raw(&scope, &session.id, 3, &json!({"step": 3}))?;

        assert_eq!(
            fixture.store.load_session_llm_raw_step_indices(&scope, &session.id)?,
            vec![3, 9]
        );
        Ok(())
    })?;
    Ok(())
}

#[test]
fn session_ui_commit_rejects_finished_step_without_raw() -> Result<()> {
    for_each_session_store_provider(|fixture| {
        let scope = fixture.create_project(&fixture.tenant_a, "commit-missing-raw")?;
        let mut session = sample_session("commit-missing-raw", "Missing raw", 200);
        session.steps[0].turn_id = Some("turn-missing-raw".to_owned());
        session.steps[0].index = 0;
        let expected_steps = [LlmRawStepKey::new("turn-missing-raw", 0)?];

        let error = fixture
            .store
            .commit_session_ui_snapshot(&scope, &session, &[], &expected_steps)
            .expect_err("a finished step without raw must reject the whole commit");

        assert!(error.to_string().contains("missing LLM raw artifact"));
        assert!(fixture.store.load_session(&scope, &session.id)?.is_none());
        assert!(fixture.store.load_session_step_llm_raw(&scope, &session.id, 0)?.is_none());
        Ok(())
    })?;
    Ok(())
}

#[test]
fn session_ui_commit_reconstructs_shared_turn_raw() -> Result<()> {
    for_each_session_store_provider(|fixture| {
        let scope = fixture.create_project(&fixture.tenant_a, "commit-reconstruct-raw")?;
        let (mut session, raw_rounds) = session_with_raw_rounds("commit-reconstruct-raw");
        session.title = "Committed raw".to_owned();
        let artifact = LlmRawArtifact::from_full_raw_rounds("turn-raw", &raw_rounds)?;
        let expected_steps =
            [LlmRawStepKey::new("turn-raw", 0)?, LlmRawStepKey::new("turn-raw", 1)?];

        fixture.store.commit_session_ui_snapshot(&scope, &session, &[artifact], &expected_steps)?;

        let mut expected_session = session.clone();
        expected_session.calls.clear();
        assert_json_eq!(fixture.store.load_session(&scope, &session.id)?, Some(expected_session));
        assert_json_eq!(
            fixture.store.load_session_turn_step_llm_raw(
                &scope,
                "commit-reconstruct-raw",
                "turn-raw",
                0
            )?,
            Some(raw_rounds[0].clone())
        );
        assert_json_eq!(
            fixture.store.load_session_turn_step_llm_raw(
                &scope,
                "commit-reconstruct-raw",
                "turn-raw",
                1
            )?,
            Some(raw_rounds[1].clone())
        );
        assert_eq!(
            fixture.store.load_session_llm_raw_step_indices(&scope, "commit-reconstruct-raw")?,
            vec![0, 1]
        );
        Ok(())
    })?;
    Ok(())
}

#[test]
fn session_ui_commit_distinguishes_reused_step_indices_by_turn() -> Result<()> {
    for_each_session_store_provider(|fixture| {
        let scope = fixture.create_project(&fixture.tenant_a, "commit-reused-step-index")?;
        let (mut session, first_turn_raw_rounds) =
            session_with_raw_rounds("commit-reused-step-index");
        let second_turn_raw = json!({
            "session_id": "commit-reused-step-index",
            "step_index": 0,
            "model": "provider/model-a",
            "system": "system prompt",
            "messages": [{"role": "user", "content": "follow-up"}],
            "message_sources": [{"kind": "user"}],
            "request": {
                "config": {"temperature": 0.2},
                "tools": [{"name": "shell"}]
            },
            "output": {"content": "follow-up final"}
        });
        session.steps.push(ChatSessionStep {
            index: 0,
            turn_id: Some("turn-follow-up".to_owned()),
            ..session.steps[0].clone()
        });
        let first_artifact =
            LlmRawArtifact::from_full_raw_rounds("turn-raw", &first_turn_raw_rounds)?;
        let second_artifact = LlmRawArtifact::from_full_raw_rounds(
            "turn-follow-up",
            std::slice::from_ref(&second_turn_raw),
        )?;
        let expected_steps = [
            LlmRawStepKey::new("turn-raw", 0)?,
            LlmRawStepKey::new("turn-raw", 1)?,
            LlmRawStepKey::new("turn-follow-up", 0)?,
        ];

        fixture.store.commit_session_ui_snapshot(
            &scope,
            &session,
            &[first_artifact, second_artifact],
            &expected_steps,
        )?;

        assert_json_eq!(
            fixture.store.load_session_turn_step_llm_raw(&scope, &session.id, "turn-raw", 0,)?,
            Some(first_turn_raw_rounds[0].clone())
        );
        assert_json_eq!(
            fixture.store.load_session_turn_step_llm_raw(
                &scope,
                &session.id,
                "turn-follow-up",
                0,
            )?,
            Some(second_turn_raw)
        );
        Ok(())
    })?;
    Ok(())
}

// 故意制造原始调用插入失败，确认快照写入与原始数据写入属于同一事务。
#[test]
fn session_ui_commit_rolls_back_snapshot_when_raw_insert_fails() -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let database = GlobalDatabase::new(sqlite_home.path().join("database/_index.sqlite3"))?;
    let fixture = StoreFixture::new(database)?;
    let scope = fixture.create_project(&fixture.tenant_a, "commit-raw-rollback")?;
    let original = sample_session("commit-raw-rollback", "Original", 100);
    fixture.store.save_session(&scope, &original)?;
    fixture.database.connect_sqlite()?.execute_batch(
        "CREATE TRIGGER fail_session_llm_raw_round_insert
         BEFORE INSERT ON session_llm_raw_rounds
         BEGIN
             SELECT RAISE(ABORT, 'injected raw round failure');
         END;",
    )?;
    let (mut replacement, raw_rounds) = session_with_raw_rounds("commit-raw-rollback");
    replacement.title = "Must roll back".to_owned();
    let artifact = LlmRawArtifact::from_full_raw_rounds("turn-raw", &raw_rounds)?;
    let expected_steps = [LlmRawStepKey::new("turn-raw", 0)?, LlmRawStepKey::new("turn-raw", 1)?];

    let error = fixture
        .store
        .commit_session_ui_snapshot(&scope, &replacement, &[artifact], &expected_steps)
        .expect_err("raw insert failure must roll back the snapshot");

    assert!(error.to_string().contains("injected raw round failure"));
    assert_json_eq!(fixture.store.load_session(&scope, "commit-raw-rollback")?, Some(original));
    assert_eq!(scoped_row_count(&fixture.database, "session_llm_raw_turns", &scope)?, 0);
    assert_eq!(scoped_row_count(&fixture.database, "session_llm_raw_rounds", &scope)?, 0);
    Ok(())
}

#[test]
fn session_store_projects_round_trip_without_cross_project_leaks() -> Result<()> {
    for_each_session_store_provider(session_store_projects_round_trip)?;
    Ok(())
}

#[test]
fn stale_ui_content_save_preserves_newer_messages() -> Result<()> {
    for_each_session_store_provider(|fixture| {
        let scope = fixture.create_project(&fixture.tenant_a, "stale-ui-content")?;
        let mut newer = sample_session("stale-ui-content", "Newer", 200);
        newer.messages.push(ChatMessage {
            role: ChatRole::User,
            content: "new tail".to_owned(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        });
        newer.message_ids.push(Some("message-3".to_owned()));
        fixture.store.save_session(&scope, &newer)?;

        let stale = sample_session("stale-ui-content", "Stale", 100);
        let error = fixture
            .store
            .save_session_ui_content(&scope, &stale)
            .expect_err("an older UI snapshot must be rejected");

        assert!(error.to_string().contains("stale session UI content"));
        assert_json_eq!(fixture.store.load_session(&scope, &newer.id)?, Some(newer));
        Ok(())
    })?;
    Ok(())
}

#[test]
fn postgres_session_store_runs_from_tokio_worker_without_nested_runtime() -> Result<()> {
    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let fixture = StoreFixture::new(postgres_fixture.database().clone())?;
    let scope = fixture.create_project(&fixture.tenant_a, "tokio-runtime-project")?;
    let store = fixture.store.clone();
    let session = sample_session("tokio-runtime-session", "Tokio runtime session", 100);

    tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build()?.block_on(
        async move {
            tokio::spawn(async move {
                store.save_session(&scope, &session)?;
                assert_json_eq!(store.load_session(&scope, &session.id)?, Some(session));
                Ok::<(), anyhow::Error>(())
            })
            .await??;
            Ok::<(), anyhow::Error>(())
        },
    )?;

    postgres_fixture.cleanup()?;
    Ok(())
}

fn session_store_projects_round_trip(fixture: &StoreFixture) -> Result<()> {
    let scope_a1 = fixture.create_project(&fixture.tenant_a, "project-1")?;
    let scope_a2 = fixture.create_project(&fixture.tenant_a, "project-2")?;
    let scope_b1 = fixture.create_project(&fixture.tenant_b, "project-1")?;
    let sort_scope = fixture.create_project(&fixture.tenant_a, "sort-project")?;

    let original = sample_session("shared-session", "tenant-a-project-1", 100);
    fixture.store.save_session(&scope_a1, &original)?;
    assert_json_eq!(fixture.store.load_session(&scope_a1, "shared-session")?, Some(original));

    let mut replacement = sample_session("shared-session", "tenant-a-project-1-replaced", 200);
    replacement.messages.truncate(1);
    replacement.message_ids.truncate(1);
    replacement.calls.clear();
    replacement.steps.clear();
    fixture.store.save_session(&scope_a1, &replacement)?;
    assert_json_eq!(
        fixture.store.load_session(&scope_a1, "shared-session")?,
        Some(replacement.clone())
    );

    let session_a2 = sample_session("shared-session", "tenant-a-project-2", 300);
    let session_b1 = sample_session("shared-session", "tenant-b-project-1", 400);
    fixture.store.save_session(&scope_a2, &session_a2)?;
    fixture.store.save_session(&scope_b1, &session_b1)?;

    assert_eq!(fixture.store.list_sessions(&scope_a1)?[0].title, replacement.title);
    assert_eq!(fixture.store.list_sessions(&scope_a2)?[0].title, session_a2.title);
    assert_eq!(fixture.store.list_sessions(&scope_b1)?[0].title, session_b1.title);
    assert_json_eq!(
        fixture.store.load_session_any(&fixture.tenant_a, "shared-session")?,
        Some(session_a2)
    );
    assert_json_eq!(
        fixture.store.load_session_any(&fixture.tenant_b, "shared-session")?,
        Some(session_b1)
    );

    let todos_a1 = sample_todos("a1");
    let todos_a2 = sample_todos("a2");
    fixture.store.save_session_todos(&scope_a1, "shared-session", &todos_a1)?;
    fixture.store.save_session_todos(&scope_a2, "shared-session", &todos_a2)?;
    assert_json_eq!(fixture.store.load_session_todos(&scope_a1, "shared-session")?, todos_a1);
    assert_json_eq!(fixture.store.load_session_todos(&scope_a2, "shared-session")?, todos_a2);

    let agent_a1 = sample_agent_info("agent-shared", "project-1", 500);
    let agent_a2 = sample_agent_info("agent-shared", "project-2", 600);
    let agent_b1 = sample_agent_info("agent-shared", "project-1", 700);
    fixture.store.save_agent_session(&scope_a1, &agent_a1)?;
    fixture.store.save_agent_session(&scope_a2, &agent_a2)?;
    fixture.store.save_agent_session(&scope_b1, &agent_b1)?;
    assert_json_eq!(fixture.store.load_agent_session(&scope_a1, "agent-shared")?, Some(agent_a1));
    assert_json_eq!(
        fixture.store.load_agent_session_any(&fixture.tenant_a, "agent-shared")?,
        Some(agent_a2)
    );
    assert_json_eq!(fixture.store.load_agent_sessions_all(&fixture.tenant_b)?, vec![agent_b1]);

    let sorted: Vec<_> = (0..55)
        .map(|index| ChatSessionMeta {
            id: format!("session-{index:02}"),
            title: format!("Session {index}"),
            updated_ms: index,
            message_count: index as usize,
            call_count: (index % 3) as usize,
            last_content: Some(format!("last {index}")),
        })
        .collect();
    fixture.store.replace_session_index(&sort_scope, &sorted)?;
    let loaded = fixture.store.list_sessions(&sort_scope)?;
    assert_eq!(loaded.len(), 50);
    assert_eq!(loaded[0].id, "session-54");
    assert_eq!(loaded[49].id, "session-05");

    let missing_scope = project_scope(&fixture.tenant_a, "missing-project")?;
    let error = fixture
        .store
        .save_session(&missing_scope, &sample_session("missing", "missing", 1))
        .expect_err("missing project must be rejected by the foreign key");
    let message = format!("{error:#}");
    match fixture.database.provider() {
        DatabaseProvider::Sqlite => assert!(message.contains("FOREIGN KEY constraint failed")),
        DatabaseProvider::Postgres => {
            assert!(message.to_ascii_lowercase().contains("foreign key"));
        }
    }

    let corrupt = sample_session("corrupt-json", "corrupt", 800);
    fixture.store.save_session(&scope_a1, &corrupt)?;
    corrupt_session_call(&fixture.database, &scope_a1, "corrupt-json")?;
    let error = fixture
        .store
        .load_session(&scope_a1, "corrupt-json")
        .expect_err("invalid call JSON must be returned");
    assert!(error.to_string().contains("parse session call payload JSON"));

    Ok(())
}

#[test]
fn session_store_delete_cascades_only_inside_project() -> Result<()> {
    for_each_session_store_provider(session_store_delete_cascade)?;
    Ok(())
}

fn session_store_delete_cascade(fixture: &StoreFixture) -> Result<()> {
    let scope_a1 = fixture.create_project(&fixture.tenant_a, "project-1")?;
    let scope_a2 = fixture.create_project(&fixture.tenant_a, "project-2")?;
    let scope_b1 = fixture.create_project(&fixture.tenant_b, "project-1")?;

    for (scope, title, updated) in
        [(&scope_a1, "a1", 100), (&scope_a2, "a2", 200), (&scope_b1, "b1", 300)]
    {
        fixture.store.save_session(scope, &sample_session("same-id", title, updated))?;
        fixture.store.save_session_todos(scope, "same-id", &sample_todos(title))?;
        fixture.store.save_agent_session(
            scope,
            &sample_agent_info("same-agent", scope.project_id().as_ref(), updated),
        )?;
    }

    assert!(fixture.store.delete_session(&scope_a1, "same-id")?);
    assert!(!fixture.store.delete_session(&scope_a1, "same-id")?);
    assert!(fixture.store.load_session(&scope_a1, "same-id")?.is_none());
    assert!(fixture.store.load_session_todos(&scope_a1, "same-id")?.is_empty());
    assert!(fixture.store.load_session(&scope_a2, "same-id")?.is_some());
    assert!(fixture.store.load_session(&scope_b1, "same-id")?.is_some());

    for table in
        ["session_index", "session_messages", "session_calls", "session_steps", "session_todos"]
    {
        assert_eq!(scoped_row_count(&fixture.database, table, &scope_a1)?, 0, "{table}");
        assert!(scoped_row_count(&fixture.database, table, &scope_a2)? > 0, "{table}");
        assert!(scoped_row_count(&fixture.database, table, &scope_b1)? > 0, "{table}");
    }

    assert!(fixture.store.delete_agent_session(&scope_a1, "same-agent")?);
    assert!(fixture.store.load_agent_session(&scope_a1, "same-agent")?.is_none());
    assert!(fixture.store.load_agent_session(&scope_a2, "same-agent")?.is_some());
    assert!(fixture.store.load_agent_session(&scope_b1, "same-agent")?.is_some());
    Ok(())
}

#[test]
fn session_store_replacements_roll_back_without_losing_existing_rows() -> Result<()> {
    for_each_session_store_provider(session_store_replacements_roll_back)?;
    Ok(())
}

#[test]
fn session_store_rejects_invalid_identifiers_before_persistence() -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let database = GlobalDatabase::new(sqlite_home.path().join("database/_index.sqlite3"))?;
    let fixture = StoreFixture::new(database)?;
    let scope = fixture.create_project(&fixture.tenant_a, "validation-project")?;

    let invalid_session = sample_session("  ", "invalid", 1);
    let error = fixture
        .store
        .save_session(&scope, &invalid_session)
        .expect_err("blank session id must be rejected");
    assert!(error.to_string().contains("session id must not be empty"));

    let duplicate_index = ChatSessionMeta {
        id: "duplicate".to_owned(),
        title: "duplicate".to_owned(),
        updated_ms: 1,
        message_count: 0,
        call_count: 0,
        last_content: None,
    };
    let error = fixture
        .store
        .replace_session_index(&scope, &[duplicate_index.clone(), duplicate_index])
        .expect_err("duplicate session index ids must be rejected");
    assert!(error.to_string().contains("duplicate session id in index replacement"));

    let invalid_todo = SessionTodoItem {
        id: "\t".to_owned(),
        content: "invalid".to_owned(),
        status: "pending".to_owned(),
        priority: "low".to_owned(),
    };
    let error = fixture
        .store
        .save_session_todos(&scope, "valid-session", &[invalid_todo])
        .expect_err("blank todo id must be rejected");
    assert!(error.to_string().contains("todo id must not be empty"));

    let mismatched_agent = sample_agent_info("agent", "another-project", 1);
    let error = fixture
        .store
        .save_agent_session(&scope, &mismatched_agent)
        .expect_err("agent project id must match its scope");
    assert!(error.to_string().contains("agent session project id must match project scope"));

    Ok(())
}

fn session_store_replacements_roll_back(fixture: &StoreFixture) -> Result<()> {
    let scope = fixture.create_project(&fixture.tenant_a, "rollback-project")?;
    let original = sample_session("kept-session", "kept", 100);
    let original_todos = sample_todos("kept");
    fixture.store.save_session(&scope, &original)?;
    fixture.store.save_session_todos(&scope, &original.id, &original_todos)?;

    let mut invalid_session = original.clone();
    invalid_session.title = "must-roll-back".to_owned();
    invalid_session.messages[0].content = "must not replace the original".to_owned();
    invalid_session.steps[0].finished_ms = Some(u64::MAX);
    fixture
        .store
        .save_session(&scope, &invalid_session)
        .expect_err("step overflow must roll back the complete session save");
    assert_json_eq!(fixture.store.load_session(&scope, &original.id)?, Some(original.clone()));

    let replacement = [
        ChatSessionMeta {
            id: "inserted-before-error".to_owned(),
            title: "inserted".to_owned(),
            updated_ms: 200,
            message_count: 0,
            call_count: 0,
            last_content: None,
        },
        ChatSessionMeta {
            id: "overflow".to_owned(),
            title: "overflow".to_owned(),
            updated_ms: u64::MAX,
            message_count: 0,
            call_count: 0,
            last_content: None,
        },
    ];
    fixture
        .store
        .replace_session_index(&scope, &replacement)
        .expect_err("overflow must roll back index replacement");
    assert_json_eq!(fixture.store.load_session(&scope, &original.id)?, Some(original));
    assert!(fixture.store.load_session(&scope, "inserted-before-error")?.is_none());

    let duplicate_todos = vec![original_todos[0].clone(), original_todos[0].clone()];
    fixture
        .store
        .save_session_todos(&scope, "kept-session", &duplicate_todos)
        .expect_err("duplicate todo ids must preserve the existing replacement");
    assert_json_eq!(fixture.store.load_session_todos(&scope, "kept-session")?, original_todos);
    Ok(())
}

// 夹具把提供方差异封装在统一接口后，避免 SQLite 与 PostgreSQL 的断言分叉。
struct StoreFixture {
    database: GlobalDatabase,
    project_store: ProjectStore,
    store: SessionStore,
    tenant_a: TenantScope,
    tenant_b: TenantScope,
}

impl StoreFixture {
    fn new(database: GlobalDatabase) -> Result<Self> {
        let project_store = ProjectStore::new(database.clone())?;
        let store = SessionStore::new(database.clone())?;
        Ok(Self {
            database,
            project_store,
            store,
            tenant_a: TenantScope::new("platform-a", "company-a")?,
            tenant_b: TenantScope::new("platform-b", "company-b")?,
        })
    }

    fn create_project(&self, tenant: &TenantScope, project_id: &str) -> Result<ProjectScope> {
        self.project_store.create(
            tenant,
            &ProjectInfo {
                id: project_id.to_owned(),
                worktree: format!("/work/{}/{project_id}", tenant.company_id().as_ref()),
                vcs: None,
                name: Some(project_id.to_owned()),
                icon: None,
                commands: None,
                time: ProjectTimeInfo { created: 1, updated: 2, initialized: None },
                sandboxes: Vec::new(),
            },
        )?;
        project_scope(tenant, project_id)
    }
}

fn for_each_session_store_provider(
    mut check: impl FnMut(&StoreFixture) -> Result<()>,
) -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_database = GlobalDatabase::new(sqlite_home.path().join("database/_index.sqlite3"))?;
    check(&StoreFixture::new(sqlite_database)?)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    check(&StoreFixture::new(postgres_fixture.database().clone())?)?;
    postgres_fixture.cleanup()?;
    Ok(())
}

fn corrupt_session_call(
    database: &GlobalDatabase,
    scope: &ProjectScope,
    session_id: &str,
) -> Result<()> {
    match database.provider() {
        DatabaseProvider::Sqlite => {
            database.connect_sqlite()?.execute(
                "UPDATE session_calls SET payload_json = 'not-json'
                 WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
                   AND session_id = ?4",
                params![
                    scope.platform_id().as_ref(),
                    scope.company_id().as_ref(),
                    scope.project_id().as_ref(),
                    session_id,
                ],
            )?;
        }
        DatabaseProvider::Postgres => {
            database.connect_postgres()?.execute(
                "UPDATE session_calls SET payload_json = 'not-json'
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND session_id = $4",
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                    &session_id,
                ],
            )?;
        }
    }
    Ok(())
}

fn project_scope(tenant: &TenantScope, project_id: &str) -> Result<ProjectScope> {
    Ok(ProjectScope::new(tenant.clone(), ProjectId::from(project_id))?)
}

fn sample_session(id: &str, title: &str, updated_ms: u64) -> ChatSession {
    ChatSession {
        id: id.to_owned(),
        title: title.to_owned(),
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "question".to_owned(),
                model: Some("provider/model-a".to_owned()),
                think_timing: vec![ThinkTiming {
                    start_ms: 10,
                    end_ms: Some(20),
                    last_update_ms: 18,
                }],
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "answer".to_owned(),
                model: Some("provider/model-a".to_owned()),
                think_timing: Vec::new(),
                processing_timing: Some(ProcessingTiming {
                    started_ms: 100,
                    completed_ms: Some(175),
                    processing_elapsed_ms: Some(90),
                    llm_elapsed_ms: Some(75),
                }),
            },
        ],
        message_ids: vec![Some("message-1".to_owned()), None],
        calls: vec![json!({"name": "shell", "args": {"cmd": "true"}})],
        steps: vec![ChatSessionStep {
            index: 7,
            turn_id: None,
            started_ms: 100,
            finished_ms: Some(130),
            start_snapshot_path: Some("start.json".to_owned()),
            finish_snapshot_path: Some("finish.json".to_owned()),
            usage: TokenUsage {
                input_tokens: 11,
                output_tokens: 12,
                cached_tokens: 3,
                reasoning_tokens: 4,
            },
            cost_usd: Some(0.25),
            finish_reason: Some("stop".to_owned()),
            model: Some("model-a".to_owned()),
            tool_timings: vec![ToolTiming {
                call_id: "call-1".to_owned(),
                name: "shell".to_owned(),
                started_ms: Some(105),
                finished_ms: Some(125),
            }],
        }],
        created_ms: 40,
        updated_ms,
    }
}

fn session_with_raw_rounds(session_id: &str) -> (ChatSession, Vec<serde_json::Value>) {
    let mut session = sample_session(session_id, "Raw rounds", 200);
    session.steps = vec![
        ChatSessionStep {
            index: 0,
            turn_id: Some("turn-raw".to_owned()),
            ..session.steps[0].clone()
        },
        ChatSessionStep {
            index: 1,
            turn_id: Some("turn-raw".to_owned()),
            ..session.steps[0].clone()
        },
    ];
    let first = json!({
        "session_id": session_id,
        "step_index": 0,
        "model": "provider/model-a",
        "system": "system prompt",
        "messages": [{"role": "user", "content": "question"}],
        "message_sources": [{"kind": "user"}],
        "request": {
            "config": {"temperature": 0.2},
            "tools": [{"name": "shell"}]
        },
        "output": {"content": "call tool"}
    });
    let second = json!({
        "session_id": session_id,
        "step_index": 1,
        "model": "provider/model-a",
        "system": "system prompt",
        "messages": [
            {"role": "user", "content": "question"},
            {"role": "assistant", "content": "call tool"},
            {"role": "tool", "content": "done"}
        ],
        "message_sources": [
            {"kind": "user"},
            {"kind": "assistant"},
            {"kind": "tool"}
        ],
        "request": {
            "config": {"temperature": 0.2},
            "tools": [{"name": "shell"}]
        },
        "output": {"content": "final"}
    });
    (session, vec![first, second])
}

fn sample_todos(label: &str) -> Vec<SessionTodoItem> {
    vec![
        SessionTodoItem {
            id: format!("{label}-todo-1"),
            content: "first".to_owned(),
            status: "in_progress".to_owned(),
            priority: "high".to_owned(),
        },
        SessionTodoItem {
            id: format!("{label}-todo-2"),
            content: "second".to_owned(),
            status: "pending".to_owned(),
            priority: "low".to_owned(),
        },
    ]
}

fn sample_agent_info(id: &str, project_id: &str, updated: u64) -> AgentSessionInfo {
    AgentSessionInfo {
        id: id.to_owned(),
        slug: format!("slug-{id}"),
        project_id: project_id.to_owned(),
        directory: format!("/work/{project_id}"),
        parent_id: Some("parent-session".to_owned()),
        summary: Some(Summary { additions: 4, deletions: 2, files: 1, diffs: None }),
        share: Some(ShareInfo { url: "https://example.test/session".to_owned() }),
        title: format!("Agent {id}"),
        version: "1".to_owned(),
        time: TimeInfo {
            created: updated.saturating_sub(1),
            updated,
            compacting: Some(updated.saturating_sub(2)),
            archived: Some(updated.saturating_add(1)),
        },
        permission: None,
        revert: Some(RevertInfo {
            message_id: "message-1".to_owned(),
            part_id: Some("part-1".to_owned()),
            snapshot: Some("snapshot-1".to_owned()),
            diff: Some("diff-1".to_owned()),
        }),
    }
}

fn sqlite_object_names(
    connection: &Connection,
    object_type: &str,
    pattern: &str,
) -> Result<Vec<String>> {
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = ?1 AND name LIKE ?2 ORDER BY name")?;
    Ok(statement
        .query_map(params![object_type, pattern], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

fn sqlite_object_exists(connection: &Connection, object_type: &str, name: &str) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = ?1 AND name = ?2",
        params![object_type, name],
        |row| row.get::<_, i64>(0),
    )? == 1)
}

fn assert_scope_columns(connection: &Connection, table: &str) -> Result<()> {
    for column in ["platform_id", "company_id", "project_id"] {
        let found: i64 = connection.query_row(
            &format!(
                "SELECT COUNT(*) FROM pragma_table_info('{table}')
                 WHERE name = ?1 AND \"notnull\" = 1"
            ),
            [column],
            |row| row.get(0),
        )?;
        assert_eq!(found, 1, "missing scoped column {table}.{column}");
    }
    Ok(())
}

fn assert_project_foreign_key(connection: &Connection, table: &str) -> Result<()> {
    let mappings = foreign_key_mappings(connection, table, "projects")?;
    assert_eq!(
        mappings,
        vec![
            ("company_id".to_owned(), "company_id".to_owned(), "RESTRICT".to_owned()),
            ("platform_id".to_owned(), "platform_id".to_owned(), "RESTRICT".to_owned()),
            ("project_id".to_owned(), "project_id".to_owned(), "RESTRICT".to_owned()),
        ],
        "incomplete projects foreign key for {table}"
    );
    Ok(())
}

fn assert_session_cascade_foreign_key(connection: &Connection, table: &str) -> Result<()> {
    let mappings = foreign_key_mappings(connection, table, "session_index")?;
    assert_eq!(
        mappings,
        vec![
            ("company_id".to_owned(), "company_id".to_owned(), "CASCADE".to_owned()),
            ("platform_id".to_owned(), "platform_id".to_owned(), "CASCADE".to_owned()),
            ("project_id".to_owned(), "project_id".to_owned(), "CASCADE".to_owned()),
            ("session_id".to_owned(), "session_id".to_owned(), "CASCADE".to_owned()),
        ],
        "incomplete session cascade foreign key for {table}"
    );
    Ok(())
}

fn foreign_key_mappings(
    connection: &Connection,
    table: &str,
    target_table: &str,
) -> Result<Vec<(String, String, String)>> {
    let mut statement = connection.prepare(&format!("PRAGMA foreign_key_list('{table}')"))?;
    let mut mappings = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(6)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .filter_map(|(target, from, to, on_delete)| {
            (target == target_table).then_some((from, to, on_delete))
        })
        .collect::<Vec<_>>();
    mappings.sort();
    Ok(mappings)
}

fn sqlite_table_columns(connection: &Connection, table: &str) -> Result<Vec<String>> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info('{table}')"))?;
    Ok(statement.query_map([], |row| row.get(1))?.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn scoped_row_count(database: &GlobalDatabase, table: &str, scope: &ProjectScope) -> Result<i64> {
    match database.provider() {
        DatabaseProvider::Sqlite => Ok(database.connect_sqlite()?.query_row(
            &format!(
                "SELECT COUNT(*) FROM {table}
                 WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3"
            ),
            params![
                scope.platform_id().as_ref(),
                scope.company_id().as_ref(),
                scope.project_id().as_ref(),
            ],
            |row| row.get(0),
        )?),
        DatabaseProvider::Postgres => Ok(database
            .connect_postgres()?
            .query_one(
                &format!(
                    "SELECT COUNT(*) FROM {table}
                     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3"
                ),
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                ],
            )?
            .try_get(0)?),
    }
}
