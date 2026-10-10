use super::{
    CreateThreadTool, ForkThreadTool, HandoffThreadTool, ListThreadsTool, ReadThreadTool,
    SendMessageToThreadTool, SetThreadArchivedTool, SetThreadPinnedTool, SetThreadTitleTool,
    WaitThreadsTool,
};
use crate::app::agent::tools::traits::Tool;
use crate::session::runtime::{CreateSessionInput, SessionScope, create_session, load_session};
use crate::session::task_session::TaskSessionCoordinator;
use futures_util::FutureExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tempfile::TempDir;
use vw_api_types::id::TenantScope;

#[tokio::test]
async fn handoff_thread_rejects_unsupported_transfer() {
    let project = TempDir::new().expect("project");
    let scope = SessionScope::new(TenantScope::default(), project.path()).expect("scope");
    let parent = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Parent".into()),
        permission: None,
    })
    .await
    .expect("parent");
    let child = crate::session::runtime::create_visible_task_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: Some(parent.info.id.clone()),
        title: Some("Task".into()),
        permission: None,
    })
    .await
    .expect("child");
    let tool = HandoffThreadTool::new(Some(parent.info.id.clone()), project.path().to_path_buf());
    assert_eq!(tool.name(), "handoff_thread");
    assert_eq!(tool.parameters_schema()["required"], json!(["thread_id"]));
    let error =
        tool.execute(json!({"thread_id": child.info.id})).await.expect_err("transfer unsupported");
    assert!(error.to_string().contains("handoff_thread is unsupported"));
    assert!(
        tool.execute(json!({"thread_id": child.info.id, "destination_host_id": "remote"}))
            .await
            .is_err()
    );
    assert!(tool.execute(json!({"thread_id": ""})).await.is_err());
    assert_eq!(
        load_session(scope.clone(), &child.info.id).await.expect("unchanged").info.parent_id,
        child.info.parent_id
    );
    assert_eq!(
        crate::session::runtime::visible_tasks_for_parent(scope, &parent.info.id)
            .await
            .expect("tasks")
            .len(),
        1
    );
    let context = super::super::toolset::ToolRuntimeContext::new(
        parent.info.id,
        Some(project.path().to_string_lossy().into_owned()),
    );
    assert!(
        super::super::toolset::tool_specs_for_context(&context)
            .iter()
            .any(|spec| spec.name == "handoff_thread")
    );
}

#[tokio::test]
async fn set_thread_pinned_round_trip_and_rejects_foreign_child() {
    let project = TempDir::new().expect("project");
    let scope = SessionScope::new(TenantScope::default(), project.path()).expect("scope");
    let parent = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Parent".into()),
        permission: None,
    })
    .await
    .expect("parent");
    let other = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Other".into()),
        permission: None,
    })
    .await
    .expect("other");
    let child = crate::session::runtime::create_visible_task_session(CreateSessionInput {
        scope,
        parent_id: Some(parent.info.id.clone()),
        title: Some("Task".into()),
        permission: None,
    })
    .await
    .expect("child");
    let tool = SetThreadPinnedTool::new(Some(parent.info.id.clone()), project.path().to_path_buf());
    let list = ListThreadsTool::new(Some(parent.info.id.clone()), project.path().to_path_buf());
    assert_eq!(tool.name(), "set_thread_pinned");
    assert_eq!(
        serde_json::from_str::<Value>(
            &tool
                .execute(json!({"thread_id":child.info.id,"pinned":true}))
                .await
                .expect("pin")
                .output
        )
        .expect("json")["pinned"],
        true
    );
    assert_eq!(
        serde_json::from_str::<Value>(&list.execute(json!({})).await.expect("list").output)
            .expect("json")["threads"][0]["pinned"],
        true
    );
    assert_eq!(
        serde_json::from_str::<Value>(
            &tool
                .execute(json!({"thread_id":child.info.id,"pinned":false}))
                .await
                .expect("unpin")
                .output
        )
        .expect("json")["pinned"],
        false
    );
    assert_eq!(
        serde_json::from_str::<Value>(&list.execute(json!({})).await.expect("list").output)
            .expect("json")["threads"][0]["pinned"],
        false
    );
    let foreign = SetThreadPinnedTool::new(Some(other.info.id), project.path().to_path_buf());
    assert!(foreign.execute(json!({"thread_id":child.info.id,"pinned":true})).await.is_err());
    assert!(tool.execute(json!({"thread_id":child.info.id,"pinned":"true"})).await.is_err());
    assert!(
        tool.execute(json!({"thread_id":child.info.id,"pinned":true,"project_id":"other"}))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn set_thread_archived_round_trip_in_project() {
    let project = TempDir::new().expect("project");
    let scope = SessionScope::new(TenantScope::default(), project.path()).expect("scope");
    let parent = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Parent".into()),
        permission: None,
    })
    .await
    .expect("parent");
    let other = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Other".into()),
        permission: None,
    })
    .await
    .expect("other");
    let child = crate::session::runtime::create_visible_task_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: Some(parent.info.id.clone()),
        title: Some("Task".into()),
        permission: None,
    })
    .await
    .expect("child");
    let tool =
        SetThreadArchivedTool::new(Some(parent.info.id.clone()), project.path().to_path_buf());
    let list = ListThreadsTool::new(Some(parent.info.id.clone()), project.path().to_path_buf());
    assert_eq!(tool.name(), "set_thread_archived");
    let archived =
        tool.execute(json!({"thread_id":child.info.id,"archived":true})).await.expect("archive");
    assert_eq!(serde_json::from_str::<Value>(&archived.output).expect("json")["archived"], true);
    assert!(
        load_session(scope.clone(), &child.info.id)
            .await
            .expect("stored")
            .info
            .time
            .archived
            .is_some()
    );
    assert!(
        serde_json::from_str::<Value>(&list.execute(json!({})).await.expect("list").output)
            .expect("json")["threads"]
            .as_array()
            .expect("threads")
            .is_empty()
    );
    let restored =
        tool.execute(json!({"thread_id":child.info.id,"archived":false})).await.expect("restore");
    assert_eq!(serde_json::from_str::<Value>(&restored.output).expect("json")["archived"], false);
    assert!(
        load_session(scope, &child.info.id).await.expect("stored").info.time.archived.is_none()
    );
    assert_eq!(
        serde_json::from_str::<Value>(&list.execute(json!({})).await.expect("list").output)
            .expect("json")["threads"]
            .as_array()
            .expect("threads")
            .len(),
        1
    );
    let foreign = SetThreadArchivedTool::new(Some(other.info.id), project.path().to_path_buf());
    assert!(foreign.execute(json!({"thread_id":child.info.id,"archived":true})).await.is_err());
    assert!(tool.execute(json!({"thread_id":child.info.id,"archived":"true"})).await.is_err());
    assert!(
        tool.execute(json!({"thread_id":child.info.id,"archived":true,"model":"x"})).await.is_err()
    );
}

#[tokio::test]
async fn set_thread_title_updates_owned_visible_task() {
    let project = TempDir::new().expect("project");
    let scope = SessionScope::new(TenantScope::default(), project.path()).expect("scope");
    let parent = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Parent".into()),
        permission: None,
    })
    .await
    .expect("parent");
    let other_parent = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Other".into()),
        permission: None,
    })
    .await
    .expect("other");
    let child = crate::session::runtime::create_visible_task_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: Some(parent.info.id.clone()),
        title: Some("Old".into()),
        permission: None,
    })
    .await
    .expect("child");
    let tool = SetThreadTitleTool::new(Some(parent.info.id.clone()), project.path().to_path_buf());
    assert_eq!(tool.name(), "set_thread_title");
    let result =
        tool.execute(json!({"thread_id": child.info.id, "title": "New"})).await.expect("rename");
    let output: Value = serde_json::from_str(&result.output).expect("output");
    assert_eq!(output["thread_id"], child.info.id);
    assert_eq!(output["title"], "New");
    assert_eq!(load_session(scope, &child.info.id).await.expect("stored").info.title, "New");
    let foreign = SetThreadTitleTool::new(Some(other_parent.info.id), project.path().to_path_buf());
    assert!(foreign.execute(json!({"thread_id": child.info.id, "title": "Wrong"})).await.is_err());
    assert!(tool.execute(json!({"thread_id": child.info.id, "title": ""})).await.is_err());
    assert!(
        tool.execute(json!({"thread_id": child.info.id, "title": "Wrong", "model": "x"}))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn fork_thread_keeps_project_and_visible_identity() {
    let project = TempDir::new().expect("project");
    let scope = SessionScope::new(TenantScope::default(), project.path()).expect("scope");
    let parent = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Parent".into()),
        permission: None,
    })
    .await
    .expect("parent");
    let source = crate::session::runtime::create_visible_task_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: Some(parent.info.id.clone()),
        title: Some("Source".into()),
        permission: None,
    })
    .await
    .expect("source");
    let tool = ForkThreadTool::new(Some(parent.info.id.clone()), project.path().to_path_buf());
    assert_eq!(tool.name(), "fork_thread");
    let result = tool.execute(json!({"thread_id": source.info.id})).await.expect("fork");
    let output: Value = serde_json::from_str(&result.output).expect("output");
    assert_eq!(output["project_id"], parent.info.project_id);
    let fork = load_session(scope, output["thread_id"].as_str().expect("thread id"))
        .await
        .expect("forked thread");
    assert_eq!(fork.info.parent_id.as_deref(), Some(parent.info.id.as_str()));
    assert!(fork.info.is_visible_task());
    assert_eq!(fork.info.project_id, source.info.project_id);
    assert!(tool.execute(json!({"thread_id": parent.info.id})).await.is_err());
    assert!(
        tool.execute(json!({"thread_id": source.info.id, "before_turn_id": "unsupported"}))
            .await
            .is_err()
    );
    assert!(
        tool.execute(json!({"thread_id": source.info.id, "message_id": "missing"})).await.is_err()
    );
}

#[tokio::test]
async fn create_thread_inherits_verified_current_project() {
    let project = TempDir::new().expect("project directory");
    let scope = SessionScope::new(TenantScope::default(), project.path()).expect("scope");
    let parent = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Parent".into()),
        permission: None,
    })
    .await
    .expect("parent");
    let executor = Arc::new(|_: String, _: vw_api_types::chat::GatewayChatStreamRequest| {
        async {
            Ok(crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected {
                output: String::new(),
                message_id: None,
                parent_message_id: None,
            })
        }
        .boxed()
    });
    let tool = CreateThreadTool::new(Some(parent.info.id.clone()), project.path().to_path_buf())
        .with_coordinator(TaskSessionCoordinator::new())
        .with_stream_executor(executor);
    assert_eq!(tool.name(), "create_thread");
    let result = tool.execute(json!({"prompt":"Do the work"})).await.expect("create");
    let output: Value = serde_json::from_str(&result.output).expect("output");
    assert_eq!(output["project_id"], parent.info.project_id);
    assert_eq!(output["directory"], parent.info.directory);
    let thread_id = output["thread_id"].as_str().expect("thread id");
    let child = load_session(scope, thread_id).await.expect("child");
    assert!(child.info.is_visible_task());
    assert_eq!(child.info.parent_id.as_deref(), Some(parent.info.id.as_str()));
    assert!(tool.execute(json!({"prompt":"x","project_id":"other"})).await.is_err());
    assert!(tool.execute(json!({"prompt":"x","model":"unsupported"})).await.is_err());
}

#[tokio::test]
async fn thread_query_tools_scope_visible_children() {
    let project = TempDir::new().expect("project");
    let scope = SessionScope::new(TenantScope::default(), project.path()).expect("scope");
    let parent = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Parent".into()),
        permission: None,
    })
    .await
    .expect("parent");
    let child = crate::session::runtime::create_visible_task_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: Some(parent.info.id.clone()),
        title: Some("Visible".into()),
        permission: None,
    })
    .await
    .expect("child");
    let other = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Other".into()),
        permission: None,
    })
    .await
    .expect("other");
    let directory = project.path().to_path_buf();
    let list = ListThreadsTool::new(Some(parent.info.id.clone()), directory.clone());
    let read = ReadThreadTool::new(Some(parent.info.id.clone()), directory.clone());
    let wait = WaitThreadsTool::new(Some(parent.info.id.clone()), directory);
    assert_eq!(
        (list.name(), read.name(), wait.name()),
        ("list_threads", "read_thread", "wait_threads")
    );
    let output: Value =
        serde_json::from_str(&list.execute(json!({})).await.expect("list").output).expect("json");
    assert_eq!(output["threads"].as_array().expect("threads").len(), 1);
    assert_eq!(output["threads"][0]["thread_id"], child.info.id);
    assert_eq!(output["threads"][0]["project_id"], parent.info.project_id);
    assert!(read.execute(json!({"thread_id": child.info.id})).await.is_ok());
    assert!(read.execute(json!({"thread_id": other.info.id})).await.is_err());
    assert!(wait.execute(json!({"thread_ids": [other.info.id], "timeout_ms": 0})).await.is_err());
    assert!(list.execute(json!({"cursor":"unsupported"})).await.is_err());
}

#[tokio::test]
async fn send_message_to_thread_rejects_busy_and_foreign_task() {
    let project = TempDir::new().expect("project");
    let scope = SessionScope::new(TenantScope::default(), project.path()).expect("scope");
    let parent = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Parent".into()),
        permission: None,
    })
    .await
    .expect("parent");
    let other_parent = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Other".into()),
        permission: None,
    })
    .await
    .expect("other parent");
    let busy = crate::session::runtime::create_visible_task_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: Some(parent.info.id.clone()),
        title: Some("Busy".into()),
        permission: None,
    })
    .await
    .expect("busy task");
    let foreign = crate::session::runtime::create_visible_task_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: Some(other_parent.info.id.clone()),
        title: Some("Foreign".into()),
        permission: None,
    })
    .await
    .expect("foreign task");
    let ready = crate::session::runtime::create_visible_task_session(CreateSessionInput {
        scope,
        parent_id: Some(parent.info.id.clone()),
        title: Some("Ready".into()),
        permission: None,
    })
    .await
    .expect("ready task");
    let coordinator = TaskSessionCoordinator::new();
    for id in [&busy.info.id, &ready.info.id] {
        coordinator.register(&parent.info.id, id).expect("register");
        coordinator.mark_running(&parent.info.id, id).expect("running");
    }
    coordinator.mark_succeeded(&parent.info.id, &ready.info.id).expect("ready");
    for id in [&busy.info.id, &ready.info.id] {
        let state =
            coordinator.snapshots(&parent.info.id, &[id.clone()]).expect("snapshot").remove(0);
        crate::session::ui_store::save_visible_task_state(
            &TenantScope::default(),
            &parent.info.project_id,
            &state,
        )
        .expect("persist state");
    }
    let executor = Arc::new(|_: String, _: vw_api_types::chat::GatewayChatStreamRequest| {
        async {
            Ok(crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected {
                output: String::new(),
                message_id: None,
                parent_message_id: None,
            })
        }
        .boxed()
    });
    let tool =
        SendMessageToThreadTool::new(Some(parent.info.id.clone()), project.path().to_path_buf())
            .with_coordinator(coordinator)
            .with_stream_executor(executor);
    assert_eq!(tool.name(), "send_message_to_thread");
    let foreign_error = tool
        .execute(json!({"thread_id": foreign.info.id, "prompt":"Continue"}))
        .await
        .expect_err("foreign task rejected");
    assert!(foreign_error.to_string().contains("visible child"));
    let busy_error = tool
        .execute(json!({"thread_id": busy.info.id, "prompt":"Continue"}))
        .await
        .expect_err("busy task rejected");
    assert!(busy_error.to_string().contains("busy"));
    let sent = tool
        .execute(json!({"thread_id": ready.info.id, "prompt":"Continue"}))
        .await
        .expect("send ready task");
    let output: Value = serde_json::from_str(&sent.output).expect("output");
    assert_eq!(output["thread_id"], ready.info.id);
    assert_eq!(output["state"], "running");
    assert!(
        tool.execute(json!({"thread_id": ready.info.id, "prompt":"x", "model":"unsupported"}))
            .await
            .is_err()
    );
}
