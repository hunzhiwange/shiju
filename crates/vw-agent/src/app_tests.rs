//! 测试应用任务门面的默认值、持久化和网关执行状态流转。
//! 临时数据库覆盖规范化、序号分配和失败完成，缺失存储时保持可恢复结果。

use super::task::{
    Task, claim_pending_gateway_execution_task, create_task, finish_gateway_execution_task,
    load_pending_gateway_execution_tasks,
};

#[test]
fn task_new_uses_stable_bootstrap_defaults() {
    let task = Task::new(10);

    assert_eq!(task.id, "bootstrap-task");
    assert_eq!(task.priority, 10);
    assert_eq!(task.model, "auto");
    assert_eq!(task.agent.as_deref(), Some("main"));
    assert_eq!(task.acp_agent, None);
    assert!(task.prompt.is_empty());
}

#[test]
fn create_task_persists_task_pool_item() {
    let temp = tempfile::tempdir().expect("temp dir");
    let task = Task {
        id: "id-1".to_string(),
        priority: 999,
        model: "model-a".to_string(),
        agent: Some("main".to_string()),
        acp_agent: None,
        prompt: "do work".to_string(),
    };

    let created = create_task(temp.path().to_str().expect("utf-8 temp path"), task.clone())
        .expect("task creation should succeed");

    assert!(created.id.starts_with('T'));
    assert_eq!(created.model, task.model);
    assert_eq!(created.prompt, task.prompt);
}

#[test]
fn config_bridge_sets_and_loads_unique_field() {
    let key = format!("app_tests_bridge_{}", std::process::id());

    super::config::set_config_field(&key, serde_json::json!({"enabled": true}));
    let config = super::config::load_app_config();

    assert_eq!(config[&key]["enabled"], true);
}

#[test]
fn agent_compat_namespace_exposes_selected_runtime_modules() {
    let config = super::agent::config::AutonomyConfig::default();
    let manager = super::agent::approval::ApprovalManager::from_config(&config);

    assert_eq!(config.level, super::agent::security::AutonomyLevel::Supervised);
    assert!(manager.needs_approval("file_write"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn create_task_normalizes_blank_fields_and_allocates_sequences() {
    let temp = tempfile::tempdir().expect("temp dir");

    let first = create_task(
        temp.path().to_str().expect("utf-8 temp path"),
        Task {
            id: "ignored".to_string(),
            priority: 7,
            model: "   ".to_string(),
            agent: Some("   ".to_string()),
            acp_agent: Some(" acp-main ".to_string()),
            prompt: "first".to_string(),
        },
    )
    .expect("first task should be created");
    let second = create_task(
        temp.path().to_str().expect("utf-8 temp path"),
        Task {
            id: "ignored-2".to_string(),
            priority: 8,
            model: " model-b ".to_string(),
            agent: None,
            acp_agent: Some(" ".to_string()),
            prompt: "second".to_string(),
        },
    )
    .expect("second task should be created");

    assert!(first.id.ends_with(".0001"));
    assert!(second.id.ends_with(".0002"));
    assert_eq!(first.model, "auto");
    assert_eq!(first.agent.as_deref(), Some("main"));
    assert_eq!(first.acp_agent.as_deref(), Some("acp-main"));
    assert_eq!(second.model, "model-b");
    assert_eq!(second.agent.as_deref(), Some("main"));
    assert_eq!(second.acp_agent, None);

    let conn = rusqlite::Connection::open(
        temp.path()
            .join(vw_config_types::paths::HOME_CONFIG_DIR_NAME)
            .join("tasks")
            .join("_index.sqlite3"),
    )
    .expect("task db should open");
    let (priority, model, agent, acp_agent, prompt, status_key, order_no): (
        i64,
        String,
        Option<String>,
        Option<String>,
        String,
        String,
        i64,
    ) = conn
        .query_row(
            "SELECT priority, model, agent, acp_agent, prompt, status_key, order_no
             FROM tasks WHERE id = ?1",
            [&first.id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .expect("created task should be persisted");

    assert_eq!(priority, 7);
    assert_eq!(model, "auto");
    assert_eq!(agent.as_deref(), Some("main"));
    assert_eq!(acp_agent.as_deref(), Some("acp-main"));
    assert_eq!(prompt, "first");
    assert_eq!(status_key, "pool");
    assert_eq!(order_no, 0);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn create_task_returns_io_error_when_project_path_is_file() {
    let temp = tempfile::tempdir().expect("temp dir");
    let project_file = temp.path().join("project-file");
    std::fs::write(&project_file, "").expect("project file should be written");

    let err = create_task(project_file.to_str().expect("utf-8 temp path"), Task::new(1))
        .expect_err("task creation should fail when project path is a file");

    assert!(
        matches!(err.kind(), std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::NotADirectory),
        "unexpected error kind: {:?}",
        err.kind()
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn task_database(temp: &tempfile::TempDir) -> std::path::PathBuf {
    temp.path()
        .join(vw_config_types::paths::HOME_CONFIG_DIR_NAME)
        .join("tasks")
        .join("_index.sqlite3")
}

#[cfg(not(target_arch = "wasm32"))]
fn create_pending_task(temp: &tempfile::TempDir, prompt: &str) -> Task {
    let created = create_task(
        temp.path().to_str().expect("utf-8 temp path"),
        Task {
            id: "ignored".to_string(),
            priority: 1,
            model: "model-a".to_string(),
            agent: Some("main".to_string()),
            acp_agent: None,
            prompt: prompt.to_string(),
        },
    )
    .expect("task should be created");
    let conn = rusqlite::Connection::open(task_database(temp)).expect("task db should open");
    conn.execute("UPDATE tasks SET status_key = 'pending' WHERE id = ?1", [&created.id])
        .expect("task should become pending");
    conn.execute(
        "UPDATE task_index_entries SET status_key = 'pending' WHERE task_id = ?1",
        [&created.id],
    )
    .expect("task index should become pending");
    created
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_task_load_claim_and_finish_success_update_persisted_state() {
    let temp = tempfile::tempdir().expect("temp dir");
    let task = create_pending_task(&temp, "execute this");
    let project = temp.path().to_str().expect("utf-8 temp path");

    let loaded =
        load_pending_gateway_execution_tasks(project, &["missing".to_string(), task.id.clone()])
            .expect("pending tasks should load");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, task.id);
    assert_eq!(loaded[0].prompt, "execute this");

    let claimed = claim_pending_gateway_execution_task(project, &task.id)
        .expect("claim should succeed")
        .expect("pending task should be claimed");
    assert_eq!(claimed.id, task.id);
    assert!(claim_pending_gateway_execution_task(project, &task.id).unwrap().is_none());

    finish_gateway_execution_task(project, &task.id, Ok("  completed output  ".to_string()))
        .expect("finish should succeed");

    let conn = rusqlite::Connection::open(task_database(&temp)).expect("task db should open");
    let (status, last_error, started): (String, Option<String>, Option<i64>) = conn
        .query_row(
            "SELECT status_key, last_error, execution_started_at_ms FROM tasks WHERE id = ?1",
            [&task.id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("finished task should exist");
    assert_eq!(status, "code_complete");
    assert!(last_error.is_none());
    assert!(started.is_none());
    let messages: Vec<String> = conn
        .prepare("SELECT message FROM task_logs WHERE task_id = ?1 ORDER BY seq_no")
        .unwrap()
        .query_map([&task.id], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(messages.iter().any(|message| message == "网关任务池 worker 开始执行"));
    assert!(messages.iter().any(|message| message == "网关执行完成: completed output"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_task_finish_failure_records_error_and_ignores_missing_storage() {
    let temp = tempfile::tempdir().expect("temp dir");
    let task = create_pending_task(&temp, "fail this");
    let project = temp.path().to_str().expect("utf-8 temp path");
    claim_pending_gateway_execution_task(project, &task.id)
        .expect("claim should succeed")
        .expect("pending task should be claimed");

    finish_gateway_execution_task(project, &task.id, Err("  provider failed  ".to_string()))
        .expect("failure should persist");

    let conn = rusqlite::Connection::open(task_database(&temp)).expect("task db should open");
    let (status, last_error): (String, Option<String>) = conn
        .query_row("SELECT status_key, last_error FROM tasks WHERE id = ?1", [&task.id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .expect("failed task should exist");
    assert_eq!(status, "failed");
    assert_eq!(last_error.as_deref(), Some("  provider failed  "));

    let missing = tempfile::tempdir().expect("missing project temp dir");
    let missing_project = missing.path().to_str().expect("utf-8 temp path");
    assert!(load_pending_gateway_execution_tasks(missing_project, &[]).unwrap().is_empty());
    assert!(claim_pending_gateway_execution_task(missing_project, "unknown").unwrap().is_none());
    finish_gateway_execution_task(missing_project, "unknown", Ok(String::new()))
        .expect("missing storage should be ignored");
}
