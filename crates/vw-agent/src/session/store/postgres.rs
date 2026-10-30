//! 会话、消息、步骤与模型原始记录的 PostgreSQL 持久化实现。
//!
//! 查询参数始终携带完整项目作用域，防止跨租户或跨项目读取会话数据。

use std::collections::HashSet;

use ::postgres::{GenericClient, Row};
use anyhow::{Context, Result, bail};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::session::info::Info as AgentSessionInfo;
use vw_shared::session::ui_types::{
    ChatMessage, ChatSession, ChatSessionMeta, ChatSessionStep, ProcessingTiming,
    SessionMessageDelta, SessionMessagePage, SessionMessageRecord, SessionTodoItem, ThinkTiming,
    TokenUsage, ToolTiming,
};

use super::{
    LlmRawArtifact, LlmRawRound, LlmRawTurnContext, SessionMessageDeltaError,
    SessionMessageDeltaResult, from_i64, from_i64_usize, message_processing_values,
    parse_agent_info, resulting_message_count, role_from_db, role_to_db, session_last_content,
    to_i64, usize_to_i64,
    validate_session_message_delta,
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
    turn_id TEXT NOT NULL DEFAULT '',
    started_ms BIGINT NOT NULL,
    finished_ms BIGINT,
    start_snapshot_path TEXT,
    finish_snapshot_path TEXT,
    usage_json TEXT NOT NULL,
    cost_usd DOUBLE PRECISION,
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

ALTER TABLE session_steps ADD COLUMN IF NOT EXISTS tool_timings_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE session_steps ADD COLUMN IF NOT EXISTS turn_id TEXT NOT NULL DEFAULT '';
UPDATE session_steps SET turn_id = '' WHERE turn_id IS NULL;
ALTER TABLE session_steps ALTER COLUMN turn_id SET DEFAULT '';
ALTER TABLE session_steps ALTER COLUMN turn_id SET NOT NULL;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'session_steps'::regclass
          AND contype = 'p'
          AND pg_get_constraintdef(oid) =
              'PRIMARY KEY (platform_id, company_id, project_id, session_id, turn_id, step_index)'
    ) THEN
        ALTER TABLE session_steps DROP CONSTRAINT session_steps_pkey;
        ALTER TABLE session_steps
            ADD PRIMARY KEY (platform_id, company_id, project_id, session_id, turn_id, step_index);
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_session_steps_project_index
ON session_steps(platform_id, company_id, project_id, session_id, step_index);

CREATE INDEX IF NOT EXISTS idx_session_steps_project_turn_index
ON session_steps(platform_id, company_id, project_id, session_id, turn_id, step_index);

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
    step_index BIGINT NOT NULL,
    provider_message_prefix_len BIGINT NOT NULL,
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

ALTER TABLE session_llm_raw_rounds ADD COLUMN IF NOT EXISTS system_delta_json TEXT;
ALTER TABLE session_llm_raw_rounds ADD COLUMN IF NOT EXISTS tools_delta_json TEXT;

CREATE INDEX IF NOT EXISTS idx_session_llm_raw_rounds_project_turn_step
ON session_llm_raw_rounds(platform_id, company_id, project_id, session_id, turn_id, step_index);

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

/// PostgreSQL 会话存储句柄，所有方法都要求显式租户或项目作用域。
#[derive(Debug, Clone)]
pub(super) struct PostgresSessionStore {
    database: GlobalDatabase,
}

impl PostgresSessionStore {
    /// 初始化会话、消息、步骤、待办和代理会话表。
    pub(super) fn new(database: GlobalDatabase) -> Result<Self> {
        database.initialize_postgres_schema(|transaction| {
            transaction
                .batch_execute(SESSION_SCHEMA)
                .context("initialize PostgreSQL session schema")?;
            Ok(())
        })?;
        Ok(Self { database })
    }

    /// 在单个事务内保存完整会话头、消息、调用和步骤快照。
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

    /// 保存 UI 所需的会话头、消息和步骤，保留已有调用计数。
    pub(super) fn save_session_ui_snapshot(
        &self,
        scope: &ProjectScope,
        session: &ChatSession,
    ) -> Result<()> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL session UI snapshot save")?;
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
        transaction.commit().context("commit PostgreSQL session UI snapshot save")?;
        Ok(())
    }

    /// 仅保存 UI 会话头与消息内容，不替换步骤或调用记录。
    pub(super) fn save_session_ui_content(
        &self,
        scope: &ProjectScope,
        session: &ChatSession,
    ) -> Result<()> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL session UI content save")?;
        reject_stale_session_ui_content(&mut transaction, scope, session)?;
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
        transaction.commit().context("commit PostgreSQL session UI content save")?;
        Ok(())
    }

    /// 原子提交 UI 快照及对应模型原始制品。
    pub(super) fn commit_session_ui_snapshot(
        &self,
        scope: &ProjectScope,
        session: &ChatSession,
        raw_artifacts: &[LlmRawArtifact],
    ) -> Result<()> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL session UI raw commit")?;
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
        upsert_session_llm_raw_artifacts(&mut transaction, scope, raw_artifacts)?;
        transaction.commit().context("commit PostgreSQL session UI raw commit")?;
        Ok(())
    }

    /// 按完整项目作用域读取会话。
    pub(super) fn load_session(
        &self,
        scope: &ProjectScope,
        session_id: &str,
    ) -> Result<Option<ChatSession>> {
        let mut connection = self.database.connect_postgres()?;
        load_session(&mut *connection, scope, session_id)
    }

    pub(super) fn load_session_message_page(
        &self,
        scope: &ProjectScope,
        session_id: &str,
        before_seq_no: Option<u64>,
        limit: usize,
    ) -> Result<SessionMessagePage> {
        let mut connection = self.database.connect_postgres()?;
        load_session_message_page_postgres(
            &mut *connection,
            scope,
            session_id,
            before_seq_no,
            limit,
        )
    }

    pub(super) fn apply_session_message_delta(
        &self,
        scope: &ProjectScope,
        delta: &SessionMessageDelta,
    ) -> Result<SessionMessageDeltaResult> {
        validate_session_message_delta(delta)
            .map_err(|error| SessionMessageDeltaError::Invalid(error.to_string()))?;
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL session message delta")?;
        let result = apply_session_message_delta_postgres(&mut transaction, scope, delta)?;
        transaction.commit().context("commit PostgreSQL session message delta")?;
        Ok(result)
    }

    /// 在租户内查找最近更新的同标识会话，再以其项目作用域加载。
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

    /// 列出项目最近更新的会话摘要，最多返回固定上限。
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

    /// 读取单个项目会话的索引摘要。
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

    /// 在事务内用给定快照完整替换项目会话索引。
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

    /// 删除项目作用域内的会话及级联子记录。
    pub(super) fn delete_session(&self, scope: &ProjectScope, session_id: &str) -> Result<bool> {
        let changed = self
            .database
            .connect_postgres()?
            .execute(
                "DELETE FROM session_index
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND session_id = $4",
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                    &session_id,
                ],
            )
            .context("delete scoped PostgreSQL session")?;
        Ok(changed == 1)
    }

    /// 按输入顺序原子替换会话待办列表。
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

    /// 保存兼容旧格式的单步骤模型原始 JSON。
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

    /// 重放指定轮次到目标步骤并重建模型原始请求与输出。
    pub(super) fn load_session_turn_step_llm_raw(
        &self,
        scope: &ProjectScope,
        session_id: &str,
        turn_id: &str,
        step_index: u32,
    ) -> Result<Option<serde_json::Value>> {
        let mut connection = self.database.connect_postgres()?;
        let context = connection
            .query_opt(
                "SELECT model_json, system_json, tools_json, initial_request_config_json,
                        initial_provider_messages_json, initial_message_sources_json
                 FROM session_llm_raw_turns
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND session_id = $4 AND turn_id = $5",
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                    &session_id,
                    &turn_id,
                ],
            )?
            .map(|row| {
                Ok::<LlmRawTurnContext, anyhow::Error>(LlmRawTurnContext {
                    turn_id: turn_id.to_owned(),
                    session_id: session_id.to_owned(),
                    model: parse_raw_json(row.try_get(0)?, "turn model")?,
                    system: parse_raw_json(row.try_get(1)?, "turn system")?,
                    tools: parse_raw_json(row.try_get(2)?, "turn tools")?,
                    initial_request_config: parse_raw_json(
                        row.try_get(3)?,
                        "turn initial request config",
                    )?,
                    initial_provider_messages: parse_raw_json(
                        row.try_get(4)?,
                        "turn initial provider messages",
                    )?,
                    initial_message_sources: parse_raw_json(
                        row.try_get(5)?,
                        "turn initial message sources",
                    )?,
                })
            })
            .transpose()?;
        let Some(context) = context else {
            return Ok(None);
        };
        let rows = connection.query(
            "SELECT step_index, provider_message_prefix_len, provider_message_suffix_json,
                    message_source_suffix_json, system_delta_json, tools_delta_json,
                    request_config_delta_json, output_json
             FROM session_llm_raw_rounds
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
               AND session_id = $4 AND turn_id = $5 AND step_index <= $6
             ORDER BY step_index",
            &[
                &scope.platform_id().as_ref(),
                &scope.company_id().as_ref(),
                &scope.project_id().as_ref(),
                &session_id,
                &turn_id,
                &i64::from(step_index),
            ],
        )?;
        let rounds =
            rows.iter().map(|row| postgres_raw_round(row, turn_id)).collect::<Result<Vec<_>>>()?;
        if rounds.last().map(|round| round.step_index) != Some(step_index) {
            return Ok(None);
        }
        Ok(Some(LlmRawArtifact { context, rounds }.reconstruct_round(step_index)?))
    }

    /// 优先从轮次制品重建步骤，缺失轮次关联时回退读取旧格式记录。
    pub(super) fn load_session_step_llm_raw(
        &self,
        scope: &ProjectScope,
        session_id: &str,
        step_index: u32,
    ) -> Result<Option<serde_json::Value>> {
        let mut connection = self.database.connect_postgres()?;
        let turn_id = connection
            .query_opt(
                "SELECT turn_id FROM session_steps
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND session_id = $4 AND step_index = $5",
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                    &session_id,
                    &to_i64(step_index as u64, "step index")?,
                ],
            )?
            .map(|row| row.try_get::<_, Option<String>>(0))
            .transpose()?;
        if let Some(turn_id) = turn_id.flatten().filter(|turn_id| !turn_id.is_empty()) {
            return self.load_session_turn_step_llm_raw(scope, session_id, &turn_id, step_index);
        }
        let step_index = to_i64(step_index as u64, "step index")?;
        let row = connection
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

    /// 合并新旧存储格式中存在原始制品的步骤索引。
    pub(super) fn load_session_llm_raw_step_indices(
        &self,
        scope: &ProjectScope,
        session_id: &str,
    ) -> Result<Vec<u32>> {
        self.database
            .connect_postgres()?
            .query(
                "SELECT step_index FROM session_llm_raw_rounds
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND session_id = $4
                 UNION
                 SELECT step_index FROM session_step_llm_raw
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND session_id = $4
                 ORDER BY step_index",
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                    &session_id,
                ],
            )
            .context("load PostgreSQL session llm raw step indices")?
            .iter()
            .map(|row| {
                let step_index: i64 = row.try_get(0)?;
                u32::try_from(step_index).context("convert PostgreSQL LLM raw step index to u32")
            })
            .collect()
    }

    /// 按持久化顺序读取会话待办。
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

    /// 按项目作用域插入或更新代理会话信息。
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

    /// 按完整项目作用域读取代理会话。
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

    /// 列出项目中的全部代理会话。
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

    /// 列出租户代理会话，并按会话标识保留最近更新记录。
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

    /// 在租户内读取最近更新的同标识代理会话。
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

    /// 删除项目作用域内的代理会话。
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

fn load_session_message_page_postgres(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    session_id: &str,
    before_seq_no: Option<u64>,
    limit: usize,
) -> Result<SessionMessagePage> {
    let fetch_limit = limit.checked_add(1).context("session message page limit overflow")?;
    let fetch_limit = usize_to_i64(fetch_limit, "session message page limit")?;
    let rows = match before_seq_no {
        Some(before) => {
            let before = to_i64(before, "session message page boundary")?;
            client.query(
                "SELECT seq_no, message_id, role, content, model, think_timing_json,
                        processing_started_ms, processing_completed_ms, processing_elapsed_ms,
                        processing_llm_elapsed_ms
                 FROM session_messages
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND session_id = $4
                   AND seq_no < $5
                 ORDER BY seq_no DESC LIMIT $6",
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                    &session_id,
                    &before,
                    &fetch_limit,
                ],
            )?
        }
        None => client.query(
            "SELECT seq_no, message_id, role, content, model, think_timing_json,
                    processing_started_ms, processing_completed_ms, processing_elapsed_ms,
                    processing_llm_elapsed_ms
             FROM session_messages
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND session_id = $4
             ORDER BY seq_no DESC LIMIT $5",
            &[
                &scope.platform_id().as_ref(),
                &scope.company_id().as_ref(),
                &scope.project_id().as_ref(),
                &session_id,
                &fetch_limit,
            ],
        )?,
    };
    let mut records = rows
        .iter()
        .map(|row| postgres_message_record(row, session_id))
        .collect::<Result<Vec<_>>>()?;
    let has_more = records.len() > limit;
    records.truncate(limit);
    records.reverse();
    Ok(SessionMessagePage { records, has_more })
}

fn postgres_message_record(row: &Row, session_id: &str) -> Result<SessionMessageRecord> {
    let processing_started_ms: Option<i64> = row.try_get(6)?;
    let processing_completed_ms: Option<i64> = row.try_get(7)?;
    let processing_elapsed_ms: Option<i64> = row.try_get(8)?;
    let processing_llm_elapsed_ms: Option<i64> = row.try_get(9)?;
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
        (None, Some(_)) => {
            bail!("session message processing completed_ms exists without started_ms: {session_id}")
        }
    };
    let role: String = row.try_get(2)?;
    let think_timing_json: String = row.try_get(5)?;
    Ok(SessionMessageRecord {
        seq_no: from_i64(row.try_get(0)?, "message seq_no")?,
        message_id: row.try_get(1)?,
        message: ChatMessage {
            role: role_from_db(&role)?,
            content: row.try_get(3)?,
            model: row.try_get(4)?,
            think_timing: serde_json::from_str(&think_timing_json).with_context(|| {
                format!("parse session message think timing JSON: {session_id}")
            })?,
            processing_timing,
        },
    })
}

fn apply_session_message_delta_postgres(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    delta: &SessionMessageDelta,
) -> Result<SessionMessageDeltaResult> {
    let row = client
        .query_opt(
            "SELECT updated_ms, message_count FROM session_index
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND session_id = $4
             FOR UPDATE",
            scoped_id_params!(scope, &delta.session_id),
        )?
        .ok_or(SessionMessageDeltaError::NotFound)?;
    let stored_updated_ms = from_i64(row.try_get(0)?, "session updated_ms")?;
    if stored_updated_ms != delta.expected_updated_ms {
        return Err(SessionMessageDeltaError::Conflict(format!(
            "stale session message delta for {}: expected updated_ms {}, stored updated_ms {}",
            delta.session_id,
            delta.expected_updated_ms,
            stored_updated_ms
        ))
        .into());
    }
    let message_count = resulting_message_count(
        from_i64(row.try_get(1)?, "session message_count")?,
        delta.truncate_after,
        &delta.upserts,
    )?;
    let truncated_rows = if let Some(boundary) = delta.truncate_after {
        let boundary = to_i64(boundary, "message truncate boundary")?;
        client.execute(
            "DELETE FROM session_messages
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND session_id = $4
               AND seq_no > $5",
            &[
                &scope.platform_id().as_ref(),
                &scope.company_id().as_ref(),
                &scope.project_id().as_ref(),
                &delta.session_id,
                &boundary,
            ],
        )? as usize
    } else {
        0
    };
    for record in &delta.upserts {
        upsert_session_message_postgres(client, scope, &delta.session_id, record)?;
    }
    let last_content = client
        .query_opt(
            "SELECT content FROM session_messages
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND session_id = $4
             ORDER BY seq_no DESC LIMIT 1",
            scoped_id_params!(scope, &delta.session_id),
        )?
        .map(|row| row.try_get::<_, String>(0))
        .transpose()?;
    let updated_ms = to_i64(delta.updated_ms, "session updated_ms")?;
    let message_count = to_i64(message_count, "session message_count")?;
    client.execute(
        "UPDATE session_index SET updated_ms = $5, message_count = $6, last_content = $7
         WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND session_id = $4",
        &[
            &scope.platform_id().as_ref(),
            &scope.company_id().as_ref(),
            &scope.project_id().as_ref(),
            &delta.session_id,
            &updated_ms,
            &message_count,
            &last_content,
        ],
    )?;
    Ok(SessionMessageDeltaResult { upserted_rows: delta.upserts.len(), truncated_rows })
}

fn upsert_session_message_postgres(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    session_id: &str,
    record: &SessionMessageRecord,
) -> Result<()> {
    let message = &record.message;
    let seq_no = to_i64(record.seq_no, "message seq_no")?;
    let think_timing_json = serde_json::to_string(&message.think_timing)?;
    let (started, completed, elapsed, llm_elapsed) = message_processing_values(message)?;
    client.execute(
        "INSERT INTO session_messages (
             platform_id, company_id, project_id, session_id, seq_no, role, message_id,
             content, model, think_timing_json, processing_started_ms, processing_completed_ms,
             processing_elapsed_ms, processing_llm_elapsed_ms
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
         ON CONFLICT (platform_id, company_id, project_id, session_id, seq_no) DO UPDATE SET
             role = excluded.role, message_id = excluded.message_id, content = excluded.content,
             model = excluded.model, think_timing_json = excluded.think_timing_json,
             processing_started_ms = excluded.processing_started_ms,
             processing_completed_ms = excluded.processing_completed_ms,
             processing_elapsed_ms = excluded.processing_elapsed_ms,
             processing_llm_elapsed_ms = excluded.processing_llm_elapsed_ms",
        &[
            &scope.platform_id().as_ref(),
            &scope.company_id().as_ref(),
            &scope.project_id().as_ref(),
            &session_id,
            &seq_no,
            &role_to_db(message.role),
            &record.message_id,
            &message.content,
            &message.model,
            &think_timing_json,
            &started,
            &completed,
            &elapsed,
            &llm_elapsed,
        ],
    )?;
    Ok(())
}

fn reject_stale_session_ui_content(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    session: &ChatSession,
) -> Result<()> {
    let stored_updated_ms = client
        .query_opt(
            "SELECT updated_ms FROM session_index
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND session_id = $4",
            scoped_id_params!(scope, &session.id),
        )?
        .map(|row| from_i64(row.get(0), "session updated_ms"))
        .transpose()?;
    if let Some(stored_updated_ms) = stored_updated_ms
        && session.updated_ms < stored_updated_ms
    {
        bail!(
            "stale session UI content for {}: incoming updated_ms {} is older than stored updated_ms {}",
            session.id,
            session.updated_ms,
            stored_updated_ms
        );
    }
    Ok(())
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
        let tool_timings_json = serde_json::to_string(&step.tool_timings)
            .context("serialize step tool timings JSON")?;
        client
            .execute(
                "INSERT INTO session_steps (
                     platform_id, company_id, project_id, session_id, step_index, turn_id, started_ms,
                     finished_ms, start_snapshot_path, finish_snapshot_path, usage_json, cost_usd,
                     finish_reason, model, tool_timings_json
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                    &session.id,
                    &step_index,
                    &step.turn_id.as_deref().unwrap_or(""),
                    &started_ms,
                    &finished_ms,
                    &step.start_snapshot_path,
                    &step.finish_snapshot_path,
                    &usage_json,
                    &step.cost_usd,
                    &step.finish_reason,
                    &step.model,
                    &tool_timings_json,
                ],
            )
            .context("insert scoped PostgreSQL session step")?;
    }
    Ok(())
}

fn upsert_session_llm_raw_artifacts(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    raw_artifacts: &[LlmRawArtifact],
) -> Result<()> {
    for artifact in raw_artifacts {
        upsert_session_llm_raw_turn(client, scope, &artifact.context)?;
        for round in &artifact.rounds {
            upsert_session_llm_raw_round(client, scope, &artifact.context.session_id, round)?;
        }
    }
    Ok(())
}

fn upsert_session_llm_raw_turn(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    context: &LlmRawTurnContext,
) -> Result<()> {
    let model_json = serde_json::to_string(&context.model)?;
    let system_json = serde_json::to_string(&context.system)?;
    let tools_json = serde_json::to_string(&context.tools)?;
    let initial_request_config_json = serde_json::to_string(&context.initial_request_config)?;
    let initial_provider_messages_json = serde_json::to_string(&context.initial_provider_messages)?;
    let initial_message_sources_json = serde_json::to_string(&context.initial_message_sources)?;
    client
        .execute(
            "INSERT INTO session_llm_raw_turns (
                 platform_id, company_id, project_id, session_id, turn_id, model_json, system_json,
                 tools_json, initial_request_config_json, initial_provider_messages_json,
                 initial_message_sources_json
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (platform_id, company_id, project_id, session_id, turn_id) DO UPDATE SET
                 model_json = excluded.model_json,
                 system_json = excluded.system_json,
                 tools_json = excluded.tools_json,
                 initial_request_config_json = excluded.initial_request_config_json,
                 initial_provider_messages_json = excluded.initial_provider_messages_json,
                 initial_message_sources_json = excluded.initial_message_sources_json",
            &[
                &scope.platform_id().as_ref(),
                &scope.company_id().as_ref(),
                &scope.project_id().as_ref(),
                &context.session_id,
                &context.turn_id,
                &model_json,
                &system_json,
                &tools_json,
                &initial_request_config_json,
                &initial_provider_messages_json,
                &initial_message_sources_json,
            ],
        )
        .context("upsert PostgreSQL session LLM raw turn")?;
    Ok(())
}

fn upsert_session_llm_raw_round(
    client: &mut impl GenericClient,
    scope: &ProjectScope,
    session_id: &str,
    round: &LlmRawRound,
) -> Result<()> {
    let step_index = i64::from(round.step_index);
    let prefix_len =
        usize_to_i64(round.provider_message_prefix_len, "LLM raw provider message prefix length")?;
    let provider_message_suffix_json = serde_json::to_string(&round.provider_message_suffix)?;
    let message_source_suffix_json = serde_json::to_string(&round.message_source_suffix)?;
    let system_delta_json = round.system_delta.as_ref().map(serde_json::to_string).transpose()?;
    let tools_delta_json = round.tools_delta.as_ref().map(serde_json::to_string).transpose()?;
    let request_config_delta_json =
        round.request_config_delta.as_ref().map(serde_json::to_string).transpose()?;
    let output_json = serde_json::to_string(&round.output)?;
    client
        .execute(
            "INSERT INTO session_llm_raw_rounds (
                 platform_id, company_id, project_id, session_id, turn_id, step_index,
                 provider_message_prefix_len, provider_message_suffix_json, message_source_suffix_json,
                 system_delta_json, tools_delta_json, request_config_delta_json, output_json
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (platform_id, company_id, project_id, session_id, turn_id, step_index)
                 DO UPDATE SET
                     provider_message_prefix_len = excluded.provider_message_prefix_len,
                     provider_message_suffix_json = excluded.provider_message_suffix_json,
                     message_source_suffix_json = excluded.message_source_suffix_json,
                     system_delta_json = excluded.system_delta_json,
                     tools_delta_json = excluded.tools_delta_json,
                     request_config_delta_json = excluded.request_config_delta_json,
                     output_json = excluded.output_json",
            &[
                &scope.platform_id().as_ref(),
                &scope.company_id().as_ref(),
                &scope.project_id().as_ref(),
                &session_id,
                &round.turn_id,
                &step_index,
                &prefix_len,
                &provider_message_suffix_json,
                &message_source_suffix_json,
                &system_delta_json,
                &tools_delta_json,
                &request_config_delta_json,
                &output_json,
            ],
        )
        .context("upsert PostgreSQL session LLM raw round")?;
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
            "SELECT step_index, turn_id, started_ms, finished_ms, start_snapshot_path, finish_snapshot_path,
                    usage_json, cost_usd, finish_reason, model, tool_timings_json
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
    let finished_ms: Option<i64> = row.try_get(3)?;
    let usage_json: String = row.try_get(6)?;
    let tool_timings_json: String = row.try_get(10)?;
    let turn_id: String = row.try_get(1)?;
    Ok(ChatSessionStep {
        index: u32::try_from(row.try_get::<_, i64>(0)?).context("read session step_index")?,
        turn_id: (!turn_id.is_empty()).then_some(turn_id),
        started_ms: from_i64(row.try_get(2)?, "step started_ms")?,
        finished_ms: finished_ms.map(|value| from_i64(value, "step finished_ms")).transpose()?,
        start_snapshot_path: row.try_get(4)?,
        finish_snapshot_path: row.try_get(5)?,
        usage: serde_json::from_str::<TokenUsage>(&usage_json)
            .with_context(|| format!("parse session step usage JSON: {session_id}"))?,
        cost_usd: row.try_get(7)?,
        finish_reason: row.try_get(8)?,
        model: row.try_get(9)?,
        tool_timings: serde_json::from_str::<Vec<ToolTiming>>(&tool_timings_json)
            .with_context(|| format!("parse session step tool timings JSON: {session_id}"))?,
    })
}

fn postgres_raw_round(row: &Row, turn_id: &str) -> Result<LlmRawRound> {
    Ok(LlmRawRound {
        turn_id: turn_id.to_owned(),
        step_index: u32::try_from(row.try_get::<_, i64>(0)?)
            .context("read PostgreSQL LLM raw step_index")?,
        provider_message_prefix_len: usize::try_from(row.try_get::<_, i64>(1)?)
            .context("read PostgreSQL LLM raw provider message prefix length")?,
        provider_message_suffix: parse_raw_json::<Vec<serde_json::Value>>(
            row.try_get(2)?,
            "provider message suffix",
        )?,
        message_source_suffix: parse_raw_json::<Vec<serde_json::Value>>(
            row.try_get(3)?,
            "message source suffix",
        )?,
        system_delta: row
            .try_get::<_, Option<String>>(4)?
            .map(|json| parse_raw_json::<serde_json::Value>(json, "system delta"))
            .transpose()?,
        tools_delta: row
            .try_get::<_, Option<String>>(5)?
            .map(|json| parse_raw_json::<serde_json::Value>(json, "tools delta"))
            .transpose()?,
        request_config_delta: row
            .try_get::<_, Option<String>>(6)?
            .map(|json| parse_raw_json::<serde_json::Value>(json, "request config delta"))
            .transpose()?,
        output: parse_raw_json::<serde_json::Value>(row.try_get(7)?, "output")?,
    })
}

fn parse_raw_json<T: serde::de::DeserializeOwned>(json: String, field: &str) -> Result<T> {
    serde_json::from_str(&json)
        .with_context(|| format!("parse stored PostgreSQL LLM raw {field} JSON"))
}

fn parse_agent_row(row: &Row, project_id: &str) -> Result<AgentSessionInfo> {
    let session_id: String = row.try_get(0)?;
    let info_json: String = row.try_get(1)?;
    parse_agent_info(&info_json, project_id, &session_id)
}

#[cfg(test)]
#[path = "postgres_tests.rs"]
mod postgres_tests;
