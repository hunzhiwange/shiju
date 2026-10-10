# Agent Scheduler

Agent Scheduler is the scheduling and local execution source of truth for agent
tasks in `vw-agent`. It owns durable task state, runs, workers, teams, resource
leases, events, outbox messages, logs, and artifacts used to decide what runs
next and to explain what already happened.

## Ownership Boundary

- `agent_scheduler` owns Agent Teams style scheduling facts and local dispatch.
- Gateway and desktop task-board flows should use Agent Scheduler DTOs and
  routes for task, run, log, artifact, event, wake, recovery, and outbox data.
- Markdown and JSON task artifacts are human-readable import/export surfaces,
  not the authoritative scheduler state.
- Matrix delivery is owned by durable outbox records plus `outbox_dispatcher`;
  scheduler transactions enqueue messages and do not send directly.
- Controller and Matrix integrations stay behind adapters. The scheduler stores
  snapshots, desired state, and audit events, but does not own external
  infrastructure.
- Worktree allocation is modeled as a resource lease. The task worktree pool is
  a non-scheduling capacity provider, not a compatibility scheduler.

## Module Roles

- `domain` defines strongly typed task, run, worker, team, capability, and lease
  IDs plus scheduler DTO conversions.
- `state_machine` validates task status transitions.
- `store` owns the scheduler portion of the selected shared user-level database and
  transactional reads/writes for tasks, runs, workers, resources, events,
  logs, artifacts, and outbox messages.
- `scheduler` contains pure selection logic for worker choice, recurring tasks,
  and team DAG planning.
- `engine` turns store facts and adapter snapshots into dispatch and recovery
  decisions.
- `runner` executes local scheduler runs and records observable output.
- `resources` adapts external capacity such as worktree slots into
  `ResourceLease` records.
- `adapters` defines controller and Matrix boundaries used by engines and
  dispatchers.
- `outbox_dispatcher` drains durable Matrix outbox messages with retry state.
- `artifacts` imports or exports human-readable task files.

## Global Storage And Scope

Project registration and Agent Scheduler use one user-level global database.
SQLite is the default; PostgreSQL is selected for the whole process through
`database/config.json` and `VIBEWINDOW_DATABASE_URL`:

- debug config: `~/.vibewindowdev/database/config.json`
- release config: `~/.vibewindow/database/config.json`
- SQLite data: `vibe-window.sqlite3` beside that config

The config accepts only lowercase `sqlite` or `postgres`; PostgreSQL also accepts
a validated schema name and requires an explicit `sslmode=require` or
`sslmode=disable` URL. `require` verifies the server with WebPKI roots. The URL
is read only from `VIBEWINDOW_DATABASE_URL`, is never persisted or logged, and
errors are redacted. The first successful selection for a config path is pinned
for the process lifetime, so changes require a restart.

`ProjectStore` owns `platform`, `company`, and `projects`.
`AgentSchedulerStore` separately owns the 12
scheduler tables: `scheduler_meta`, `scheduler_tasks`, `scheduler_runs`,
`scheduler_task_dependencies`,
`scheduler_task_logs`, `scheduler_task_artifacts`,
`scheduler_run_workspaces`, `scheduler_events`, `scheduler_outbox_messages`,
`scheduler_workers`, `scheduler_teams`, and `scheduler_resource_leases`.
Within the global 32-table matrix, `projects` and these 12 tables are the
scheduler's supported 13-table project-scoped integration set.
Scheduler rows use
`(platform_id, company_id, project_id)` as their scope prefix and reference the
matching `projects` row. Project deletion is restricted while scheduler rows
remain. Task and run descendants use scoped foreign keys; integer event and
artifact IDs are global cursors only and never replace scope predicates.

Every store, engine, runner, gateway, and client operation carries an explicit
tenant or project scope. Built-in desktop and CLI flows construct the local
tenant as `platform_id = local`, `company_id = local`; remote routes must send
both values explicitly. A worktree path is runtime context, not tenant
identity.

## Task Graph Scheduling

Finite tasks can have successful-completion dependencies stored in
`scheduler_task_dependencies`. A project-scoped `PUT` of `/task-graph`
atomically replaces the complete edge set; a `GET` returns the edges, current
task status, derived readiness, and waiting or blocking task IDs. Every graph
read, replacement, candidate lookup, and run claim carries the same
`(platform_id, company_id, project_id)` scope.

`Queued` means a task is in the scheduling pool, not that it is ready. A task
is ready only when every predecessor is currently `Completed`. A predecessor in
`Submitted` keeps the successor waiting; `Failed`, `Cancelled`, and `Archived`
block it by default. Readiness is derived at read and dispatch time and is
never persisted as a separate `Ready` state.

Wake is explicit. A task completion does not automatically dispatch its
successors; a later Matrix, local, directed, or project-local wake reevaluates
the graph. Each execution entry point prefilters on the same readiness rule,
then the SQLite or PostgreSQL claim transaction repeats the dependency check
while moving `Queued` to `Assigned` and creating the run atomically.

The first version covers only top-level `Finite` tasks and successful
dependencies. It does not migrate `SubTask.batch` or
`depends_on_batches`, add conditional or priority edges, or change worker
selection. Legacy task `dag_json`, `DependencyGraphStatus`, and `ready_nodes` remain
compatibility/planning data and do not participate in scheduler dispatch.

## Migration And Safety

SQLite startup retains the strict path cutover outside the scheduler table
schema, while the legacy project JSON import targets the selected global
provider:

1. With SQLite selected, the previous `database/_index.sqlite3`, or the older user-level
   `agent_scheduler/_index.sqlite3`, is moved to `database/vibe-window.sqlite3`
   only when the target and the other source are absent, after a non-blocking
   lock, integrity check, and WAL checkpoint. Conflicting paths fail startup
   without merging them.
2. Legacy project JSON under `storage/project` is parsed completely and
   imported into the local tenant before the source directory is moved to the
   global migration backup.

Scheduler initialization creates and accesses only the current `scheduler_*`
tables. If the global database contains an unprefixed scheduler table,
initialization fails explicitly without migrating, renaming, or deleting it.
The legacy global path move above still preserves the database contents, so an
old database with unsupported tables remains available for operator recovery.

Project-local `agent_scheduler/_index.sqlite3` and `tasks/_index.sqlite3`
databases are not scanned, read, modified, or deleted. Import of human-readable
Markdown and JSON task artifacts remains supported and is not SQLite schema
compatibility.

SQLite connections enable WAL, foreign keys, a bounded busy timeout, and a
schema lock. On Unix, the global database directory is mode `0700`; the
database and schema lock are mode `0600`. Errors and logs must not include
tokens or sensitive payloads.

PostgreSQL uses a shared synchronous pool, `$n` parameters, scoped foreign keys,
transaction-level advisory schema locks, and row locks/`SKIP LOCKED` for claims
and leases. The selected backend never probes or connects to the other backend.
There is no cross-backend read, fallback, replication, merge, migration, dual
write, or runtime switch. Switching is an operator-managed migration followed
by a restart; old backend data is preserved but not read.

PostgreSQL fixture tests take their DSN only from
`VIBEWINDOW_TEST_POSTGRES_URL`, create a random guarded `vw_test_<uuid>` schema,
and drop only that schema. The local profile maps `MaxOpenConns=100` to
`r2d2.max_size`, `MaxLifetime=300s` to `max_lifetime`, and `Debug=false` to
disabled SQL debug logging. `MaxIdleConns=10` is an unmapped baseline because
`r2d2` has no maximum-idle cap; it must not become `min_idle`. Existing local
business schemas and data are outside the fixture boundary.

## Notes From HiClaw

HiClaw separates Manager coordination, Worker execution, Teams, Matrix rooms,
controller reconciliation, and object storage. VibeWindow keeps that separation
inside Rust boundaries: the scheduler records decisions, adapters describe
external runtime state, and durable files remain readable artifacts.
