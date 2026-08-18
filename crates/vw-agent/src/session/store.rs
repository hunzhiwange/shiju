use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::session::info::Info as AgentSessionInfo;
use vw_shared::session::ui_types::{
    ChatMessage, ChatRole, ChatSession, ChatSessionMeta, ChatSessionStep, ProcessingTiming,
    SessionTodoItem, ThinkTiming, TokenUsage, ToolTiming,
};

use crate::global_database::config::DatabaseProvider;
use crate::global_database::{GlobalDatabase, run_sync_database};
use crate::session::llm_raw::{LlmRawArtifact, LlmRawRound, LlmRawTurnContext};

mod postgres;

use postgres::PostgresSessionStore;

const SESSION_SCHEMA: &str = r#"
BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS session_index (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
    title TEXT NOT NULL,
    created_ms INTEGER NOT NULL,
    updated_ms INTEGER NOT NULL,
    message_count INTEGER NOT NULL,
    call_count INTEGER NOT NULL,
    last_content TEXT,
    PRIMARY KEY (platform_id, company_id, project_id, session_id),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_session_index_project_updated
ON session_index(platform_id, company_id, project_id, updated_ms DESC, session_id DESC);
CREATE INDEX IF NOT EXISTS idx_session_index_tenant_updated
ON session_index(platform_id, company_id, updated_ms DESC, session_id DESC, project_id);

CREATE TABLE IF NOT EXISTS session_messages (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
    seq_no INTEGER NOT NULL,
    role TEXT NOT NULL,
    message_id TEXT,
    content TEXT NOT NULL,
    model TEXT,
    think_timing_json TEXT NOT NULL,
    processing_started_ms INTEGER,
    processing_completed_ms INTEGER,
    processing_elapsed_ms INTEGER,
    processing_llm_elapsed_ms INTEGER,
    PRIMARY KEY (platform_id, company_id, project_id, session_id, seq_no),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (platform_id, company_id, project_id, session_id)
        REFERENCES session_index(platform_id, company_id, project_id, session_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_messages_project_seq
ON session_messages(platform_id, company_id, project_id, session_id, seq_no);

CREATE TABLE IF NOT EXISTS session_calls (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
    seq_no INTEGER NOT NULL,
    payload_json TEXT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_id, session_id, seq_no),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (platform_id, company_id, project_id, session_id)
        REFERENCES session_index(platform_id, company_id, project_id, session_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_calls_project_seq
ON session_calls(platform_id, company_id, project_id, session_id, seq_no);

CREATE TABLE IF NOT EXISTS session_steps (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
    step_index INTEGER NOT NULL,
    turn_id TEXT NOT NULL DEFAULT '',
    started_ms INTEGER NOT NULL,
    finished_ms INTEGER,
    start_snapshot_path TEXT,
    finish_snapshot_path TEXT,
    usage_json TEXT NOT NULL,
    cost_usd REAL,
    finish_reason TEXT,
    model TEXT,
    tool_timings_json TEXT NOT NULL DEFAULT '[]',
    PRIMARY KEY (platform_id, company_id, project_id, session_id, turn_id, step_index),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (platform_id, company_id, project_id, session_id)
        REFERENCES session_index(platform_id, company_id, project_id, session_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_steps_project_index
ON session_steps(platform_id, company_id, project_id, session_id, step_index);

CREATE INDEX IF NOT EXISTS idx_session_steps_project_turn_index
ON session_steps(platform_id, company_id, project_id, session_id, turn_id, step_index);

CREATE TABLE IF NOT EXISTS session_todos (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
    order_no INTEGER NOT NULL,
    todo_id TEXT NOT NULL CHECK (trim(todo_id) <> ''),
    content TEXT NOT NULL,
    status TEXT NOT NULL,
    priority TEXT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_id, session_id, order_no),
    UNIQUE (platform_id, company_id, project_id, session_id, todo_id),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (platform_id, company_id, project_id, session_id)
        REFERENCES session_index(platform_id, company_id, project_id, session_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_todos_project_order
ON session_todos(platform_id, company_id, project_id, session_id, order_no, todo_id);

CREATE TABLE IF NOT EXISTS session_step_llm_raw (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
    step_index INTEGER NOT NULL,
    payload_json TEXT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_id, session_id, step_index),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (platform_id, company_id, project_id, session_id)
        REFERENCES session_index(platform_id, company_id, project_id, session_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_step_llm_raw_project_step
ON session_step_llm_raw(platform_id, company_id, project_id, session_id, step_index);

CREATE TABLE IF NOT EXISTS session_llm_raw_turns (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
    turn_id TEXT NOT NULL CHECK (trim(turn_id) <> ''),
    model_json TEXT NOT NULL,
    system_json TEXT NOT NULL,
    tools_json TEXT NOT NULL,
    initial_request_config_json TEXT NOT NULL,
    initial_provider_messages_json TEXT NOT NULL,
    initial_message_sources_json TEXT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_id, session_id, turn_id),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (platform_id, company_id, project_id, session_id)
        REFERENCES session_index(platform_id, company_id, project_id, session_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_llm_raw_turns_project_session
ON session_llm_raw_turns(platform_id, company_id, project_id, session_id, turn_id);

CREATE TABLE IF NOT EXISTS session_llm_raw_rounds (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
    turn_id TEXT NOT NULL CHECK (trim(turn_id) <> ''),
    step_index INTEGER NOT NULL,
    provider_message_prefix_len INTEGER NOT NULL,
    provider_message_suffix_json TEXT NOT NULL,
    message_source_suffix_json TEXT NOT NULL,
    system_delta_json TEXT,
    tools_delta_json TEXT,
    request_config_delta_json TEXT,
    output_json TEXT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_id, session_id, turn_id, step_index),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (platform_id, company_id, project_id, session_id, turn_id)
        REFERENCES session_llm_raw_turns(platform_id, company_id, project_id, session_id, turn_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_llm_raw_rounds_project_turn_step
ON session_llm_raw_rounds(platform_id, company_id, project_id, session_id, turn_id, step_index);

CREATE TABLE IF NOT EXISTS session_agent_sessions (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
    parent_id TEXT,
    updated_ms INTEGER NOT NULL,
    info_json TEXT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_id, session_id),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_session_agent_sessions_project_updated
ON session_agent_sessions(
    platform_id, company_id, project_id, updated_ms DESC, session_id DESC
);
CREATE INDEX IF NOT EXISTS idx_session_agent_sessions_tenant_updated
ON session_agent_sessions(
    platform_id, company_id, updated_ms DESC, session_id DESC, project_id
);
CREATE INDEX IF NOT EXISTS idx_session_agent_sessions_parent
ON session_agent_sessions(platform_id, company_id, project_id, parent_id, session_id);

COMMIT;
"#;

#[derive(Debug, Clone)]
pub struct SessionStore {
    provider: SessionStoreProvider,
}

/// 一个已完成 LLM 调用的稳定标识。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LlmRawStepKey {
    turn_id: String,
    step_index: u32,
}

impl LlmRawStepKey {
    pub fn new(turn_id: impl Into<String>, step_index: u32) -> Result<Self> {
        let turn_id = turn_id.into();
        validate_id("LLM raw turn id", &turn_id)?;
        Ok(Self { turn_id, step_index })
    }

    pub fn turn_id(&self) -> &str {
        &self.turn_id
    }

    pub fn step_index(&self) -> u32 {
        self.step_index
    }
}

#[derive(Debug, Clone)]
enum SessionStoreProvider {
    Sqlite(GlobalDatabase),
    Postgres(PostgresSessionStore),
}

impl SessionStore {
    pub fn new(database: GlobalDatabase) -> Result<Self> {
        let provider = match database.provider() {
            DatabaseProvider::Sqlite => {
                database.initialize_sqlite_schema(|connection| {
                    connection
                        .execute_batch(SESSION_SCHEMA)
                        .context("initialize SQLite session schema")?;
                    ensure_sqlite_message_model_column(connection)?;
                    ensure_sqlite_message_processing_timing_columns(connection)?;
                    ensure_sqlite_step_tool_timings_column(connection)?;
                    ensure_sqlite_step_turn_id_column(connection)?;
                    ensure_sqlite_llm_raw_round_context_delta_columns(connection)?;
                    Ok(())
                })?;
                SessionStoreProvider::Sqlite(database)
            }
            DatabaseProvider::Postgres => {
                SessionStoreProvider::Postgres(PostgresSessionStore::new(database)?)
            }
        };
        Ok(Self { provider })
    }

    pub fn storage_path(&self) -> Option<&Path> {
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => Some(database.path()),
            SessionStoreProvider::Postgres(_) => None,
        }
    }

    pub fn save_session(&self, scope: &ProjectScope, session: &ChatSession) -> Result<()> {
        validate_id("session id", &session.id)?;
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                database.with_sqlite_transaction(TransactionBehavior::Immediate, |transaction| {
                    upsert_session_header(transaction, scope, session, session.calls.len())?;
                    replace_session_messages(transaction, scope, session)?;
                    replace_session_calls(transaction, scope, session)?;
                    replace_session_steps(transaction, scope, session)?;
                    Ok(())
                })
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.save_session(scope, session))
            }
        }
    }

    pub fn save_session_ui_snapshot(
        &self,
        scope: &ProjectScope,
        session: &ChatSession,
    ) -> Result<()> {
        validate_id("session id", &session.id)?;
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                database.with_sqlite_transaction(TransactionBehavior::Immediate, |transaction| {
                    let call_count = existing_session_call_count(
                        transaction,
                        scope,
                        &session.id,
                        session.calls.len(),
                    )?;
                    upsert_session_header(transaction, scope, session, call_count)?;
                    replace_session_messages(transaction, scope, session)?;
                    replace_session_steps(transaction, scope, session)?;
                    Ok(())
                })
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.save_session_ui_snapshot(scope, session))
            }
        }
    }

    /// 保存 UI 可编辑的会话字段，保留由会话运行时独立持久化的调用与步骤。
    pub fn save_session_ui_content(
        &self,
        scope: &ProjectScope,
        session: &ChatSession,
    ) -> Result<()> {
        validate_id("session id", &session.id)?;
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                database.with_sqlite_transaction(TransactionBehavior::Immediate, |transaction| {
                    let call_count = existing_session_call_count(
                        transaction,
                        scope,
                        &session.id,
                        session.calls.len(),
                    )?;
                    upsert_session_header(transaction, scope, session, call_count)?;
                    replace_session_messages(transaction, scope, session)?;
                    Ok(())
                })
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.save_session_ui_content(scope, session))
            }
        }
    }

    /// 原子提交 UI 会话快照与本次已完成步骤对应的 LLM raw 工件。
    pub fn commit_session_ui_snapshot(
        &self,
        scope: &ProjectScope,
        session: &ChatSession,
        raw_artifacts: &[LlmRawArtifact],
        expected_finished_steps: &[LlmRawStepKey],
    ) -> Result<()> {
        validate_id("session id", &session.id)?;
        validate_raw_commit(session, raw_artifacts, expected_finished_steps)?;
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                database.with_sqlite_transaction(TransactionBehavior::Immediate, |transaction| {
                    let call_count = existing_session_call_count(
                        transaction,
                        scope,
                        &session.id,
                        session.calls.len(),
                    )?;
                    upsert_session_header(transaction, scope, session, call_count)?;
                    replace_session_messages(transaction, scope, session)?;
                    replace_session_steps(transaction, scope, session)?;
                    upsert_session_llm_raw_artifacts(transaction, scope, raw_artifacts)?;
                    Ok(())
                })
            }
            SessionStoreProvider::Postgres(store) => run_sync_database(|| {
                store.commit_session_ui_snapshot(scope, session, raw_artifacts)
            }),
        }
    }

    pub fn load_session(
        &self,
        scope: &ProjectScope,
        session_id: &str,
    ) -> Result<Option<ChatSession>> {
        validate_id("session id", session_id)?;
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                load_session_from_connection(&connection, scope, session_id)
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.load_session(scope, session_id))
            }
        }
    }

    pub fn load_session_any(
        &self,
        tenant: &TenantScope,
        session_id: &str,
    ) -> Result<Option<ChatSession>> {
        validate_id("session id", session_id)?;
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                let project_id = connection
                    .query_row(
                        "SELECT project_id FROM session_index
                         WHERE platform_id = ?1 AND company_id = ?2 AND session_id = ?3
                         ORDER BY updated_ms DESC, project_id, session_id DESC
                         LIMIT 1",
                        params![
                            tenant.platform_id().as_ref(),
                            tenant.company_id().as_ref(),
                            session_id,
                        ],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .context("find tenant SQLite session project")?;
                let Some(project_id) = project_id else {
                    return Ok(None);
                };
                let scope = ProjectScope::new(tenant.clone(), ProjectId::from(project_id))?;
                load_session_from_connection(&connection, &scope, session_id)
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.load_session_any(tenant, session_id))
            }
        }
    }

    pub fn list_sessions(&self, scope: &ProjectScope) -> Result<Vec<ChatSessionMeta>> {
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                let mut statement = connection.prepare(
                    "SELECT session_id, title, updated_ms, message_count, call_count, last_content
                     FROM session_index
                     WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
                     ORDER BY updated_ms DESC, session_id DESC
                     LIMIT 50",
                )?;
                let mut rows = statement.query(scope_params(scope))?;
                let mut sessions = Vec::new();
                while let Some(row) = rows.next()? {
                    sessions.push(ChatSessionMeta {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        updated_ms: from_i64(row.get(2)?, "session updated_ms")?,
                        message_count: from_i64_usize(row.get(3)?, "session message_count")?,
                        call_count: from_i64_usize(row.get(4)?, "session call_count")?,
                        last_content: row.get(5)?,
                    });
                }
                Ok(sessions)
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.list_sessions(scope))
            }
        }
    }

    pub fn load_session_meta(
        &self,
        scope: &ProjectScope,
        session_id: &str,
    ) -> Result<Option<ChatSessionMeta>> {
        validate_id("session id", session_id)?;
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                let row = connection
                    .query_row(
                        "SELECT title, updated_ms, message_count, call_count, last_content
                         FROM session_index
                         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
                           AND session_id = ?4",
                        scoped_id_params(scope, session_id),
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, i64>(1)?,
                                row.get::<_, i64>(2)?,
                                row.get::<_, i64>(3)?,
                                row.get::<_, Option<String>>(4)?,
                            ))
                        },
                    )
                    .optional()?;
                row.map(|(title, updated_ms, message_count, call_count, last_content)| {
                    Ok(ChatSessionMeta {
                        id: session_id.to_owned(),
                        title,
                        updated_ms: from_i64(updated_ms, "session updated_ms")?,
                        message_count: from_i64_usize(message_count, "session message_count")?,
                        call_count: from_i64_usize(call_count, "session call_count")?,
                        last_content,
                    })
                })
                .transpose()
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.load_session_meta(scope, session_id))
            }
        }
    }

    pub fn replace_session_index(
        &self,
        scope: &ProjectScope,
        sessions: &[ChatSessionMeta],
    ) -> Result<()> {
        let mut ids = HashSet::with_capacity(sessions.len());
        for session in sessions {
            validate_id("session id", &session.id)?;
            if !ids.insert(session.id.as_str()) {
                bail!("duplicate session id in index replacement: {}", session.id);
            }
        }
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                database.with_sqlite_transaction(TransactionBehavior::Immediate, |transaction| {
                    transaction.execute(
                        "DELETE FROM session_index
                         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3",
                        scope_params(scope),
                    )?;
                    for session in sessions {
                        transaction.execute(
                            "INSERT INTO session_index (
                                 platform_id, company_id, project_id, session_id, title, created_ms,
                                 updated_ms, message_count, call_count, last_content
                             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                            params![
                                scope.platform_id().as_ref(),
                                scope.company_id().as_ref(),
                                scope.project_id().as_ref(),
                                session.id,
                                session.title,
                                to_i64(session.updated_ms, "session updated_ms")?,
                                to_i64(session.updated_ms, "session updated_ms")?,
                                usize_to_i64(session.message_count, "session message_count")?,
                                usize_to_i64(session.call_count, "session call_count")?,
                                session.last_content,
                            ],
                        )?;
                    }
                    Ok(())
                })
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.replace_session_index(scope, sessions))
            }
        }
    }

    pub fn delete_session(&self, scope: &ProjectScope, session_id: &str) -> Result<bool> {
        validate_id("session id", session_id)?;
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                let changed = database.connect_sqlite()?.execute(
                    "DELETE FROM session_index
                     WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
                       AND session_id = ?4",
                    scoped_id_params(scope, session_id),
                )?;
                Ok(changed == 1)
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.delete_session(scope, session_id))
            }
        }
    }

    pub fn save_session_todos(
        &self,
        scope: &ProjectScope,
        session_id: &str,
        todos: &[SessionTodoItem],
    ) -> Result<()> {
        validate_id("session id", session_id)?;
        let mut ids = HashSet::with_capacity(todos.len());
        for todo in todos {
            validate_id("todo id", &todo.id)?;
            if !ids.insert(todo.id.as_str()) {
                bail!("duplicate todo id in session: {}", todo.id);
            }
        }
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                database.with_sqlite_transaction(TransactionBehavior::Immediate, |transaction| {
                    transaction.execute(
                        "DELETE FROM session_todos
                         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
                           AND session_id = ?4",
                        scoped_id_params(scope, session_id),
                    )?;
                    for (order_no, todo) in todos.iter().enumerate() {
                        transaction.execute(
                            "INSERT INTO session_todos (
                                 platform_id, company_id, project_id, session_id, order_no,
                                 todo_id, content, status, priority
                             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                            params![
                                scope.platform_id().as_ref(),
                                scope.company_id().as_ref(),
                                scope.project_id().as_ref(),
                                session_id,
                                usize_to_i64(order_no, "todo order_no")?,
                                todo.id,
                                todo.content,
                                todo.status,
                                todo.priority,
                            ],
                        )?;
                    }
                    Ok(())
                })
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.save_session_todos(scope, session_id, todos))
            }
        }
    }

    pub fn save_session_step_llm_raw(
        &self,
        scope: &ProjectScope,
        session_id: &str,
        step_index: u32,
        payload: &serde_json::Value,
    ) -> Result<()> {
        validate_id("session id", session_id)?;
        let payload_json =
            serde_json::to_string(payload).context("serialize LLM raw session step payload")?;
        let step_index_sqlite = to_i64(step_index as u64, "step index")?;

        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                database.connect_sqlite()?.execute(
                    "INSERT INTO session_step_llm_raw (
                        platform_id, company_id, project_id, session_id, step_index, payload_json
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                    ON CONFLICT (platform_id, company_id, project_id, session_id, step_index)
                        DO UPDATE SET payload_json = excluded.payload_json",
                    params![
                        scope.platform_id().as_ref(),
                        scope.company_id().as_ref(),
                        scope.project_id().as_ref(),
                        session_id,
                        step_index_sqlite,
                        payload_json,
                    ],
                )?;
                Ok(())
            }
            SessionStoreProvider::Postgres(store) => run_sync_database(|| {
                store.save_session_step_llm_raw(scope, session_id, step_index, payload)
            }),
        }
    }

    /// 以稳定的 turn_id 和 step_index 重建一条 LLM 原始记录。
    pub fn load_session_turn_step_llm_raw(
        &self,
        scope: &ProjectScope,
        session_id: &str,
        turn_id: &str,
        step_index: u32,
    ) -> Result<Option<serde_json::Value>> {
        validate_id("session id", session_id)?;
        validate_id("LLM raw turn id", turn_id)?;
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                load_sqlite_session_turn_step_llm_raw(
                    &connection, scope, session_id, turn_id, step_index,
                )
            }
            SessionStoreProvider::Postgres(store) => run_sync_database(|| {
                store.load_session_turn_step_llm_raw(scope, session_id, turn_id, step_index)
            }),
        }
    }

    pub fn load_session_step_llm_raw(
        &self,
        scope: &ProjectScope,
        session_id: &str,
        step_index: u32,
    ) -> Result<Option<serde_json::Value>> {
        validate_id("session id", session_id)?;

        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                let turn_id = sqlite_session_step_turn_id(&connection, scope, session_id, step_index)?;
                match turn_id {
                    Some(turn_id) => load_sqlite_session_turn_step_llm_raw(
                        &connection, scope, session_id, &turn_id, step_index,
                    ),
                    None => load_sqlite_legacy_session_step_llm_raw(
                        &connection, scope, session_id, step_index,
                    ),
                }
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.load_session_step_llm_raw(scope, session_id, step_index))
            }
        }
    }

    pub fn load_session_llm_raw_step_indices(
        &self,
        scope: &ProjectScope,
        session_id: &str,
    ) -> Result<Vec<u32>> {
        validate_id("session id", session_id)?;

        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                let mut statement = connection.prepare(
                    "SELECT step_index FROM session_llm_raw_rounds
                     WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
                       AND session_id = ?4
                     UNION
                     SELECT step_index FROM session_step_llm_raw
                     WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
                       AND session_id = ?4
                     ORDER BY step_index",
                )?;
                statement
                    .query_map(
                        params![
                            scope.platform_id().as_ref(),
                            scope.company_id().as_ref(),
                            scope.project_id().as_ref(),
                            session_id,
                        ],
                        |row| row.get::<_, i64>(0),
                    )?
                    .map(|step_index| {
                        let step_index = step_index?;
                        u32::try_from(step_index).context("convert LLM raw step index to u32")
                    })
                    .collect()
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.load_session_llm_raw_step_indices(scope, session_id))
            }
        }
    }

    pub fn load_session_todos(
        &self,
        scope: &ProjectScope,
        session_id: &str,
    ) -> Result<Vec<SessionTodoItem>> {
        validate_id("session id", session_id)?;
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                let mut statement = connection.prepare(
                    "SELECT todo_id, content, status, priority
                     FROM session_todos
                     WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
                       AND session_id = ?4
                     ORDER BY order_no, todo_id",
                )?;
                let todos = statement
                    .query_map(scoped_id_params(scope, session_id), |row| {
                        Ok(SessionTodoItem {
                            id: row.get(0)?,
                            content: row.get(1)?,
                            status: row.get(2)?,
                            priority: row.get(3)?,
                        })
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(todos)
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.load_session_todos(scope, session_id))
            }
        }
    }

    pub fn save_agent_session(&self, scope: &ProjectScope, info: &AgentSessionInfo) -> Result<()> {
        validate_id("agent session id", &info.id)?;
        if info.project_id != scope.project_id().as_ref() {
            bail!("agent session project id must match project scope");
        }
        let info_json = serde_json::to_string(info).context("serialize agent session info JSON")?;
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                database.connect_sqlite()?.execute(
                    "INSERT INTO session_agent_sessions (
                         platform_id, company_id, project_id, session_id, parent_id, updated_ms,
                         info_json
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT (platform_id, company_id, project_id, session_id) DO UPDATE SET
                         parent_id = excluded.parent_id,
                         updated_ms = excluded.updated_ms,
                         info_json = excluded.info_json",
                    params![
                        scope.platform_id().as_ref(),
                        scope.company_id().as_ref(),
                        scope.project_id().as_ref(),
                        info.id,
                        info.parent_id,
                        to_i64(info.time.updated, "agent session updated_ms")?,
                        info_json,
                    ],
                )?;
                Ok(())
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.save_agent_session(scope, info, &info_json))
            }
        }
    }

    pub fn load_agent_session(
        &self,
        scope: &ProjectScope,
        session_id: &str,
    ) -> Result<Option<AgentSessionInfo>> {
        validate_id("agent session id", session_id)?;
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                let info_json = database
                    .connect_sqlite()?
                    .query_row(
                        "SELECT info_json FROM session_agent_sessions
                         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
                           AND session_id = ?4",
                        scoped_id_params(scope, session_id),
                        |row| row.get::<_, String>(0),
                    )
                    .optional()?;
                info_json
                    .map(|json| parse_agent_info(&json, scope.project_id().as_ref(), session_id))
                    .transpose()
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.load_agent_session(scope, session_id))
            }
        }
    }

    pub fn load_agent_sessions(&self, scope: &ProjectScope) -> Result<Vec<AgentSessionInfo>> {
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                let mut statement = connection.prepare(
                    "SELECT session_id, info_json FROM session_agent_sessions
                     WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
                     ORDER BY updated_ms DESC, session_id DESC",
                )?;
                let mut rows = statement.query(scope_params(scope))?;
                let mut sessions = Vec::new();
                while let Some(row) = rows.next()? {
                    let session_id: String = row.get(0)?;
                    let info_json: String = row.get(1)?;
                    sessions.push(parse_agent_info(
                        &info_json,
                        scope.project_id().as_ref(),
                        &session_id,
                    )?);
                }
                Ok(sessions)
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.load_agent_sessions(scope))
            }
        }
    }

    pub fn load_agent_sessions_all(&self, tenant: &TenantScope) -> Result<Vec<AgentSessionInfo>> {
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                let mut statement = connection.prepare(
                    "SELECT project_id, session_id, info_json FROM session_agent_sessions
                     WHERE platform_id = ?1 AND company_id = ?2
                     ORDER BY updated_ms DESC, session_id DESC, project_id",
                )?;
                let mut rows = statement
                    .query(params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(),])?;
                let mut seen = HashSet::new();
                let mut sessions = Vec::new();
                while let Some(row) = rows.next()? {
                    let project_id: String = row.get(0)?;
                    let session_id: String = row.get(1)?;
                    let info_json: String = row.get(2)?;
                    if seen.insert(session_id.clone()) {
                        sessions.push(parse_agent_info(&info_json, &project_id, &session_id)?);
                    }
                }
                Ok(sessions)
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.load_agent_sessions_all(tenant))
            }
        }
    }

    pub fn load_agent_session_any(
        &self,
        tenant: &TenantScope,
        session_id: &str,
    ) -> Result<Option<AgentSessionInfo>> {
        validate_id("agent session id", session_id)?;
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                let row = database
                    .connect_sqlite()?
                    .query_row(
                        "SELECT project_id, info_json FROM session_agent_sessions
                         WHERE platform_id = ?1 AND company_id = ?2 AND session_id = ?3
                         ORDER BY updated_ms DESC, project_id
                         LIMIT 1",
                        params![
                            tenant.platform_id().as_ref(),
                            tenant.company_id().as_ref(),
                            session_id,
                        ],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                    )
                    .optional()?;
                row.map(|(project_id, info_json)| {
                    parse_agent_info(&info_json, &project_id, session_id)
                })
                .transpose()
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.load_agent_session_any(tenant, session_id))
            }
        }
    }

    pub fn delete_agent_session(&self, scope: &ProjectScope, session_id: &str) -> Result<bool> {
        validate_id("agent session id", session_id)?;
        match &self.provider {
            SessionStoreProvider::Sqlite(database) => {
                let changed = database.connect_sqlite()?.execute(
                    "DELETE FROM session_agent_sessions
                     WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
                       AND session_id = ?4",
                    scoped_id_params(scope, session_id),
                )?;
                Ok(changed == 1)
            }
            SessionStoreProvider::Postgres(store) => {
                run_sync_database(|| store.delete_agent_session(scope, session_id))
            }
        }
    }
}

fn sqlite_session_step_turn_id(
    connection: &Connection,
    scope: &ProjectScope,
    session_id: &str,
    step_index: u32,
) -> Result<Option<String>> {
    let turn_id = connection
        .query_row(
            "SELECT turn_id FROM session_steps
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
               AND session_id = ?4 AND step_index = ?5",
            params![
                scope.platform_id().as_ref(),
                scope.company_id().as_ref(),
                scope.project_id().as_ref(),
                session_id,
                i64::from(step_index),
            ],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?;
    Ok(turn_id.flatten().filter(|turn_id| !turn_id.is_empty()))
}

fn load_sqlite_legacy_session_step_llm_raw(
    connection: &Connection,
    scope: &ProjectScope,
    session_id: &str,
    step_index: u32,
) -> Result<Option<serde_json::Value>> {
    let payload_json = connection
        .query_row(
            "SELECT payload_json
             FROM session_step_llm_raw
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
               AND session_id = ?4 AND step_index = ?5",
            params![
                scope.platform_id().as_ref(),
                scope.company_id().as_ref(),
                scope.project_id().as_ref(),
                session_id,
                i64::from(step_index),
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    payload_json
        .map(|payload_json| {
            serde_json::from_str::<serde_json::Value>(&payload_json).with_context(|| {
                format!("parse session llm raw step payload JSON: {session_id}/{step_index}")
            })
        })
        .transpose()
}

fn load_sqlite_session_turn_step_llm_raw(
    connection: &Connection,
    scope: &ProjectScope,
    session_id: &str,
    turn_id: &str,
    step_index: u32,
) -> Result<Option<serde_json::Value>> {
    let stored_context = connection
        .query_row(
            "SELECT model_json, system_json, tools_json, initial_request_config_json,
                    initial_provider_messages_json, initial_message_sources_json
             FROM session_llm_raw_turns
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
               AND session_id = ?4 AND turn_id = ?5",
            params![
                scope.platform_id().as_ref(),
                scope.company_id().as_ref(),
                scope.project_id().as_ref(),
                session_id,
                turn_id,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?;
    let Some((model, system, tools, initial_request_config, initial_provider_messages, initial_message_sources)) = stored_context else {
        return Ok(None);
    };
    let context = LlmRawTurnContext {
        turn_id: turn_id.to_owned(),
        session_id: session_id.to_owned(),
        model: parse_raw_json(model, "turn model")?,
        system: parse_raw_json(system, "turn system")?,
        tools: parse_raw_json(tools, "turn tools")?,
        initial_request_config: parse_raw_json(initial_request_config, "turn initial request config")?,
        initial_provider_messages: parse_raw_json(
            initial_provider_messages,
            "turn initial provider messages",
        )?,
        initial_message_sources: parse_raw_json(initial_message_sources, "turn initial message sources")?,
    };
    let mut statement = connection.prepare(
        "SELECT step_index, provider_message_prefix_len, provider_message_suffix_json,
                message_source_suffix_json, system_delta_json, tools_delta_json,
                request_config_delta_json, output_json
         FROM session_llm_raw_rounds
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
           AND session_id = ?4 AND turn_id = ?5 AND step_index <= ?6
         ORDER BY step_index",
    )?;
    let rounds = statement
        .query_map(
            params![
                scope.platform_id().as_ref(),
                scope.company_id().as_ref(),
                scope.project_id().as_ref(),
                session_id,
                turn_id,
                i64::from(step_index),
            ],
            |row| sqlite_raw_round(row, turn_id),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if rounds.last().map(|round| round.step_index) != Some(step_index) {
        return Ok(None);
    }
    Ok(Some(LlmRawArtifact { context, rounds }.reconstruct_round(step_index)?))
}

fn sqlite_raw_round(row: &rusqlite::Row<'_>, turn_id: &str) -> rusqlite::Result<LlmRawRound> {
    let parse_value = |json: String, field: &str| -> rusqlite::Result<serde_json::Value> {
        serde_json::from_str(&json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("parse LLM raw {field}: {error}"),
                )),
            )
        })
    };
    let parse_values = |json: String, field: &str| -> rusqlite::Result<Vec<serde_json::Value>> {
        serde_json::from_str(&json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("parse LLM raw {field}: {error}"),
                )),
            )
        })
    };
    let prefix_len: i64 = row.get(1)?;
    Ok(LlmRawRound {
        turn_id: turn_id.to_owned(),
        step_index: u32::try_from(row.get::<_, i64>(0)?).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Integer, Box::new(error))
        })?,
        provider_message_prefix_len: usize::try_from(prefix_len).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Integer, Box::new(error))
        })?,
        provider_message_suffix: parse_values(row.get(2)?, "provider message suffix")?,
        message_source_suffix: parse_values(row.get(3)?, "message source suffix")?,
        system_delta: row
            .get::<_, Option<String>>(4)?
            .map(|json| parse_value(json, "system delta"))
            .transpose()?,
        tools_delta: row
            .get::<_, Option<String>>(5)?
            .map(|json| parse_value(json, "tools delta"))
            .transpose()?,
        request_config_delta: row
            .get::<_, Option<String>>(6)?
            .map(|json| parse_value(json, "request config delta"))
            .transpose()?,
        output: parse_value(row.get(7)?, "output")?,
    })
}

fn parse_raw_json<T: serde::de::DeserializeOwned>(json: String, field: &str) -> Result<T> {
    serde_json::from_str(&json).with_context(|| format!("parse stored LLM raw {field} JSON"))
}

fn upsert_session_header(
    transaction: &Transaction<'_>,
    scope: &ProjectScope,
    session: &ChatSession,
    call_count: usize,
) -> Result<()> {
    transaction.execute(
        "INSERT INTO session_index (
             platform_id, company_id, project_id, session_id, title, created_ms, updated_ms,
             message_count, call_count, last_content
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT (platform_id, company_id, project_id, session_id) DO UPDATE SET
             title = excluded.title,
             created_ms = excluded.created_ms,
             updated_ms = excluded.updated_ms,
             message_count = excluded.message_count,
             call_count = excluded.call_count,
             last_content = excluded.last_content",
        params![
            scope.platform_id().as_ref(),
            scope.company_id().as_ref(),
            scope.project_id().as_ref(),
            session.id,
            session.title,
            to_i64(session.created_ms, "session created_ms")?,
            to_i64(session.updated_ms, "session updated_ms")?,
            usize_to_i64(session.messages.len(), "session message_count")?,
            usize_to_i64(call_count, "session call_count")?,
            session_last_content(&session.messages),
        ],
    )?;
    Ok(())
}

fn existing_session_call_count(
    transaction: &Transaction<'_>,
    scope: &ProjectScope,
    session_id: &str,
    fallback: usize,
) -> Result<usize> {
    let call_count = transaction
        .query_row(
            "SELECT call_count FROM session_index
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND session_id = ?4",
            scoped_id_params(scope, session_id),
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    call_count.map_or(Ok(fallback), |value| from_i64_usize(value, "session call_count"))
}

fn replace_session_messages(
    transaction: &Transaction<'_>,
    scope: &ProjectScope,
    session: &ChatSession,
) -> Result<()> {
    transaction.execute(
        "DELETE FROM session_messages
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND session_id = ?4",
        scoped_id_params(scope, &session.id),
    )?;
    for (seq_no, message) in session.messages.iter().enumerate() {
        let think_timing_json =
            serde_json::to_string(&message.think_timing).context("serialize think timing JSON")?;
        let processing_started_ms = message
            .processing_timing
            .as_ref()
            .map(|timing| to_i64(timing.started_ms, "message processing started_ms"))
            .transpose()?;
        let processing_completed_ms = message
            .processing_timing
            .as_ref()
            .and_then(|timing| timing.completed_ms)
            .map(|value| to_i64(value, "message processing completed_ms"))
            .transpose()?;
        let processing_llm_elapsed_ms = message
            .processing_timing
            .as_ref()
            .and_then(|timing| timing.llm_elapsed_ms)
            .map(|value| to_i64(value, "message processing llm_elapsed_ms"))
            .transpose()?;
        let processing_elapsed_ms = message
            .processing_timing
            .as_ref()
            .and_then(|timing| timing.processing_elapsed_ms)
            .map(|value| to_i64(value, "message processing elapsed_ms"))
            .transpose()?;
        transaction.execute(
            "INSERT INTO session_messages (
                 platform_id, company_id, project_id, session_id, seq_no, role, message_id,
                 content, model, think_timing_json, processing_started_ms, processing_completed_ms,
                 processing_elapsed_ms, processing_llm_elapsed_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                scope.platform_id().as_ref(),
                scope.company_id().as_ref(),
                scope.project_id().as_ref(),
                session.id,
                usize_to_i64(seq_no, "message seq_no")?,
                role_to_db(message.role),
                session.message_ids.get(seq_no).cloned().flatten(),
                message.content,
                message.model,
                think_timing_json,
                processing_started_ms,
                processing_completed_ms,
                processing_elapsed_ms,
                processing_llm_elapsed_ms,
            ],
        )?;
    }
    Ok(())
}

fn replace_session_calls(
    transaction: &Transaction<'_>,
    scope: &ProjectScope,
    session: &ChatSession,
) -> Result<()> {
    transaction.execute(
        "DELETE FROM session_calls
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND session_id = ?4",
        scoped_id_params(scope, &session.id),
    )?;
    for (seq_no, payload) in session.calls.iter().enumerate() {
        let payload_json = serde_json::to_string(payload).context("serialize session call JSON")?;
        transaction.execute(
            "INSERT INTO session_calls (
                 platform_id, company_id, project_id, session_id, seq_no, payload_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                scope.platform_id().as_ref(),
                scope.company_id().as_ref(),
                scope.project_id().as_ref(),
                session.id,
                usize_to_i64(seq_no, "call seq_no")?,
                payload_json,
            ],
        )?;
    }
    Ok(())
}

fn replace_session_steps(
    transaction: &Transaction<'_>,
    scope: &ProjectScope,
    session: &ChatSession,
) -> Result<()> {
    transaction.execute(
        "DELETE FROM session_steps
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND session_id = ?4",
        scoped_id_params(scope, &session.id),
    )?;
    for step in &session.steps {
        let usage_json =
            serde_json::to_string(&step.usage).context("serialize token usage JSON")?;
        let tool_timings_json = serde_json::to_string(&step.tool_timings)
            .context("serialize step tool timings JSON")?;
        transaction.execute(
            "INSERT INTO session_steps (
                 platform_id, company_id, project_id, session_id, step_index, turn_id, started_ms,
                 finished_ms, start_snapshot_path, finish_snapshot_path, usage_json, cost_usd,
                 finish_reason, model, tool_timings_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                scope.platform_id().as_ref(),
                scope.company_id().as_ref(),
                scope.project_id().as_ref(),
                session.id,
                i64::from(step.index),
                step.turn_id.as_deref().unwrap_or(""),
                to_i64(step.started_ms, "step started_ms")?,
                step.finished_ms.map(|value| to_i64(value, "step finished_ms")).transpose()?,
                step.start_snapshot_path,
                step.finish_snapshot_path,
                usage_json,
                step.cost_usd,
                step.finish_reason,
                step.model,
                tool_timings_json,
            ],
        )?;
    }
    Ok(())
}

fn validate_raw_commit(
    session: &ChatSession,
    raw_artifacts: &[LlmRawArtifact],
    expected_finished_steps: &[LlmRawStepKey],
) -> Result<()> {
    let mut expected = HashSet::with_capacity(expected_finished_steps.len());
    for key in expected_finished_steps {
        validate_id("LLM raw turn id", key.turn_id())?;
        if !expected.insert((key.turn_id(), key.step_index())) {
            bail!(
                "duplicate expected LLM raw finished step: {}/{}",
                key.turn_id(),
                key.step_index()
            );
        }
        let has_finished_step = session.steps.iter().any(|step| {
            step.finished_ms.is_some()
                && step.index == key.step_index()
                && step.turn_id.as_deref() == Some(key.turn_id())
        });
        if !has_finished_step {
            bail!(
                "expected LLM raw finished step is absent from session snapshot: {}/{}",
                key.turn_id(),
                key.step_index()
            );
        }
    }

    let mut actual = HashSet::new();
    for artifact in raw_artifacts {
        validate_id("LLM raw turn id", &artifact.context.turn_id)?;
        if artifact.context.session_id != session.id {
            bail!("LLM raw artifact session_id does not match the UI snapshot");
        }
        for round in &artifact.rounds {
            if round.turn_id != artifact.context.turn_id {
                bail!("LLM raw round turn_id does not match its context");
            }
            if !actual.insert((round.turn_id.as_str(), round.step_index)) {
                bail!(
                    "duplicate LLM raw artifact round: {}/{}",
                    round.turn_id,
                    round.step_index
                );
            }
        }
    }

    for key in &expected {
        if !actual.contains(key) {
            bail!("missing LLM raw artifact for finished step: {}/{}", key.0, key.1);
        }
    }
    for key in &actual {
        if !expected.contains(key) {
            bail!("LLM raw artifact has no expected finished step: {}/{}", key.0, key.1);
        }
    }
    Ok(())
}

fn upsert_session_llm_raw_artifacts(
    transaction: &Transaction<'_>,
    scope: &ProjectScope,
    raw_artifacts: &[LlmRawArtifact],
) -> Result<()> {
    for artifact in raw_artifacts {
        upsert_session_llm_raw_turn(transaction, scope, &artifact.context)?;
        for round in &artifact.rounds {
            upsert_session_llm_raw_round(transaction, scope, &artifact.context.session_id, round)?;
        }
    }
    Ok(())
}

fn upsert_session_llm_raw_turn(
    transaction: &Transaction<'_>,
    scope: &ProjectScope,
    context: &LlmRawTurnContext,
) -> Result<()> {
    transaction.execute(
        "INSERT INTO session_llm_raw_turns (
             platform_id, company_id, project_id, session_id, turn_id, model_json, system_json,
             tools_json, initial_request_config_json, initial_provider_messages_json,
             initial_message_sources_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT (platform_id, company_id, project_id, session_id, turn_id) DO UPDATE SET
             model_json = excluded.model_json,
             system_json = excluded.system_json,
             tools_json = excluded.tools_json,
             initial_request_config_json = excluded.initial_request_config_json,
             initial_provider_messages_json = excluded.initial_provider_messages_json,
             initial_message_sources_json = excluded.initial_message_sources_json",
        params![
            scope.platform_id().as_ref(),
            scope.company_id().as_ref(),
            scope.project_id().as_ref(),
            context.session_id,
            context.turn_id,
            serde_json::to_string(&context.model)?,
            serde_json::to_string(&context.system)?,
            serde_json::to_string(&context.tools)?,
            serde_json::to_string(&context.initial_request_config)?,
            serde_json::to_string(&context.initial_provider_messages)?,
            serde_json::to_string(&context.initial_message_sources)?,
        ],
    )?;
    Ok(())
}

fn upsert_session_llm_raw_round(
    transaction: &Transaction<'_>,
    scope: &ProjectScope,
    session_id: &str,
    round: &LlmRawRound,
) -> Result<()> {
    transaction.execute(
        "INSERT INTO session_llm_raw_rounds (
             platform_id, company_id, project_id, session_id, turn_id, step_index,
             provider_message_prefix_len, provider_message_suffix_json, message_source_suffix_json,
             system_delta_json, tools_delta_json, request_config_delta_json, output_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT (platform_id, company_id, project_id, session_id, turn_id, step_index)
             DO UPDATE SET
                 provider_message_prefix_len = excluded.provider_message_prefix_len,
                 provider_message_suffix_json = excluded.provider_message_suffix_json,
                 message_source_suffix_json = excluded.message_source_suffix_json,
                 system_delta_json = excluded.system_delta_json,
                 tools_delta_json = excluded.tools_delta_json,
                 request_config_delta_json = excluded.request_config_delta_json,
                 output_json = excluded.output_json",
        params![
            scope.platform_id().as_ref(),
            scope.company_id().as_ref(),
            scope.project_id().as_ref(),
            session_id,
            round.turn_id,
            i64::from(round.step_index),
            usize_to_i64(round.provider_message_prefix_len, "LLM raw provider message prefix length")?,
            serde_json::to_string(&round.provider_message_suffix)?,
            serde_json::to_string(&round.message_source_suffix)?,
            round.system_delta.as_ref().map(serde_json::to_string).transpose()?,
            round.tools_delta.as_ref().map(serde_json::to_string).transpose()?,
            round.request_config_delta.as_ref().map(serde_json::to_string).transpose()?,
            serde_json::to_string(&round.output)?,
        ],
    )?;
    Ok(())
}

fn load_session_from_connection(
    connection: &Connection,
    scope: &ProjectScope,
    session_id: &str,
) -> Result<Option<ChatSession>> {
    let header = connection
        .query_row(
            "SELECT title, created_ms, updated_ms FROM session_index
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND session_id = ?4",
            scoped_id_params(scope, session_id),
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
        )
        .optional()?;
    let Some((title, created_ms, updated_ms)) = header else {
        return Ok(None);
    };

    let (messages, message_ids) = load_messages(connection, scope, session_id)?;
    let calls = load_calls(connection, scope, session_id)?;
    let steps = load_steps(connection, scope, session_id)?;
    Ok(Some(ChatSession {
        id: session_id.to_owned(),
        title,
        messages,
        message_ids,
        calls,
        steps,
        created_ms: from_i64(created_ms, "session created_ms")?,
        updated_ms: from_i64(updated_ms, "session updated_ms")?,
    }))
}

fn load_messages(
    connection: &Connection,
    scope: &ProjectScope,
    session_id: &str,
) -> Result<(Vec<ChatMessage>, Vec<Option<String>>)> {
    let mut statement = connection.prepare(
        "SELECT role, message_id, content, think_timing_json, model,
                processing_started_ms, processing_completed_ms, processing_elapsed_ms,
                processing_llm_elapsed_ms
         FROM session_messages
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND session_id = ?4
         ORDER BY seq_no",
    )?;
    let mut rows = statement.query(scoped_id_params(scope, session_id))?;
    let mut messages = Vec::new();
    let mut message_ids = Vec::new();
    while let Some(row) = rows.next()? {
        let role: String = row.get(0)?;
        let think_timing_json: String = row.get(3)?;
        let processing_started_ms: Option<i64> = row.get(5)?;
        let processing_completed_ms: Option<i64> = row.get(6)?;
        let processing_elapsed_ms: Option<i64> = row.get(7)?;
        let processing_llm_elapsed_ms: Option<i64> = row.get(8)?;
        let processing_timing = match (processing_started_ms, processing_completed_ms) {
            (None, None) => None,
            (Some(started_ms), completed_ms) => Some(ProcessingTiming {
                started_ms: from_i64(started_ms, "message processing started_ms")?,
                completed_ms: completed_ms
                    .map(|value| from_i64(value, "message processing completed_ms"))
                    .transpose()?,
                processing_elapsed_ms: processing_elapsed_ms
                    .map(|value| from_i64(value, "message processing elapsed_ms"))
                    .transpose()?,
                llm_elapsed_ms: processing_llm_elapsed_ms
                    .map(|value| from_i64(value, "message processing llm_elapsed_ms"))
                    .transpose()?,
            }),
            (None, Some(_)) => bail!(
                "session message processing completed_ms exists without started_ms: {session_id}"
            ),
        };
        message_ids.push(row.get(1)?);
        messages.push(ChatMessage {
            role: role_from_db(&role)?,
            content: row.get(2)?,
            model: row.get(4)?,
            think_timing: serde_json::from_str::<Vec<ThinkTiming>>(&think_timing_json)
                .with_context(|| {
                    format!("parse session message think timing JSON: {session_id}")
                })?,
            processing_timing,
        });
    }
    Ok((messages, message_ids))
}

fn ensure_sqlite_message_model_column(connection: &Connection) -> Result<()> {
    let mut statement = connection.prepare("PRAGMA table_info(session_messages)")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if row.get::<_, String>(1)? == "model" {
            return Ok(());
        }
    }

    connection
        .execute("ALTER TABLE session_messages ADD COLUMN model TEXT", [])
        .context("add SQLite session message model column")?;
    Ok(())
}

fn ensure_sqlite_message_processing_timing_columns(connection: &Connection) -> Result<()> {
    let columns = {
        let mut statement = connection.prepare("PRAGMA table_info(session_messages)")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
        rows.collect::<rusqlite::Result<HashSet<_>>>()?
    };
    if !columns.contains("processing_started_ms") {
        connection
            .execute("ALTER TABLE session_messages ADD COLUMN processing_started_ms INTEGER", [])
            .context("add SQLite session message processing started_ms column")?;
    }
    if !columns.contains("processing_completed_ms") {
        connection
            .execute("ALTER TABLE session_messages ADD COLUMN processing_completed_ms INTEGER", [])
            .context("add SQLite session message processing completed_ms column")?;
    }
    if !columns.contains("processing_elapsed_ms") {
        connection
            .execute("ALTER TABLE session_messages ADD COLUMN processing_elapsed_ms INTEGER", [])
            .context("add SQLite session message processing elapsed_ms column")?;
    }
    if !columns.contains("processing_llm_elapsed_ms") {
        connection
            .execute(
                "ALTER TABLE session_messages ADD COLUMN processing_llm_elapsed_ms INTEGER",
                [],
            )
            .context("add SQLite session message processing llm_elapsed_ms column")?;
    }
    Ok(())
}

fn ensure_sqlite_step_tool_timings_column(connection: &Connection) -> Result<()> {
    let mut statement = connection.prepare("PRAGMA table_info(session_steps)")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    if rows.collect::<rusqlite::Result<HashSet<_>>>()?.contains("tool_timings_json") {
        return Ok(());
    }
    connection
        .execute(
            "ALTER TABLE session_steps ADD COLUMN tool_timings_json TEXT NOT NULL DEFAULT '[]'",
            [],
        )
        .context("add SQLite session step tool timings column")?;
    Ok(())
}

fn ensure_sqlite_llm_raw_round_context_delta_columns(connection: &Connection) -> Result<()> {
    let mut statement = connection.prepare("PRAGMA table_info(session_llm_raw_rounds)")?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<HashSet<_>>>()?;
    if !columns.contains("system_delta_json") {
        connection
            .execute("ALTER TABLE session_llm_raw_rounds ADD COLUMN system_delta_json TEXT", [])
            .context("add SQLite LLM raw round system delta column")?;
    }
    if !columns.contains("tools_delta_json") {
        connection
            .execute("ALTER TABLE session_llm_raw_rounds ADD COLUMN tools_delta_json TEXT", [])
            .context("add SQLite LLM raw round tools delta column")?;
    }
    Ok(())
}

fn ensure_sqlite_step_turn_id_column(connection: &Connection) -> Result<()> {
    let mut statement = connection.prepare("PRAGMA table_info(session_steps)")?;
    let columns = statement
        .query_map([], |row| Ok((row.get::<_, String>(1)?, row.get::<_, i64>(5)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if !columns.iter().any(|(name, _)| name == "turn_id") {
        connection
            .execute(
                "ALTER TABLE session_steps ADD COLUMN turn_id TEXT NOT NULL DEFAULT ''",
                [],
            )
            .context("add SQLite session step turn_id column")?;
    }

    if columns
        .iter()
        .any(|(name, primary_key_order)| name == "turn_id" && *primary_key_order > 0)
    {
        return Ok(());
    }

    connection
        .execute_batch(
            "BEGIN IMMEDIATE;
             ALTER TABLE session_steps RENAME TO session_steps_legacy;
             CREATE TABLE session_steps (
                 platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
                 company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
                 project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
                 session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
                 step_index INTEGER NOT NULL,
                 turn_id TEXT NOT NULL DEFAULT '',
                 started_ms INTEGER NOT NULL,
                 finished_ms INTEGER,
                 start_snapshot_path TEXT,
                 finish_snapshot_path TEXT,
                 usage_json TEXT NOT NULL,
                 cost_usd REAL,
                 finish_reason TEXT,
                 model TEXT,
                 tool_timings_json TEXT NOT NULL DEFAULT '[]',
                 PRIMARY KEY (platform_id, company_id, project_id, session_id, turn_id, step_index),
                 FOREIGN KEY (platform_id, company_id, project_id)
                     REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
                 FOREIGN KEY (platform_id, company_id, project_id, session_id)
                     REFERENCES session_index(platform_id, company_id, project_id, session_id)
                     ON DELETE CASCADE
             );
             INSERT INTO session_steps (
                 platform_id, company_id, project_id, session_id, step_index, turn_id, started_ms,
                 finished_ms, start_snapshot_path, finish_snapshot_path, usage_json, cost_usd,
                 finish_reason, model, tool_timings_json
             )
             SELECT
                 platform_id, company_id, project_id, session_id, step_index, COALESCE(turn_id, ''),
                 started_ms, finished_ms, start_snapshot_path, finish_snapshot_path, usage_json,
                 cost_usd, finish_reason, model, tool_timings_json
             FROM session_steps_legacy;
             DROP TABLE session_steps_legacy;
             CREATE INDEX idx_session_steps_project_index
                 ON session_steps(platform_id, company_id, project_id, session_id, step_index);
             CREATE INDEX idx_session_steps_project_turn_index
                 ON session_steps(platform_id, company_id, project_id, session_id, turn_id, step_index);
             COMMIT;",
        )
        .context("migrate SQLite session step key to turn identity")?;
    Ok(())
}

fn load_calls(
    connection: &Connection,
    scope: &ProjectScope,
    session_id: &str,
) -> Result<Vec<serde_json::Value>> {
    let mut statement = connection.prepare(
        "SELECT payload_json FROM session_calls
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND session_id = ?4
         ORDER BY seq_no",
    )?;
    let mut rows = statement.query(scoped_id_params(scope, session_id))?;
    let mut calls = Vec::new();
    while let Some(row) = rows.next()? {
        let payload_json: String = row.get(0)?;
        calls.push(
            serde_json::from_str(&payload_json)
                .with_context(|| format!("parse session call payload JSON: {session_id}"))?,
        );
    }
    Ok(calls)
}

fn load_steps(
    connection: &Connection,
    scope: &ProjectScope,
    session_id: &str,
) -> Result<Vec<ChatSessionStep>> {
    let mut statement = connection.prepare(
        "SELECT step_index, turn_id, started_ms, finished_ms, start_snapshot_path, finish_snapshot_path,
                usage_json, cost_usd, finish_reason, model, tool_timings_json
         FROM session_steps
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND session_id = ?4
         ORDER BY step_index",
    )?;
    let mut rows = statement.query(scoped_id_params(scope, session_id))?;
    let mut steps = Vec::new();
    while let Some(row) = rows.next()? {
        let usage_json: String = row.get(6)?;
        let tool_timings_json: String = row.get(10)?;
        let finished_ms: Option<i64> = row.get(3)?;
        let turn_id: String = row.get(1)?;
        steps.push(ChatSessionStep {
            index: u32::try_from(row.get::<_, i64>(0)?).context("read session step_index")?,
            turn_id: (!turn_id.is_empty()).then_some(turn_id),
            started_ms: from_i64(row.get(2)?, "step started_ms")?,
            finished_ms: finished_ms
                .map(|value| from_i64(value, "step finished_ms"))
                .transpose()?,
            start_snapshot_path: row.get(4)?,
            finish_snapshot_path: row.get(5)?,
            usage: serde_json::from_str::<TokenUsage>(&usage_json)
                .with_context(|| format!("parse session step usage JSON: {session_id}"))?,
            cost_usd: row.get(7)?,
            finish_reason: row.get(8)?,
            model: row.get(9)?,
            tool_timings: serde_json::from_str::<Vec<ToolTiming>>(&tool_timings_json)
                .with_context(|| format!("parse session step tool timings JSON: {session_id}"))?,
        });
    }
    Ok(steps)
}

fn parse_agent_info(
    info_json: &str,
    expected_project_id: &str,
    session_id: &str,
) -> Result<AgentSessionInfo> {
    let info = serde_json::from_str::<AgentSessionInfo>(info_json)
        .with_context(|| format!("parse agent session info JSON: {session_id}"))?;
    if info.id != session_id {
        bail!("agent session JSON id does not match stored session id: {session_id}");
    }
    if info.project_id != expected_project_id {
        bail!("agent session JSON project id does not match stored project scope: {session_id}");
    }
    Ok(info)
}

fn session_last_content(messages: &[ChatMessage]) -> Option<String> {
    messages
        .iter()
        .rev()
        .find_map(|message| (!message.content.trim().is_empty()).then(|| message.content.clone()))
}

fn role_to_db(role: ChatRole) -> &'static str {
    match role {
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::System => "system",
        ChatRole::Tool => "tool",
    }
}

fn role_from_db(role: &str) -> Result<ChatRole> {
    match role {
        "user" => Ok(ChatRole::User),
        "assistant" => Ok(ChatRole::Assistant),
        "system" => Ok(ChatRole::System),
        "tool" => Ok(ChatRole::Tool),
        _ => bail!("invalid stored session message role: {role}"),
    }
}

fn validate_id(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{field} must not be empty");
    }
    Ok(())
}

fn to_i64(value: u64, field: &str) -> Result<i64> {
    i64::try_from(value).with_context(|| format!("{field} exceeds database BIGINT range"))
}

fn usize_to_i64(value: usize, field: &str) -> Result<i64> {
    i64::try_from(value).with_context(|| format!("{field} exceeds database BIGINT range"))
}

fn from_i64(value: i64, field: &str) -> Result<u64> {
    u64::try_from(value).with_context(|| format!("{field} is negative"))
}

fn from_i64_usize(value: i64, field: &str) -> Result<usize> {
    usize::try_from(value).with_context(|| format!("{field} is negative or too large"))
}

fn scope_params(scope: &ProjectScope) -> (&str, &str, &str) {
    (scope.platform_id().as_ref(), scope.company_id().as_ref(), scope.project_id().as_ref())
}

fn scoped_id_params<'a>(
    scope: &'a ProjectScope,
    id: &'a str,
) -> (&'a str, &'a str, &'a str, &'a str) {
    (scope.platform_id().as_ref(), scope.company_id().as_ref(), scope.project_id().as_ref(), id)
}
