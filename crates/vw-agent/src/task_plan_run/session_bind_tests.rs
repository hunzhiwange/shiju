//! plan-run 子会话 Info 绑定与可渲染历史落盘测试。

use super::{
    PlanTaskEntry, PlanTasksDocument, SessionRunRequest, SessionRunner, run_task_plan,
    session_bind::{ensure_plan_run_session_info, persist_plan_run_session_history},
};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use vw_api_types::task::TaskPlanRunRequest;

#[derive(Clone)]
struct ScriptedSessionRunner {
    responses: Arc<Mutex<VecDeque<Result<String, String>>>>,
    plan_fixture: Arc<Mutex<Option<PlanFixture>>>,
}

#[derive(Clone)]
struct PlanFixture {
    plan_dir_rel: String,
    tasks: Vec<PlanTaskEntry>,
}

#[async_trait]
impl SessionRunner for ScriptedSessionRunner {
    async fn run_session(
        &self,
        request: SessionRunRequest,
        _delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String, String> {
        if let Some(fixture) = self.plan_fixture.lock().expect("fixture lock").take() {
            write_plan_fixture(&request.root, &fixture)?;
        }
        // 子任务成功后删除编号任务文件，使固定目标模式首轮验收可通过。
        if !request.session_id.ends_with("-plan")
            && !request.session_id.contains("-goal-")
        {
            let plan_root = PathBuf::from(&request.root).join(".plans");
            remove_task_mds(&plan_root);
        }
        self.responses
            .lock()
            .expect("responses lock")
            .pop_front()
            .unwrap_or_else(|| {
                if request.session_id.contains("-goal-eval-") {
                    Ok(r#"{"achieved":true,"reason":"验证通过"}"#.to_string())
                } else {
                    Err("scripted session runner: no more responses".to_string())
                }
            })
    }
}

fn remove_task_mds(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            remove_task_mds(&path);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("md")
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name != "plan.md")
        {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn write_plan_fixture(root: &str, fixture: &PlanFixture) -> Result<(), String> {
    let plan_abs = PathBuf::from(root).join(&fixture.plan_dir_rel);
    std::fs::create_dir_all(&plan_abs)
        .map_err(|error| format!("create plan dir failed: {error}"))?;

    let doc = PlanTasksDocument {
        plan_dir: fixture.plan_dir_rel.clone(),
        tasks: fixture.tasks.clone(),
    };
    let tasks_json = serde_json::to_string_pretty(&doc)
        .map_err(|error| format!("serialize tasks.json failed: {error}"))?;
    std::fs::write(plan_abs.join("tasks.json"), tasks_json)
        .map_err(|error| format!("write tasks.json failed: {error}"))?;
    std::fs::write(plan_abs.join("plan.md"), "# plan\n")
        .map_err(|error| format!("write plan.md failed: {error}"))?;
    std::fs::write(plan_abs.join("prompt.md"), "# prompt\n")
        .map_err(|error| format!("write prompt.md failed: {error}"))?;

    for task in &fixture.tasks {
        let path = plan_abs.join(&task.file);
        std::fs::write(&path, format!("# {}\n\nstatus: 未开始\n", task.title))
            .map_err(|error| format!("write {} failed: {error}", path.display()))?;
    }
    Ok(())
}

fn entry(id: &str, title: &str, file: &str, batch: u32) -> PlanTaskEntry {
    PlanTaskEntry {
        id: id.to_string(),
        title: title.to_string(),
        file: file.to_string(),
        batch,
        depends_on_batches: Vec::new(),
        status: "未开始".to_string(),
    }
}

#[tokio::test]
async fn plan_run_binds_child_session_info_with_parent() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let session_id = "bind-demo-plan";
    let parent_id = "main-chat-session";

    ensure_plan_run_session_info(session_id, Some(parent_id), &root, "规划 · bind-demo")
        .await
        .expect("ensure info should succeed");

    let info = crate::session::ui_store::load_agent_session_any(session_id)
        .expect("Info should be readable from store");
    assert_eq!(info.id, session_id);
    assert_eq!(info.parent_id.as_deref(), Some(parent_id));
    assert_eq!(info.directory, root);
    assert!(info.title.contains("规划"), "title should mention 规划: {}", info.title);
    assert!(info.is_plan_run());
    let created = info.time.created;
    let slug = info.slug.clone();

    ensure_plan_run_session_info(session_id, Some(parent_id), &root, "规划 · bind-demo")
        .await
        .expect("repeated ensure info should succeed");

    let rebound = crate::session::ui_store::load_agent_session_any(session_id)
        .expect("rebound Info should be readable from store");
    assert_eq!(rebound.time.created, created);
    assert_eq!(rebound.slug, slug);
    assert_eq!(rebound.parent_id.as_deref(), Some(parent_id));
    assert_eq!(rebound.title, "规划 · bind-demo");
    assert!(rebound.is_plan_run());

    persist_plan_run_session_history(
        session_id,
        &root,
        "规划 · bind-demo",
        "user prompt",
        "assistant reply",
    )
    .await
    .expect("history persist should succeed");

    let chat = crate::session::ui_store::load_session_any(session_id)
        .expect("ChatSession should be loadable");
    assert!(
        !chat.messages.is_empty() || !chat.calls.is_empty(),
        "messages or calls must be non-empty for desktop render"
    );
    assert!(
        chat.messages.len() >= 2,
        "expected user+assistant messages, got {}",
        chat.messages.len()
    );
    assert_eq!(chat.messages[0].content, "user prompt");
    assert_eq!(chat.messages[1].content, "assistant reply");
}

#[tokio::test]
async fn plan_run_binds_session_info_without_parent() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let session_id = "bind-orphan-plan";

    ensure_plan_run_session_info(session_id, None, &root, "规划 · orphan")
        .await
        .expect("ensure info without parent should still create Info");

    let info = crate::session::ui_store::load_agent_session_any(session_id)
        .expect("Info should exist even without parent_session_id");
    assert_eq!(info.id, session_id);
    assert_eq!(info.parent_id, None);
    assert!(info.title.contains("规划"));
}

#[tokio::test]
async fn plan_run_binds_and_persists_history_through_run_task_plan() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let task_id = "bind-flow";
    let plan_dir_rel = format!(".plans/task-{task_id}");
    let parent_id = "parent-main";

    let runner = ScriptedSessionRunner {
        responses: Arc::new(Mutex::new(VecDeque::from(vec![
            Ok("plan ready".to_string()),
            Ok("subtask done".to_string()),
        ]))),
        plan_fixture: Arc::new(Mutex::new(Some(PlanFixture {
            plan_dir_rel,
            tasks: vec![entry("1", "实现绑定", "1-bind.md", 1)],
        }))),
    };

    let request = TaskPlanRunRequest {
        query: "绑定子会话并落盘历史".to_string(),
        root: root.clone(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: Some(parent_id.to_string()),
    };

    run_task_plan(task_id, &request, &runner, |_| true)
        .await
        .expect("run_task_plan should succeed");

    let planning_id = format!("{task_id}-plan");
    let subtask_id = format!("{task_id}-1");

    let planning_info = crate::session::ui_store::load_agent_session_any(&planning_id)
        .expect("planning Info should exist");
    assert_eq!(planning_info.parent_id.as_deref(), Some(parent_id));
    assert!(planning_info.title.contains("规划"));

    let subtask_info = crate::session::ui_store::load_agent_session_any(&subtask_id)
        .expect("subtask Info should exist");
    assert_eq!(subtask_info.parent_id.as_deref(), Some(parent_id));
    assert!(subtask_info.title.contains("实现绑定"));

    let planning_chat = crate::session::ui_store::load_session_any(&planning_id)
        .expect("planning ChatSession should load");
    assert!(
        !planning_chat.messages.is_empty(),
        "planning messages must be non-empty for desktop"
    );

    let subtask_chat = crate::session::ui_store::load_session_any(&subtask_id)
        .expect("subtask ChatSession should load");
    assert!(
        !subtask_chat.messages.is_empty(),
        "subtask messages must be non-empty for desktop"
    );
}
