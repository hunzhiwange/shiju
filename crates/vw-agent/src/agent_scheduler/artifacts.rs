//! 负责有限任务产物的落盘与旧格式导入，并约束产物路径不得逃逸指定根目录。

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use vw_api_types::common::TimestampMs;

use super::domain::{TaskId, TeamId, WorkerName};
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::tasks::{NewAgentTask, StoredAgentTask};

#[derive(Debug, Clone, Default)]
pub struct TaskArtifactWriter;

impl TaskArtifactWriter {
    pub fn new() -> Self {
        Self
    }

    pub fn write_task_artifacts(
        &self,
        artifact_root: &Path,
        task: &StoredAgentTask,
    ) -> Result<TaskArtifactPaths> {
        ensure_finite_task(task)?;
        let meta_json = artifact_path(artifact_root, &task.meta_uri)?;
        let spec_md = artifact_path(artifact_root, &task.spec_uri)?;
        let task_dir = meta_json
            .parent()
            .with_context(|| format!("resolve artifact task directory: {}", meta_json.display()))?;
        let result_md = task_dir.join("result.md");
        let spec = TaskArtifactSpec::from_task(task)?;

        write_json_file(&meta_json, &build_meta_json(task)?)?;
        write_markdown_file(&spec_md, &build_spec_markdown(task, &spec))?;
        write_markdown_file(&result_md, &build_result_markdown(&spec))?;

        Ok(TaskArtifactPaths { meta_json, spec_md, result_md })
    }

    pub fn import_legacy_task(&self, task_dir: &Path) -> Result<ImportedLegacyTask> {
        let meta_path = task_dir.join("meta.json");
        let spec_path = task_dir.join("spec.md");
        let meta: LegacyTaskMeta = serde_json::from_str(
            &std::fs::read_to_string(&meta_path)
                .with_context(|| format!("read legacy task meta: {}", meta_path.display()))?,
        )
        .with_context(|| format!("parse legacy task meta: {}", meta_path.display()))?;
        let spec_markdown = std::fs::read_to_string(&spec_path)
            .with_context(|| format!("read legacy task spec: {}", spec_path.display()))?;
        let spec = parse_spec_markdown(&spec_markdown);
        let task_id = legacy_task_id(task_dir, &meta)?;
        let kind = AgentTaskKind::parse(&meta.task_type)?;
        ensure_importable_kind(kind)?;
        let status = legacy_status_to_agent_status(kind, &meta.status)?;
        let title = meta.title.clone().unwrap_or_else(|| title_from_spec(&spec_markdown, &task_id));
        let metadata = imported_metadata(&spec);

        Ok(ImportedLegacyTask {
            task: NewAgentTask {
                id: TaskId::new(task_id.clone())?,
                title,
                kind,
                status,
                spec_uri: format!("shared/tasks/{task_id}/spec.md"),
                meta_uri: format!("shared/tasks/{task_id}/meta.json"),
                assigned_to: meta.assigned_to.map(WorkerName::new).transpose()?,
                team_id: None,
                room_id: meta.room_id,
                project_room_id: meta.project_room_id,
                delegated_to_team: meta.delegated_to_team.map(TeamId::new).transpose()?,
                schedule: meta.schedule,
                timezone: meta.timezone,
                next_scheduled_at: meta.next_scheduled_at,
                last_executed_at: meta.last_executed_at,
                dag: Vec::new(),
                metadata,
                created_at_ms: parse_created_at_ms(meta.created_at.as_deref())?,
            },
            spec,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskArtifactPaths {
    pub meta_json: PathBuf,
    pub spec_md: PathBuf,
    pub result_md: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedLegacyTask {
    pub task: NewAgentTask,
    pub spec: TaskArtifactSpec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskArtifactSpec {
    pub requirements: String,
    pub acceptance_criteria: Vec<String>,
    pub context: Option<String>,
    pub result: Option<String>,
}

impl TaskArtifactSpec {
    fn from_task(task: &StoredAgentTask) -> Result<Self> {
        Ok(Self {
            requirements: metadata_string(&task.metadata, "requirements")
                .unwrap_or_else(|| task.title.clone()),
            acceptance_criteria: metadata_string_array(&task.metadata, "acceptance_criteria")?,
            context: metadata_string(&task.metadata, "context"),
            result: metadata_string(&task.metadata, "result"),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyTaskMeta {
    #[serde(rename = "type")]
    task_type: String,
    status: String,
    #[serde(default)]
    task_id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    assigned_to: Option<String>,
    #[serde(default)]
    room_id: Option<String>,
    #[serde(default)]
    project_room_id: Option<String>,
    #[serde(default)]
    delegated_to_team: Option<String>,
    #[serde(default)]
    schedule: Option<String>,
    #[serde(default)]
    timezone: Option<String>,
    #[serde(default)]
    next_scheduled_at: Option<String>,
    #[serde(default)]
    last_executed_at: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct ExportedTaskMeta {
    #[serde(rename = "type")]
    task_type: &'static str,
    status: &'static str,
    task_id: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    assigned_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    room_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_room_id: Option<String>,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<String>,
}

fn ensure_finite_task(task: &StoredAgentTask) -> Result<()> {
    if task.kind != AgentTaskKind::Finite {
        bail!("artifact export currently supports finite tasks only");
    }
    Ok(())
}

fn ensure_importable_kind(kind: AgentTaskKind) -> Result<()> {
    if kind != AgentTaskKind::Finite {
        bail!("legacy artifact import currently supports finite tasks only");
    }
    Ok(())
}

fn build_meta_json(task: &StoredAgentTask) -> Result<ExportedTaskMeta> {
    Ok(ExportedTaskMeta {
        task_type: "finite",
        status: hiclaw_finite_status(task.status)?,
        task_id: task.id.as_str().to_string(),
        title: task.title.clone(),
        assigned_to: task.assigned_to.as_ref().map(|worker| worker.as_str().to_string()),
        room_id: task.room_id.clone(),
        project_room_id: task.project_room_id.clone(),
        created_at: timestamp_ms_to_rfc3339(task.created_at_ms)?,
        completed_at: if task.status == AgentTaskStatus::Completed {
            Some(timestamp_ms_to_rfc3339(task.updated_at_ms)?)
        } else {
            None
        },
    })
}

fn hiclaw_finite_status(status: AgentTaskStatus) -> Result<&'static str> {
    match status {
        AgentTaskStatus::Draft
        | AgentTaskStatus::Queued
        | AgentTaskStatus::Assigned
        | AgentTaskStatus::Dispatched
        | AgentTaskStatus::Acknowledged
        | AgentTaskStatus::Running
        | AgentTaskStatus::Submitted
        | AgentTaskStatus::Blocked
        | AgentTaskStatus::Paused => Ok("assigned"),
        AgentTaskStatus::Completed => Ok("completed"),
        AgentTaskStatus::Failed => Ok("failed"),
        AgentTaskStatus::Cancelled => Ok("cancelled"),
        AgentTaskStatus::Archived => Ok("archived"),
        AgentTaskStatus::Active | AgentTaskStatus::Revision => {
            bail!("unsupported finite task status for artifact export: {}", status.as_str())
        }
    }
}

fn legacy_status_to_agent_status(kind: AgentTaskKind, status: &str) -> Result<AgentTaskStatus> {
    match (kind, status) {
        (AgentTaskKind::Finite, "assigned") => Ok(AgentTaskStatus::Assigned),
        (AgentTaskKind::Finite, "completed") => Ok(AgentTaskStatus::Completed),
        (AgentTaskKind::Finite, "failed") => Ok(AgentTaskStatus::Failed),
        (AgentTaskKind::Finite, "cancelled") => Ok(AgentTaskStatus::Cancelled),
        (AgentTaskKind::Finite, "archived") => Ok(AgentTaskStatus::Archived),
        (AgentTaskKind::Finite, "blocked") => Ok(AgentTaskStatus::Blocked),
        (AgentTaskKind::Finite, "paused") => Ok(AgentTaskStatus::Paused),
        _ => AgentTaskStatus::parse(status),
    }
}

fn build_spec_markdown(task: &StoredAgentTask, spec: &TaskArtifactSpec) -> String {
    let mut output = String::new();
    output.push_str(&format!("# {}\n\n", task.title.trim()));
    output.push_str("## Requirements\n\n");
    output.push_str(spec.requirements.trim());
    output.push_str("\n\n## Acceptance Criteria\n\n");
    if spec.acceptance_criteria.is_empty() {
        output.push_str("- Completion can be verified from the task request.\n");
    } else {
        for criterion in &spec.acceptance_criteria {
            output.push_str(&format!("- {}\n", criterion.trim()));
        }
    }
    output.push_str("\n## Context\n\n");
    output.push_str(spec.context.as_deref().unwrap_or("Not provided.").trim());
    output.push('\n');
    output
}

fn build_result_markdown(spec: &TaskArtifactSpec) -> String {
    let result = spec.result.as_deref().unwrap_or("No result recorded.").trim();
    format!("# Result\n\n{result}\n")
}

fn parse_spec_markdown(markdown: &str) -> TaskArtifactSpec {
    let requirements =
        markdown_section(markdown, "Requirements").unwrap_or_else(|| markdown.trim().to_string());
    let acceptance_criteria = markdown_section(markdown, "Acceptance Criteria")
        .map(|section| parse_acceptance_criteria(&section))
        .unwrap_or_default();
    let context = markdown_section(markdown, "Context")
        .map(|section| section.trim().to_string())
        .filter(|section| !section.is_empty() && section != "Not provided.");

    TaskArtifactSpec { requirements, acceptance_criteria, context, result: None }
}

fn markdown_section(markdown: &str, heading: &str) -> Option<String> {
    let mut capture = false;
    let mut lines = Vec::new();
    for line in markdown.lines() {
        if let Some(current_heading) = line.trim().strip_prefix("## ") {
            if capture {
                break;
            }
            capture = current_heading.trim().eq_ignore_ascii_case(heading);
            continue;
        }
        if capture {
            lines.push(line);
        }
    }
    let section = lines.join("\n").trim().to_string();
    (!section.is_empty()).then_some(section)
}

fn parse_acceptance_criteria(section: &str) -> Vec<String> {
    section
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let item = trimmed.strip_prefix("- ").unwrap_or(trimmed).trim();
            (!item.is_empty()).then(|| item.to_string())
        })
        .collect()
}

fn imported_metadata(spec: &TaskArtifactSpec) -> Value {
    let mut metadata = Map::new();
    metadata.insert("requirements".to_string(), Value::String(spec.requirements.clone()));
    metadata.insert(
        "acceptance_criteria".to_string(),
        Value::Array(spec.acceptance_criteria.iter().cloned().map(Value::String).collect()),
    );
    if let Some(context) = &spec.context {
        metadata.insert("context".to_string(), Value::String(context.clone()));
    }
    Value::Object(metadata)
}

fn title_from_spec(markdown: &str, task_id: &str) -> String {
    markdown
        .lines()
        .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
        .unwrap_or(task_id)
        .to_string()
}

fn legacy_task_id(task_dir: &Path, meta: &LegacyTaskMeta) -> Result<String> {
    if let Some(task_id) = &meta.task_id {
        return Ok(task_id.clone());
    }
    task_dir.file_name().and_then(|name| name.to_str()).map(str::to_string).with_context(|| {
        format!("infer task id from legacy task directory: {}", task_dir.display())
    })
}

fn metadata_string(metadata: &Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn metadata_string_array(metadata: &Value, key: &str) -> Result<Vec<String>> {
    let Some(value) = metadata.get(key) else {
        return Ok(Vec::new());
    };
    let Value::Array(items) = value else {
        bail!("task metadata field must be a string array: {key}");
    };
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .with_context(|| format!("task metadata field contains a non-string item: {key}"))
        })
        .collect()
}

fn parse_created_at_ms(created_at: Option<&str>) -> Result<TimestampMs> {
    let Some(created_at) = created_at else {
        return Ok(TimestampMs(0));
    };
    let timestamp = DateTime::parse_from_rfc3339(created_at)
        .with_context(|| format!("parse legacy task created_at: {created_at}"))?;
    Ok(TimestampMs(timestamp.timestamp_millis()))
}

fn timestamp_ms_to_rfc3339(timestamp: TimestampMs) -> Result<String> {
    let seconds = timestamp.0.div_euclid(1_000);
    let nanos = timestamp.0.rem_euclid(1_000) as u32 * 1_000_000;
    let datetime = Utc
        .timestamp_opt(seconds, nanos)
        .single()
        .with_context(|| format!("convert timestamp ms to RFC3339: {}", timestamp.0))?;
    Ok(datetime.to_rfc3339())
}

fn artifact_path(artifact_root: &Path, relative_uri: &str) -> Result<PathBuf> {
    let relative_path = Path::new(relative_uri);
    if relative_uri.trim().is_empty() || relative_path.is_absolute() {
        bail!("artifact uri must be a non-empty relative path: {relative_uri}");
    }
    for component in relative_path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("artifact uri must stay under artifact root: {relative_uri}");
            }
        }
    }
    Ok(artifact_root.join(relative_path))
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)
        .with_context(|| format!("serialize artifact json: {}", path.display()))?;
    write_file(path, &json)
}

fn write_markdown_file(path: &Path, content: &str) -> Result<()> {
    write_file(path, content)
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create artifact directory: {}", parent.display()))?;
    }
    let content = ensure_trailing_newline(content);
    std::fs::write(path, content)
        .with_context(|| format!("write agent scheduler artifact: {}", path.display()))
}

fn ensure_trailing_newline(content: &str) -> String {
    if content.ends_with('\n') { content.to_string() } else { format!("{content}\n") }
}
