use anyhow::{Context, Result, bail};

use super::super::schema::{LEGACY_SCHEDULER_TABLES, SCHEDULER_TABLES};

pub(super) fn reject_unsupported_schema(
    connection: &mut impl ::postgres::GenericClient,
) -> Result<()> {
    for table in LEGACY_SCHEDULER_TABLES {
        let exists: bool = connection
            .query_one(
                "SELECT EXISTS(
                     SELECT 1
                     FROM information_schema.tables
                     WHERE table_schema = current_schema()
                       AND table_name = $1
                       AND table_type = 'BASE TABLE'
                 )",
                &[&table],
            )
            .context("inspect PostgreSQL agent scheduler legacy tables")?
            .get(0);
        if exists {
            bail!("unsupported unprefixed agent scheduler table: {table}");
        }
    }
    for table in SCHEDULER_TABLES {
        let exists: bool = connection
            .query_one(
                "SELECT EXISTS(
                     SELECT 1
                     FROM information_schema.tables
                     WHERE table_schema = current_schema()
                       AND table_name = $1
                       AND table_type = 'BASE TABLE'
                 )",
                &[&table],
            )
            .context("inspect PostgreSQL agent scheduler current tables")?
            .get(0);
        if !exists {
            continue;
        }
        for column in ["platform_id", "company_id", "project_id"] {
            let column_exists: bool = connection
                .query_one(
                    "SELECT EXISTS(
                         SELECT 1
                         FROM information_schema.columns
                         WHERE table_schema = current_schema()
                           AND table_name = $1 AND column_name = $2
                           AND is_nullable = 'NO'
                     )",
                    &[&table, &column],
                )
                .context("inspect PostgreSQL agent scheduler scope columns")?
                .get(0);
            if !column_exists {
                bail!(
                    "unsupported unscoped agent scheduler table: {table} missing required column {column}"
                );
            }
        }
    }
    Ok(())
}

pub(super) fn initialize_schema(connection: &mut impl ::postgres::GenericClient) -> Result<()> {
    connection
        .batch_execute(AGENT_SCHEDULER_SCHEMA)
        .context("initialize PostgreSQL agent scheduler schema")?;
    Ok(())
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod schema_tests;

const AGENT_SCHEDULER_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS scheduler_meta (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_id, key),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS scheduler_tasks (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    title TEXT NOT NULL,
    spec_uri TEXT NOT NULL DEFAULT '',
    meta_uri TEXT NOT NULL DEFAULT '',
    assigned_to TEXT,
    team_id TEXT,
    room_id TEXT,
    project_room_id TEXT,
    delegated_to_team TEXT,
    schedule TEXT,
    timezone TEXT,
    next_scheduled_at TEXT,
    last_executed_at TEXT,
    dag_json TEXT NOT NULL DEFAULT '[]',
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_id, id),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS scheduler_runs (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    worker_id TEXT,
    status TEXT NOT NULL,
    attempt BIGINT NOT NULL,
    lease_acquired_at_ms BIGINT,
    lease_expires_at_ms BIGINT,
    started_at_ms BIGINT,
    finished_at_ms BIGINT,
    last_heartbeat_at_ms BIGINT,
    error_message TEXT,
    PRIMARY KEY (platform_id, company_id, project_id, id),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (platform_id, company_id, project_id, task_id)
        REFERENCES scheduler_tasks(platform_id, company_id, project_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS scheduler_task_logs (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    run_id TEXT,
    seq_no BIGINT NOT NULL,
    timestamp_ms BIGINT NOT NULL,
    status_from TEXT,
    status_to TEXT,
    status_from_key TEXT,
    status_to_key TEXT,
    message TEXT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_id, task_id, seq_no),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (platform_id, company_id, project_id, task_id)
        REFERENCES scheduler_tasks(platform_id, company_id, project_id, id) ON DELETE CASCADE,
    FOREIGN KEY (platform_id, company_id, project_id, run_id)
        REFERENCES scheduler_runs(platform_id, company_id, project_id, id)
);

CREATE TABLE IF NOT EXISTS scheduler_task_artifacts (
    id BIGSERIAL PRIMARY KEY,
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    run_id TEXT,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    path TEXT,
    payload_json TEXT NOT NULL DEFAULT '{}',
    created_at_ms BIGINT NOT NULL,
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (platform_id, company_id, project_id, task_id)
        REFERENCES scheduler_tasks(platform_id, company_id, project_id, id) ON DELETE CASCADE,
    FOREIGN KEY (platform_id, company_id, project_id, run_id)
        REFERENCES scheduler_runs(platform_id, company_id, project_id, id)
);

CREATE TABLE IF NOT EXISTS scheduler_run_workspaces (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    slot_id TEXT NOT NULL,
    path TEXT NOT NULL,
    branch TEXT NOT NULL,
    recorded_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_id, run_id),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (platform_id, company_id, project_id, run_id)
        REFERENCES scheduler_runs(platform_id, company_id, project_id, id) ON DELETE CASCADE,
    FOREIGN KEY (platform_id, company_id, project_id, task_id)
        REFERENCES scheduler_tasks(platform_id, company_id, project_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS scheduler_events (
    id BIGSERIAL PRIMARY KEY,
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    task_id TEXT,
    run_id TEXT,
    worker_id TEXT,
    team_id TEXT,
    kind TEXT NOT NULL,
    from_status TEXT,
    to_status TEXT,
    from_status_key TEXT,
    to_status_key TEXT,
    message TEXT NOT NULL DEFAULT '',
    payload_json TEXT NOT NULL DEFAULT '{}',
    created_at_ms BIGINT NOT NULL,
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS scheduler_outbox_messages (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    target_kind TEXT NOT NULL,
    target_id TEXT NOT NULL,
    recipient_id TEXT,
    body TEXT NOT NULL,
    status TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    attempts BIGINT NOT NULL DEFAULT 0,
    next_attempt_at_ms BIGINT,
    delivered_at_ms BIGINT,
    last_error TEXT,
    payload_json TEXT NOT NULL DEFAULT '{}',
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_id, id),
    UNIQUE (platform_id, company_id, project_id, idempotency_key),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS scheduler_workers (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    runtime TEXT,
    deployment TEXT,
    model TEXT NOT NULL DEFAULT '',
    model_provider TEXT,
    status TEXT NOT NULL,
    desired_state TEXT,
    team_id TEXT,
    worker_role TEXT,
    matrix_user_id TEXT,
    room_id TEXT,
    container_state TEXT,
    idle_since_ms BIGINT,
    last_seen_at_ms BIGINT,
    last_heartbeat_at TEXT,
    capabilities_json TEXT NOT NULL DEFAULT '[]',
    updated_at_ms BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (platform_id, company_id, project_id, id),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS scheduler_teams (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    leader_id TEXT NOT NULL,
    team_room_id TEXT,
    leader_dm_room_id TEXT,
    leader_ready BOOLEAN NOT NULL,
    ready_workers BIGINT NOT NULL,
    total_workers BIGINT NOT NULL,
    members_json TEXT NOT NULL DEFAULT '[]',
    updated_at_ms BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (platform_id, company_id, project_id, id),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS scheduler_resource_leases (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    resource_kind TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    holder_id TEXT NOT NULL,
    status TEXT NOT NULL,
    acquired_at_ms BIGINT NOT NULL,
    expires_at_ms BIGINT,
    released_at_ms BIGINT,
    taint_reason TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (platform_id, company_id, project_id, id),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_scheduler_tasks_status
ON scheduler_tasks(platform_id, company_id, project_id, status, id);
CREATE INDEX IF NOT EXISTS idx_scheduler_tasks_assignee
ON scheduler_tasks(platform_id, company_id, project_id, assigned_to, status, id);
CREATE INDEX IF NOT EXISTS idx_scheduler_tasks_next_scheduled_at
ON scheduler_tasks(platform_id, company_id, project_id, next_scheduled_at, id);
CREATE INDEX IF NOT EXISTS idx_scheduler_runs_task_status
ON scheduler_runs(platform_id, company_id, project_id, task_id, status, started_at_ms);
CREATE INDEX IF NOT EXISTS idx_scheduler_runs_lease_expiry
ON scheduler_runs(platform_id, company_id, project_id, status, lease_expires_at_ms, id);
CREATE INDEX IF NOT EXISTS idx_scheduler_task_logs_task_seq
ON scheduler_task_logs(platform_id, company_id, project_id, task_id, seq_no);
CREATE INDEX IF NOT EXISTS idx_scheduler_task_logs_run_seq
ON scheduler_task_logs(platform_id, company_id, project_id, run_id, seq_no);
CREATE INDEX IF NOT EXISTS idx_scheduler_task_artifacts_task
ON scheduler_task_artifacts(platform_id, company_id, project_id, task_id, id);
CREATE INDEX IF NOT EXISTS idx_scheduler_run_workspaces_task
ON scheduler_run_workspaces(platform_id, company_id, project_id, task_id, run_id);
CREATE INDEX IF NOT EXISTS idx_scheduler_events_task_cursor
ON scheduler_events(platform_id, company_id, project_id, task_id, id);
CREATE INDEX IF NOT EXISTS idx_scheduler_events_kind_cursor
ON scheduler_events(platform_id, company_id, project_id, kind, id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_scheduler_outbox_messages_idempotency_key
ON scheduler_outbox_messages(platform_id, company_id, project_id, idempotency_key);
CREATE INDEX IF NOT EXISTS idx_scheduler_outbox_delivery_status
ON scheduler_outbox_messages(platform_id, company_id, project_id, status, next_attempt_at_ms, id);
CREATE INDEX IF NOT EXISTS idx_scheduler_workers_status
ON scheduler_workers(platform_id, company_id, project_id, status, id);
CREATE INDEX IF NOT EXISTS idx_scheduler_workers_team_status
ON scheduler_workers(platform_id, company_id, project_id, team_id, status, id);
CREATE INDEX IF NOT EXISTS idx_scheduler_workers_runtime_status
ON scheduler_workers(platform_id, company_id, project_id, runtime, status, id);
CREATE INDEX IF NOT EXISTS idx_scheduler_workers_container_state
ON scheduler_workers(platform_id, company_id, project_id, container_state, status, id);
CREATE INDEX IF NOT EXISTS idx_scheduler_teams_status
ON scheduler_teams(platform_id, company_id, project_id, status, id);
CREATE INDEX IF NOT EXISTS idx_scheduler_resource_leases_resource_status
ON scheduler_resource_leases(platform_id, company_id, project_id, resource_kind, resource_id, status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_scheduler_resource_leases_active_resource
ON scheduler_resource_leases(platform_id, company_id, project_id, resource_kind, resource_id)
WHERE status = 'active';
CREATE INDEX IF NOT EXISTS idx_scheduler_resource_leases_holder
ON scheduler_resource_leases(platform_id, company_id, project_id, holder_id, status);
CREATE INDEX IF NOT EXISTS idx_scheduler_resource_leases_expiry
ON scheduler_resource_leases(platform_id, company_id, project_id, status, expires_at_ms, id);
"#;
