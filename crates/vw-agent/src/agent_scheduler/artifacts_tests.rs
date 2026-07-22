use std::fs;
use std::path::Path;

use anyhow::Result;
use chrono::DateTime;
use serde_json::{Value, json};
use vw_api_types::common::TimestampMs;

use super::artifacts::TaskArtifactWriter;
use super::domain::{TaskId, WorkerName};
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::tasks::StoredAgentTask;

#[test]
fn agent_artifacts_export_and_import_legacy_task_files() -> Result<()> {
    let artifact_root = tempfile::tempdir()?;
    let task = finite_task()?;
    let writer = TaskArtifactWriter::new();

    let paths = writer.write_task_artifacts(artifact_root.path(), &task)?;

    let meta: Value = serde_json::from_str(&fs::read_to_string(&paths.meta_json)?)?;
    assert_eq!(meta["type"], "finite");
    assert_eq!(meta["status"], "assigned");
    assert_eq!(meta["assigned_to"], "worker-a");
    assert_eq!(meta["room_id"], "!worker-room:example.test");
    assert_eq!(
        DateTime::parse_from_rfc3339(meta["created_at"].as_str().expect("created_at"))?
            .timestamp_millis(),
        task.created_at_ms.0
    );

    let spec = fs::read_to_string(&paths.spec_md)?;
    assert!(spec.contains("Build artifact export without MinIO."));
    assert!(spec.contains("- meta.json keeps HiClaw status"));
    assert!(spec.contains("- spec.md keeps the acceptance criteria"));
    assert!(fs::read_to_string(&paths.result_md)?.contains("No result yet."));

    let imported =
        writer.import_legacy_task(&artifact_root.path().join("shared/tasks/task-artifact-1"))?;
    assert_eq!(imported.task.id.as_str(), task.id.as_str());
    assert_eq!(imported.task.kind, AgentTaskKind::Finite);
    assert_eq!(imported.task.status, AgentTaskStatus::Assigned);
    assert_eq!(imported.task.assigned_to.as_ref().map(WorkerName::as_str), Some("worker-a"));
    assert_eq!(imported.task.room_id.as_deref(), Some("!worker-room:example.test"));
    assert_eq!(imported.task.created_at_ms, task.created_at_ms);
    assert_eq!(imported.spec.requirements, "Build artifact export without MinIO.");
    assert_eq!(
        imported.spec.acceptance_criteria,
        vec![
            "meta.json keeps HiClaw status".to_string(),
            "spec.md keeps the acceptance criteria".to_string()
        ]
    );

    Ok(())
}

#[test]
fn agent_artifacts_reject_non_finite_task_export() -> Result<()> {
    let artifact_root = tempfile::tempdir()?;
    let mut task = finite_task()?;
    task.kind = AgentTaskKind::Infinite;

    let error = TaskArtifactWriter::new()
        .write_task_artifacts(artifact_root.path(), &task)
        .expect_err("infinite task export must be rejected");

    assert!(error.to_string().contains("artifact export currently supports finite tasks only"));
    Ok(())
}

#[test]
fn agent_artifacts_reject_path_traversal_uri() -> Result<()> {
    let artifact_root = tempfile::tempdir()?;
    let mut task = finite_task()?;
    task.meta_uri = "../outside/meta.json".to_string();

    let error = TaskArtifactWriter::new()
        .write_task_artifacts(artifact_root.path(), &task)
        .expect_err("artifact path must remain under root");

    assert!(error.to_string().contains("artifact uri must stay under artifact root"));
    assert!(fs::read_dir(artifact_root.path())?.next().is_none());
    Ok(())
}

#[test]
fn agent_artifacts_reject_non_array_acceptance_criteria() -> Result<()> {
    let artifact_root = tempfile::tempdir()?;
    let mut task = finite_task()?;
    task.metadata["acceptance_criteria"] = json!("not-an-array");

    let error = TaskArtifactWriter::new()
        .write_task_artifacts(artifact_root.path(), &task)
        .expect_err("acceptance criteria must be a string array");

    assert!(
        error
            .to_string()
            .contains("task metadata field must be a string array: acceptance_criteria")
    );
    Ok(())
}

#[test]
fn agent_artifacts_export_maps_terminal_statuses_and_completed_time() -> Result<()> {
    let artifact_root = tempfile::tempdir()?;
    let writer = TaskArtifactWriter::new();

    let cases = [
        (AgentTaskStatus::Completed, "completed", true),
        (AgentTaskStatus::Failed, "failed", false),
        (AgentTaskStatus::Cancelled, "cancelled", false),
        (AgentTaskStatus::Archived, "archived", false),
    ];
    for (index, (status, legacy_status, expect_completed_at)) in cases.into_iter().enumerate() {
        let mut task = finite_task()?;
        task.id = TaskId::new(format!("task-terminal-status-{index}"))?;
        task.status = status;
        task.meta_uri = format!("shared/tasks/{}/meta.json", task.id.as_str());
        task.spec_uri = format!("shared/tasks/{}/spec.md", task.id.as_str());

        let paths = writer.write_task_artifacts(artifact_root.path(), &task)?;
        let meta: Value = serde_json::from_str(&fs::read_to_string(paths.meta_json)?)?;

        assert_eq!(meta["status"], legacy_status);
        assert_eq!(meta.get("completed_at").is_some(), expect_completed_at);
        if expect_completed_at {
            assert_eq!(
                DateTime::parse_from_rfc3339(meta["completed_at"].as_str().expect("completed_at"))?
                    .timestamp_millis(),
                task.updated_at_ms.0
            );
        }
    }

    Ok(())
}

#[test]
fn agent_artifacts_export_defaults_missing_spec_metadata() -> Result<()> {
    let artifact_root = tempfile::tempdir()?;
    let mut task = finite_task()?;
    task.metadata = json!({});

    let paths = TaskArtifactWriter::new().write_task_artifacts(artifact_root.path(), &task)?;

    let spec = fs::read_to_string(paths.spec_md)?;
    assert!(spec.contains("Artifact compatibility export"));
    assert!(spec.contains("- Completion can be verified from the task request."));
    assert!(spec.contains("Not provided."));
    assert_eq!(fs::read_to_string(paths.result_md)?, "# Result\n\nNo result recorded.\n");
    Ok(())
}

#[test]
fn agent_artifacts_reject_unsupported_finite_export_statuses() -> Result<()> {
    let artifact_root = tempfile::tempdir()?;
    let writer = TaskArtifactWriter::new();

    for status in [AgentTaskStatus::Active, AgentTaskStatus::Revision] {
        let mut task = finite_task()?;
        task.status = status;

        let error = writer
            .write_task_artifacts(artifact_root.path(), &task)
            .expect_err("active and revision are not HiClaw finite statuses");

        assert!(error.to_string().contains("unsupported finite task status for artifact export"));
    }
    Ok(())
}

#[test]
fn agent_artifacts_reject_empty_or_absolute_artifact_uri() -> Result<()> {
    let artifact_root = tempfile::tempdir()?;
    let writer = TaskArtifactWriter::new();

    let mut empty_meta_uri = finite_task()?;
    empty_meta_uri.meta_uri = "  ".to_string();
    let empty_error = writer
        .write_task_artifacts(artifact_root.path(), &empty_meta_uri)
        .expect_err("empty meta uri must be rejected");
    assert!(empty_error.to_string().contains("artifact uri must be a non-empty relative path"));

    let mut absolute_spec_uri = finite_task()?;
    absolute_spec_uri.spec_uri = "/tmp/spec.md".to_string();
    let absolute_error = writer
        .write_task_artifacts(artifact_root.path(), &absolute_spec_uri)
        .expect_err("absolute spec uri must be rejected");
    assert!(absolute_error.to_string().contains("artifact uri must be a non-empty relative path"));

    Ok(())
}

#[test]
fn agent_artifacts_import_legacy_defaults_from_directory_and_spec_title() -> Result<()> {
    let legacy_root = tempfile::tempdir()?;
    let task_dir = legacy_root.path().join("task-from-directory");
    write_legacy_task(
        &task_dir,
        json!({
            "type": "finite",
            "status": "completed",
            "delegated_to_team": "team-a",
            "schedule": "0 9 * * *",
            "timezone": "Asia/Shanghai",
            "next_scheduled_at": "2030-01-01T01:00:00Z",
            "last_executed_at": "2030-01-01T00:00:00Z"
        }),
        "# Imported Title\n\nLoose requirement text without known sections.\n",
    )?;

    let imported = TaskArtifactWriter::new().import_legacy_task(&task_dir)?;

    assert_eq!(imported.task.id.as_str(), "task-from-directory");
    assert_eq!(imported.task.title, "Imported Title");
    assert_eq!(imported.task.status, AgentTaskStatus::Completed);
    assert_eq!(imported.task.created_at_ms, TimestampMs(0));
    assert_eq!(imported.task.delegated_to_team.as_ref().map(|team| team.as_str()), Some("team-a"));
    assert_eq!(imported.task.schedule.as_deref(), Some("0 9 * * *"));
    assert_eq!(
        imported.spec.requirements,
        "# Imported Title\n\nLoose requirement text without known sections."
    );
    assert!(imported.spec.acceptance_criteria.is_empty());
    assert_eq!(imported.spec.context, None);
    assert_eq!(
        imported.task.metadata,
        json!({
            "requirements": "# Imported Title\n\nLoose requirement text without known sections.",
            "acceptance_criteria": []
        })
    );
    Ok(())
}

#[test]
fn agent_artifacts_import_maps_legacy_finite_statuses() -> Result<()> {
    let legacy_root = tempfile::tempdir()?;
    let writer = TaskArtifactWriter::new();
    let cases = [
        ("failed", AgentTaskStatus::Failed),
        ("cancelled", AgentTaskStatus::Cancelled),
        ("archived", AgentTaskStatus::Archived),
        ("blocked", AgentTaskStatus::Blocked),
        ("paused", AgentTaskStatus::Paused),
    ];

    for (status, expected) in cases {
        let task_dir = legacy_root.path().join(format!("task-{status}"));
        write_legacy_task(
            &task_dir,
            json!({
                "type": "finite",
                "status": status,
                "task_id": format!("task-{status}"),
                "title": format!("Task {status}")
            }),
            "## Requirements\n\nCheck status mapping.\n",
        )?;

        let imported = writer.import_legacy_task(&task_dir)?;
        assert_eq!(imported.task.status, expected);
    }

    Ok(())
}

#[test]
fn agent_artifacts_import_rejects_invalid_legacy_inputs() -> Result<()> {
    let legacy_root = tempfile::tempdir()?;
    let writer = TaskArtifactWriter::new();

    let missing_meta = legacy_root.path().join("missing-meta");
    fs::create_dir_all(&missing_meta)?;
    let error = writer.import_legacy_task(&missing_meta).expect_err("missing meta.json must fail");
    assert!(error.to_string().contains("read legacy task meta"));

    let malformed_meta = legacy_root.path().join("malformed-meta");
    fs::create_dir_all(&malformed_meta)?;
    fs::write(malformed_meta.join("meta.json"), "{not json")?;
    let error =
        writer.import_legacy_task(&malformed_meta).expect_err("malformed meta.json must fail");
    assert!(error.to_string().contains("parse legacy task meta"));

    let missing_spec = legacy_root.path().join("missing-spec");
    fs::create_dir_all(&missing_spec)?;
    fs::write(
        missing_spec.join("meta.json"),
        serde_json::to_string(&json!({"type": "finite", "status": "assigned"}))?,
    )?;
    let error = writer.import_legacy_task(&missing_spec).expect_err("missing spec.md must fail");
    assert!(error.to_string().contains("read legacy task spec"));

    let infinite = legacy_root.path().join("infinite");
    write_legacy_task(
        &infinite,
        json!({"type": "infinite", "status": "active"}),
        "## Requirements\n\nNot importable.\n",
    )?;
    let error = writer.import_legacy_task(&infinite).expect_err("infinite import must fail");
    assert!(
        error.to_string().contains("legacy artifact import currently supports finite tasks only")
    );

    let invalid_created_at = legacy_root.path().join("invalid-created-at");
    write_legacy_task(
        &invalid_created_at,
        json!({
            "type": "finite",
            "status": "assigned",
            "created_at": "tomorrow-ish"
        }),
        "## Requirements\n\nNeeds valid timestamp.\n",
    )?;
    let error =
        writer.import_legacy_task(&invalid_created_at).expect_err("invalid created_at must fail");
    assert!(error.to_string().contains("parse legacy task created_at"));

    Ok(())
}

fn finite_task() -> Result<StoredAgentTask> {
    Ok(StoredAgentTask {
        id: TaskId::new("task-artifact-1")?,
        title: "Artifact compatibility export".to_string(),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Assigned,
        spec_uri: "shared/tasks/task-artifact-1/spec.md".to_string(),
        meta_uri: "shared/tasks/task-artifact-1/meta.json".to_string(),
        assigned_to: Some(WorkerName::new("worker-a")?),
        team_id: None,
        room_id: Some("!worker-room:example.test".to_string()),
        project_room_id: Some("!project-room:example.test".to_string()),
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({
            "requirements": "Build artifact export without MinIO.",
            "acceptance_criteria": [
                "meta.json keeps HiClaw status",
                "spec.md keeps the acceptance criteria"
            ],
            "context": "Runs from structured scheduler facts.",
            "result": "No result yet."
        }),
        created_at_ms: TimestampMs(1_783_000_100_000),
        updated_at_ms: TimestampMs(1_783_000_200_000),
    })
}

fn write_legacy_task(task_dir: &Path, meta: Value, spec: &str) -> Result<()> {
    fs::create_dir_all(task_dir)?;
    fs::write(task_dir.join("meta.json"), serde_json::to_string_pretty(&meta)?)?;
    fs::write(task_dir.join("spec.md"), spec)?;
    Ok(())
}
