use std::collections::HashSet;

use ::postgres::{GenericClient, Row};
use anyhow::{Context, Result};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::session::info::Info as AgentSessionInfo;
use vw_shared::session::ui_types::{
    ChatMessage, ChatSession, ChatSessionMeta, ChatSessionStep, ProcessingTiming, SessionTodoItem,
    ThinkTiming, TokenUsage,
};

use super::{
    from_i64, from_i64_usize, parse_agent_info, role_from_db, role_to_db, session_last_content,
    to_i64, usize_to_i64,
};
use crate::global_database::GlobalDatabase;

macro_rules! scope_params {
    ($scope:expr) => {
        &[
            &$scope.platform_id().as_ref(),
            &$scope.company_id().as_ref(),
            &$scope.project_id().as_ref(),
        ]
    };
}

macro_rules! scoped_id_params {
    ($scope:expr, $id:expr) => {
        &[
            &$scope.platform_id().as_ref(),
            &$scope.company_id().as_ref(),
            &$scope.project_id().as_ref(),
            &$id,
        ]
    };
}

const SESSION_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS session_index (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
    title TEXT NOT NULL,
    created_ms BIGINT NOT NULL,
    updated_ms BIGINT NOT NULL,
    message_count BIGINT NOT NULL,
    call_count BIGINT NOT NULL,
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
    seq_no BIGINT NOT NULL,
    role TEXT NOT NULL,
    message_id TEXT,
    content TEXT NOT NULL,
    model TEXT,
    think_timing_json TEXT NOT NULL,
    processing_started_ms BIGINT,
    processing_completed_ms BIGINT,
    processing_elapsed_ms BIGINT,
    processing_llm_elapsed_ms BIGINT,
    PRIMARY KEY (platform_id, company_id, project_id, session_id, seq_no),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (platform_id, company_id, project_id, session_id)
        REFERENCES session_index(platform_id, company_id, project_id, session_id)
        ON DELETE CASCADE
);

ALTER TABLE session_messages ADD COLUMN IF NOT EXISTS model TEXT;
ALTER TABLE session_messages ADD COLUMN IF NOT EXISTS processing_started_ms BIGINT;
ALTER TABLE session_messages ADD COLUMN IF NOT EXISTS processing_completed_ms BIGINT;
ALTER TABLE session_messages ADD COLUMN IF NOT EXISTS processing_elapsed_ms BIGINT;
ALTER TABLE session_messages ADD COLUMN IF NOT EXISTS processing_llm_elapsed_ms BIGINT;

CREATE INDEX IF NOT EXISTS idx_session_messages_project_seq
ON session_messages(platform_id, company_id, project_id, session_id, seq_no);

CREATE TABLE IF NOT EXISTS session_calls (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
    seq_no BIGINT NOT NULL,
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
    step_index BIGINT NOT NULL,
    started_ms BIGINT NOT NULL,
    finished_ms BIGINT,
    start_snapshot_path TEXT,
    finish_snapshot_path TEXT,
    usage_json TEXT NOT NULL,
    cost_usd DOUBLE PRECISION,
    finish_reason TEXT,
    model TEXT,
    PRIMARY KEY (platform_id, company_id, project_id, session_id, step_index),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (platform_id, company_id, project_id, session_id)
        REFERENCES session_index(platform_id, company_id, project_id, session_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_steps_project_index
ON session_steps(platform_id, company_id, project_id, session_id, step_index);

CREATE TABLE IF NOT EXISTS session_todos (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
    order_no BIGINT NOT NULL,
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
    step_index BIGINT NOT NULL,
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

CREATE TABLE IF NOT EXISTS session_agent_sessions (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
    parent_id TEXT,
    updated_ms BIGINT NOT NULL,
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
"#;

#[derive(Debug, Clone)]
pub(super) struct PostgresSessionStore {
    database: GlobalDatabase,
}

impl PostgresSessionStore {
    pub(super) fn new(database: GlobalDatabase) -> Result<Self> {
        database.initialize_postgres_schema(|transaction| {
            transaction
                .batch_execute(SESSION_SCHEMA)
                .context("initialize PostgreSQL session schema")?;
            Ok(())
        })?;
        Ok(Self { database })
    }

    pub(super) fn save_session(&self, scope: &ProjectScope, session: &ChatSession) -> Result<()> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction = connection.transaction().context("begin PostgreSQL session save")?;
        upsert_session_header(&mut transaction, scope, session, session.calls.len())?;
        replace_session_messages(&mut transaction, scope, session)?;
        replace_session_calls(&mut transaction, scope, session)?;
        replace_session_steps(&mut transaction, scope, session)?;
        transaction.commit().context("commit PostgreSQL session save")?;
        Ok(())
    }

    pub(super) fn save_session_ui_snapshot(
        &self,
        scope: &ProjectScope,
        session: &ChatSession,
    ) -> Result<()> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction = connection
            .transaction()
            .context("begin PostgreSQL session UI snapshot save")?;
        let call_count = transaction
            .query_opt(
                "SELECT call_count FROM session_index
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND session_id = $4",
                scoped_id_params!(scope, &session.id),
            )?
            .map(|row| from_i64_usize(row.get(0), "session call_count"))
            .transpose()?
            .unwrap_or(session.calls.len());
        upsert_session_header(&mut transaction, scope, session, call_count)?;
        replace_session_messages(&mut transaction, scope, session)?;
        replace_session_steps(&mut transaction, scope, session)?;
        transaction
            .commit()
            .context("commit PostgreSQL session UI snapshot save")?;
        Ok(())
    }

    pub(super) fn load_session(
        &self,
        scope: &ProjectScope,
        session_id: &str,
    ) -> Result<Option<ChatSession>> {
        let mut connection = self.database.connect_postgres()?;
        load_session(&mut *connection, scope, session_id)
    }

    pub(super) fn load_session_any(
        &self,
        tenant: &TenantScope,
        session_id: &str,
    ) -> Result<Option<ChatSession>> {
        let mut connection = self.database.connect_postgres()?;
        let row = connection
            .query_opt(
                "SELECT project_id FROM session_index
                 WHERE platform_id = $1 AND company_id = $2 AND session_id = $3
                 ORDER BY updated_ms DESC, project_id, session_id DESC
                 LIMIT 1",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &session_id],
            )
            .context("find tenant PostgreSQL session project")?;
        let Some(row) = row else {
            return Ok(None);
        };
        let project_id: String = row.try_get(0).context("read PostgreSQL session project id")?;
        let scope = ProjectScope::new(tenant.clone(), ProjectId::from(project_id))?;
        load_session(&mut *connection, &scope, session_id)
    }

    pub(super) fn list_sessions(&self, scope: &ProjectScope) -> Result<Vec<ChatSessionMeta>> {
        let mut connection = self.database.connect_postgres()?;
        connection
            .query(
                "SELECT session_id, title, updated_ms, message_count, call_count, last_content
                 FROM session_index
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                 ORDER BY updated_ms DESC, session_id DESC
                 LIMIT 50",
                scope_params!(scope),
            )
            .context("list scoped PostgreSQL sessions")?
            .iter()
            .map(map_session_meta)
            .collect()
    }

    pub(super) fn load_session_meta(
        &self,
        scope: &ProjectScope,
        session_id: &str,
    ) -> Result<Option<ChatSessionMeta>> {
        let mut connection = self.database.connect_postgres()?;
        connection
            .query_opt(
                "SELECT session_id, title, updated_ms, message_count, call_count, last_content
                 FROM session_index
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND session_id = $4",
                scoped_id_params!(scope, session_id),
            )
            .context("load scoped PostgreSQL session metadata")?
            .as_ref()
            .map(map_session_meta)
            .transpose()
    }

    pub(super) fn replace_session_index(
        &self,
        scope: &ProjectScope,
        sessions: &[ChatSessionMeta],
    ) -> Result<()> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL session index replacement")?;
        transaction
            .execute(
                "DELETE FROM session_index
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3",
                scope_params!(scope),
            )
            .context("delete scoped PostgreSQL session index")?;
        for session in sessions {
            let updated_ms = to_i64(session.updated_ms, "session updated_ms")?;
            let message_count = usize_to_i64(session.message_count, "session message_count")?;
            let call_count = usize_to_i64(session.call_count, "session call_count")?;
            transaction
                .execute(
                    "INSERT INTO session_index (
                         platform_id, company_id, project_id, session_id, title, created_ms,
                         updated_ms, message_count, call_count, last_content
                     ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                    &[
                        &scope.platform_id().as_ref(),
                        &scope.company_id().as_ref(),
                        &scope.project_id().as_ref(),
                        &session.id,
                        &session.title,
                        &updated_ms,
                        &updated_ms,
                        &message_count,
                        &call_count,
                        &session.last_content,
                    ],
                )
                .context("insert scoped PostgreSQL session index row")?;
        }
        transaction.commit().context("commit PostgreSQL session index replacement")?;
        Ok(())
    }

    pub(super) fn delete_session(&self, scope: &ProjectScope, session_id: &str) -> Result<bool> {
        let changed = self
            .database
            .connect_postgres()?
            .execute(
                "DELETE FROM session_index
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND session_id = $4",
                scoped_id_params!(scope, session_id),
            )
            .context("delete scoped PostgreSQL session")?;
        Ok(changed == 1)
    }

    pub(super) fn save_session_todos(
        &self,
        scope: &ProjectScope,
        session_id: &str,
        todos: &[SessionTodoItem],
    ) -> Result<()> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL session todo replacement")?;
        transaction
            .execute(
                "DELETE FROM session_todos
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND session_id = $4",
                scoped_id_params!(scope, session_id),
            )
            .context("delete scoped PostgreSQL session todos")?;
        for (order_no, todo) in todos.iter().enumerate() {
            let order_no = usize_to_i64(order_no, "todo order_no")?;
            transaction
                .execute(
                    "INSERT INTO session_todos (
                         platform_id, company_id, project_id, session_id, order_no,
                         todo_id, content, status, priority
                     ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                    &[
                        &scope.platform_id().as_ref(),
                        &scope.company_id().as_ref(),
                        &scope.project_id().as_ref(),
                        &session_id,
                        &order_no,
                        &todo.id,
                        &todo.content,
                        &todo.status,
                        &todo.priority,
                    ],
                )
                .context("insert scoped PostgreSQL session todo")?;
        }
        transaction.commit().context("commit PostgreSQL session todo replacement")?;
        Ok(())
    }

    pub(super) fn save_session_step_llm_raw(
        &self,
        scope: &ProjectScope,
        session_id: &str,
        step_index: u32,
        payload: &serde_json::Value,
    ) -> Result<()> {
        let step_index = to_i64(step_index as u64, "step index")?;
        let payload_json = serde_json::to_string(payload)
            .context("serialize PostgreSQL LLM raw session step payload")?;
        self.database
            .connect_postgres()?
            .execute(
                "INSERT INTO session_step_llm_raw (
                     platform_id, company_id, project_id, session_id, step_index, payload_json
                 ) VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (platform_id, company_id, project_id, session_id, step_index)
                     DO UPDATE SET payload_json = EXCLUDED.payload_json",
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                    &session_id,
                    &step_index,
                    &payload_json,
                ],
            )
            .context("save PostgreSQL session llm raw step payload")?;
        Ok(())
    }

    pub(super) fn load_session_step_llm_raw(
        &self,
        scope: &ProjectScope,
        session_id: &str,
        step_index: u32,
    ) -> Result<Option<serde_json::Value>> {
        let step_index = to_i64(step_index as u64, "step index")?;
        let row = self
            .database
            .connect_postgres()?
            .query_opt(
                "SELECT payload_json FROM session_step_llm_raw
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND session_id = $4 AND step_index = $5",
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                    &session_id,
                    &step_index,
                ],
            )
            .context("load PostgreSQL session llm raw step payload")?;
        row.map(|row| {
            let payload_json: String = row.try_get(0)?;
            serde_json::from_str(&payload_json).with_context(|| {
                format!("parse session llm raw step payload JSON: {session_id}/{step_index}")
            })
        })
        .transpose()
    }

    pub(super) fn load_session_todos(
        &self,
        scope: &ProjectScope,
        session_id: &str,
    ) -> Result<Vec<SessionTodoItem>> {
        let mut connection = self.database.connect_postgres()?;
        connection
            .query(
                "SELECT todo_id, content, status, priority
                 FROM session_todos
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND session_id = $4
                 ORDER BY order_no, todo_id",
                scoped_id_params!(scope, session_id),
            )
            .context("load scoped PostgreSQL session todos")?
            .iter()
            .map(|row| {
                Ok(SessionTodoItem {
                    id: row.try_get(0)?,
                    content: row.try_get(1)?,
                    status: row.try_get(2)?,
                    priority: row.try_get(3)?,
                })
            })
            .collect()
    }

    pub(super) fn save_agent_session(
        &self,
        scope: &ProjectScope,
        info: &AgentSessionInfo,
        info_json: &str,
    ) -> Result<()> {
        let updated_ms = to_i64(info.time.updated, "agent session updated_ms")?;
        self.database
            .connect_postgres()?
            .execute(
                "INSERT INTO session_agent_sessions (
                     platform_id, company_id, project_id, session_id, parent_id, updated_ms,
                     info_json
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (platform_id, company_id, project_id, session_id) DO UPDATE SET
                     parent_id = excluded.parent_id,
                     updated_ms = excluded.updated_ms,
                     info_json = excluded.info_json",
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                    &info.id,
                    &info.parent_id,
                    &updated_ms,
                    &info_json,
                ],
            )
            .context("save scoped PostgreSQL agent session")?;
        Ok(())
    }

    pub(super) fn load_agent_session(
        &self,
        scope: &ProjectScope,
        session_id: &str,
    ) -> Result<Option<AgentSessionInfo>> {
        let row = self
            .database
            .connect_postgres()?
            .query_opt(
                "SELECT info_json FROM session_agent_sessions
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND session_id = $4",
                scoped_id_params!(scope, session_id),
            )
            .context("load scoped PostgreSQL agent session")?;
        row.map(|row| {
            let info_json: String = row.try_get(0)?;
            parse_agent_info(&info_json, scope.project_id().as_ref(), session_id)
        })
        .transpose()
    }

    pub(super) fn load_agent_sessions(
        &self,
        scope: &ProjectScope,
    ) -> Result<Vec<AgentSessionInfo>> {
        let mut connection = self.database.connect_postgres()?;
        connection
            .query(
                "SELECT session_id, info_json FROM session_agent_sessions
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                 ORDER BY updated_ms DESC, session_id DESC",
                scope_params!(scope),
            )
            .context("list scoped PostgreSQL agent sessions")?
            .iter()
            .map(|row| parse_agent_row(row, scope.project_id().as_ref()))
            .collect()
    }

    pub(super) fn load_agent_sessions_all(
        &self,
        tenant: &TenantScope,
    ) -> Result<Vec<AgentSessionInfo>> {
        let mut connection = self.database.connect_postgres()?;
        let rows = connection
            .query(
                "SELECT project_id, session_id, info_json FROM session_agent_sessions
                 WHERE platform_id = $1 AND company_id = $2
                 ORDER BY updated_ms DESC, session_id DESC, project_id",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref()],
            )
            .context("list tenant PostgreSQL agent sessions")?;
        let mut seen = HashSet::new();
        let mut sessions = Vec::new();
        for row in rows {
            let project_id: String = row.try_get(0)?;
            let session_id: String = row.try_get(1)?;
            if seen.insert(session_id.clone()) {
                let info_json: String = row.try_get(2)?;
                sessions.push(parse_agent_info(&info_json, &project_id, &session_id)?);
            }
        }
        Ok(sessions)
    }

    pub(super) fn load_agent_session_any(
        &self,
        tenant: &TenantScope,
        session_id: &str,
    ) -> Result<Option<AgentSessionInfo>> {
        let row = self
            .database
            .connect_postgres()?
            .query_opt(
                "SELECT project_id, info_json FROM session_agent_sessions
                 WHERE platform_id = $1 AND company_id = $2 AND session_id = $3
                 ORDER BY updated_ms DESC, project_id
                 LIMIT 1",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &session_id],
            )
            .context("load tenant PostgreSQL agent session")?;
        row.map(|row| {
            let project_id: String = row.try_get(0)?;
            let info_json: String = row.try_get(1)?;
            parse_agent_info(&info_json, &project_id, session_id)
        })
        .transpose()
    }

    pub(super) fn delete_agent_session(
        &self,
        scope: &ProjectScope,
        session_id: &str,
    ) -> Result<bool> {
        let changed = self
            .database
            .connect_postgres()?
            .execute(
                "DELETE FROM session_agent_sessions
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND session_id = $4",
                scoped_id_params!(scope, session_id),
            )
            .context("delete scoped PostgreSQL agent session")?;
        Ok(changed == 1)
    }
}

fn upsert_session_header(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    session: &ChatSession,
    call_count: usize,
) -> Result<()> {
    let created_ms = to_i64(session.created_ms, "session created_ms")?;
    let updated_ms = to_i64(session.updated_ms, "session updated_ms")?;
    let message_count = usize_to_i64(session.messages.len(), "session message_count")?;
    let call_count = usize_to_i64(call_count, "session call_count")?;
    let last_content = session_last_content(&session.messages);
    client
        .execute(
            "INSERT INTO session_index (
                 platform_id, company_id, project_id, session_id, title, created_ms, updated_ms,
                 message_count, call_count, last_content
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (platform_id, company_id, project_id, session_id) DO UPDATE SET
                 title = excluded.title,
                 created_ms = excluded.created_ms,
                 updated_ms = excluded.updated_ms,
                 message_count = excluded.message_count,
                 call_count = excluded.call_count,
                 last_content = excluded.last_content",
            &[
                &scope.platform_id().as_ref(),
                &scope.company_id().as_ref(),
                &scope.project_id().as_ref(),
                &session.id,
                &session.title,
                &created_ms,
                &updated_ms,
                &message_count,
                &call_count,
                &last_content,
            ],
        )
        .context("upsert scoped PostgreSQL session header")?;
    Ok(())
}

fn replace_session_messages(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    session: &ChatSession,
) -> Result<()> {
    delete_session_children(client, "session_messages", scope, &session.id)?;
    for (sequence, message) in session.messages.iter().enumerate() {
        let message_id = session.message_ids.get(sequence).cloned().flatten();
        let seq_no = usize_to_i64(sequence, "message seq_no")?;
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
        client
            .execute(
                "INSERT INTO session_messages (
                     platform_id, company_id, project_id, session_id, seq_no, role, message_id,
                     content, model, think_timing_json, processing_started_ms,
                     processing_completed_ms, processing_elapsed_ms, processing_llm_elapsed_ms
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                    &session.id,
                    &seq_no,
                    &role_to_db(message.role),
                    &message_id,
                    &message.content,
                    &message.model,
                    &think_timing_json,
                    &processing_started_ms,
                    &processing_completed_ms,
                    &processing_elapsed_ms,
                    &processing_llm_elapsed_ms,
                ],
            )
            .context("insert scoped PostgreSQL session message")?;
    }
    Ok(())
}

fn replace_session_calls(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    session: &ChatSession,
) -> Result<()> {
    delete_session_children(client, "session_calls", scope, &session.id)?;
    for (seq_no, payload) in session.calls.iter().enumerate() {
        let seq_no = usize_to_i64(seq_no, "call seq_no")?;
        let payload_json = serde_json::to_string(payload).context("serialize session call JSON")?;
        client
            .execute(
                "INSERT INTO session_calls (
                     platform_id, company_id, project_id, session_id, seq_no, payload_json
                 ) VALUES ($1, $2, $3, $4, $5, $6)",
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                    &session.id,
                    &seq_no,
                    &payload_json,
                ],
            )
            .context("insert scoped PostgreSQL session call")?;
    }
    Ok(())
}

fn replace_session_steps(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    session: &ChatSession,
) -> Result<()> {
    delete_session_children(client, "session_steps", scope, &session.id)?;
    for step in &session.steps {
        let step_index = i64::from(step.index);
        let started_ms = to_i64(step.started_ms, "step started_ms")?;
        let finished_ms =
            step.finished_ms.map(|value| to_i64(value, "step finished_ms")).transpose()?;
        let usage_json =
            serde_json::to_string(&step.usage).context("serialize token usage JSON")?;
        client
            .execute(
                "INSERT INTO session_steps (
                     platform_id, company_id, project_id, session_id, step_index, started_ms,
                     finished_ms, start_snapshot_path, finish_snapshot_path, usage_json, cost_usd,
                     finish_reason, model
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                    &session.id,
                    &step_index,
                    &started_ms,
                    &finished_ms,
                    &step.start_snapshot_path,
                    &step.finish_snapshot_path,
                    &usage_json,
                    &step.cost_usd,
                    &step.finish_reason,
                    &step.model,
                ],
            )
            .context("insert scoped PostgreSQL session step")?;
    }
    Ok(())
}

fn delete_session_children(
    client: &mut impl GenericClient,
    table: &str,
    scope: &ProjectScope,
    session_id: &str,
) -> Result<()> {
    client
        .execute(
            &format!(
                "DELETE FROM {table}
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND session_id = $4"
            ),
            scoped_id_params!(scope, session_id),
        )
        .with_context(|| format!("delete scoped PostgreSQL {table} rows"))?;
    Ok(())
}

fn load_session(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    session_id: &str,
) -> Result<Option<ChatSession>> {
    let header = client
        .query_opt(
            "SELECT title, created_ms, updated_ms FROM session_index
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND session_id = $4",
            scoped_id_params!(scope, session_id),
        )
        .context("load scoped PostgreSQL session header")?;
    let Some(header) = header else {
        return Ok(None);
    };
    let title: String = header.try_get(0)?;
    let created_ms: i64 = header.try_get(1)?;
    let updated_ms: i64 = header.try_get(2)?;
    let (messages, message_ids) = load_messages(client, scope, session_id)?;
    let calls = load_calls(client, scope, session_id)?;
    let steps = load_steps(client, scope, session_id)?;
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
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    session_id: &str,
) -> Result<(Vec<ChatMessage>, Vec<Option<String>>)> {
    let rows = client
        .query(
            "SELECT role, message_id, content, think_timing_json, model,
                    processing_started_ms, processing_completed_ms, processing_elapsed_ms,
                    processing_llm_elapsed_ms
             FROM session_messages
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND session_id = $4
             ORDER BY seq_no",
            scoped_id_params!(scope, session_id),
        )
        .context("load scoped PostgreSQL session messages")?;
    let mut messages = Vec::with_capacity(rows.len());
    let mut message_ids = Vec::with_capacity(rows.len());
    for row in rows {
        let role: String = row.try_get(0)?;
        let think_timing_json: String = row.try_get(3)?;
        let processing_started_ms: Option<i64> = row.try_get(5)?;
        let processing_completed_ms: Option<i64> = row.try_get(6)?;
        let processing_elapsed_ms: Option<i64> = row.try_get(7)?;
        let processing_llm_elapsed_ms: Option<i64> = row.try_get(8)?;
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
            (None, Some(_)) => anyhow::bail!(
                "session message processing completed_ms exists without started_ms: {session_id}"
            ),
        };
        message_ids.push(row.try_get(1)?);
        messages.push(ChatMessage {
            role: role_from_db(&role)?,
            content: row.try_get(2)?,
            model: row.try_get(4)?,
            think_timing: serde_json::from_str::<Vec<ThinkTiming>>(&think_timing_json)
                .with_context(|| {
                    format!("parse session message think timing JSON: {session_id}")
                })?,
            processing_timing,
        });
    }
    Ok((messages, message_ids))
}

fn load_calls(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    session_id: &str,
) -> Result<Vec<serde_json::Value>> {
    client
        .query(
            "SELECT payload_json FROM session_calls
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND session_id = $4
             ORDER BY seq_no",
            scoped_id_params!(scope, session_id),
        )
        .context("load scoped PostgreSQL session calls")?
        .iter()
        .map(|row| {
            let payload_json: String = row.try_get(0)?;
            serde_json::from_str(&payload_json)
                .with_context(|| format!("parse session call payload JSON: {session_id}"))
        })
        .collect()
}

fn load_steps(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    session_id: &str,
) -> Result<Vec<ChatSessionStep>> {
    client
        .query(
            "SELECT step_index, started_ms, finished_ms, start_snapshot_path, finish_snapshot_path,
                    usage_json, cost_usd, finish_reason, model
             FROM session_steps
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND session_id = $4
             ORDER BY step_index",
            scoped_id_params!(scope, session_id),
        )
        .context("load scoped PostgreSQL session steps")?
        .iter()
        .map(|row| map_session_step(row, session_id))
        .collect()
}

fn map_session_meta(row: &Row) -> Result<ChatSessionMeta> {
    Ok(ChatSessionMeta {
        id: row.try_get(0)?,
        title: row.try_get(1)?,
        updated_ms: from_i64(row.try_get(2)?, "session updated_ms")?,
        message_count: from_i64_usize(row.try_get(3)?, "session message_count")?,
        call_count: from_i64_usize(row.try_get(4)?, "session call_count")?,
        last_content: row.try_get(5)?,
    })
}

fn map_session_step(row: &Row, session_id: &str) -> Result<ChatSessionStep> {
    let finished_ms: Option<i64> = row.try_get(2)?;
    let usage_json: String = row.try_get(5)?;
    Ok(ChatSessionStep {
        index: u32::try_from(row.try_get::<_, i64>(0)?).context("read session step_index")?,
        started_ms: from_i64(row.try_get(1)?, "step started_ms")?,
        finished_ms: finished_ms.map(|value| from_i64(value, "step finished_ms")).transpose()?,
        start_snapshot_path: row.try_get(3)?,
        finish_snapshot_path: row.try_get(4)?,
        usage: serde_json::from_str::<TokenUsage>(&usage_json)
            .with_context(|| format!("parse session step usage JSON: {session_id}"))?,
        cost_usd: row.try_get(6)?,
        finish_reason: row.try_get(7)?,
        model: row.try_get(8)?,
    })
}

fn parse_agent_row(row: &Row, project_id: &str) -> Result<AgentSessionInfo> {
    let session_id: String = row.try_get(0)?;
    let info_json: String = row.try_get(1)?;
    parse_agent_info(&info_json, project_id, &session_id)
}

#[cfg(test)]
#[path = "postgres_tests.rs"]
mod postgres_tests;
