use crate::WorkflowRuntime;
use serde_json::json;

async fn action_graph<F: Fn(&std::path::Path, Vec<&str>) -> (WorkflowRuntime, tempfile::TempDir)>(
    factory: &F,
    root: &std::path::Path,
    actions: Vec<serde_json::Value>,
    allowed: Vec<&str>,
) -> vw_api_workflow::WorkflowRunResponse {
    let mut nodes = vec![json!({"id":"start","data":{"type":"start"}})];
    let mut edges = vec![];
    let mut previous = "start".to_string();
    for (index, data) in actions.into_iter().enumerate() {
        let id = format!("action{index}");
        edges.push(json!({"source":previous,"target":id}));
        nodes.push(json!({"id":id,"data":data}));
        previous = id;
    }
    nodes.push(json!({"id":"answer","data":{"type":"answer","answer":"done"}}));
    edges.push(json!({"source":previous,"target":"answer"}));
    let (runtime, _config_dir) = factory(root, allowed);
    crate::run_workflow(
        runtime,
        vw_api_workflow::WorkflowRunRequest {
            workflow_yaml: Some(
                json!({"workflow":{"graph":{"nodes":nodes,"edges":edges}}}).to_string(),
            ),
            max_steps: 20,
            ..Default::default()
        },
    )
    .await
    .unwrap()
}

fn action_repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Fixture").unwrap();
    config.set_str("user.email", "fixture@example.invalid").unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[]).unwrap();
    std::fs::write(dir.path().join("output.txt"), "result").unwrap();
    dir
}

pub async fn task_pool_actions_absent_steps_have_no_side_effects<F: Fn(&std::path::Path, Vec<&str>) -> (WorkflowRuntime, tempfile::TempDir)>(factory: F) {
    let dir = action_repository();
    let repo = git2::Repository::open(dir.path()).unwrap();
    let before = repo.head().unwrap().target();
    let response = action_graph(&factory, dir.path(), vec![], vec![]).await;
    assert_eq!(response.status, vw_api_workflow::WorkflowRunStatus::Succeeded);
    assert_eq!(repo.head().unwrap().target(), before);
    assert!(repo.index().unwrap().is_empty());
    assert_eq!(std::fs::read_to_string(dir.path().join("output.txt")).unwrap(), "result");
    // The same graph runs in an ordinary directory without creating Git resources.
    let ordinary = tempfile::tempdir().unwrap();
    assert_eq!(action_graph(&factory, ordinary.path(), vec![], vec![]).await.status, response.status);
    assert!(!ordinary.path().join(".git").exists());
}

pub async fn task_pool_actions_explicit_git_steps_use_native_tool_permissions<F: Fn(&std::path::Path, Vec<&str>) -> (WorkflowRuntime, tempfile::TempDir)>(factory: F) {
    let dir = action_repository();
    let repo = git2::Repository::open(dir.path()).unwrap();
    let before = repo.head().unwrap().target();
    let base = repo.head().unwrap().peel_to_commit().unwrap();
    let mut builder = repo.treebuilder(Some(&base.tree().unwrap())).unwrap();
    builder.insert("source.txt", repo.blob(b"source").unwrap(), 0o100644).unwrap();
    let tree = repo.find_tree(builder.write().unwrap()).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(Some("refs/heads/feature"), &signature, &signature, "source", &tree, &[&base])
        .unwrap();
    let target = repo.head().unwrap().shorthand().unwrap().to_string();
    let steps = vec![
        json!({"type":"tool","provider":"shiju","tool_name":"git_operations","inputs":{"operation":"add","paths":"output.txt"}}),
        json!({"type":"tool","provider":"shiju","tool_name":"git_operations","inputs":{"operation":"commit","message":"explicit"}}),
        json!({"type":"tool","provider":"shiju","tool_name":"git_operations","inputs":{"operation":"merge","source":"feature","target":target}}),
    ];
    let denied = action_graph(&factory, dir.path(), steps.clone(), vec![]).await;
    assert_eq!(denied.status, vw_api_workflow::WorkflowRunStatus::Failed);
    assert_eq!(repo.head().unwrap().target(), before);
    let allowed = action_graph(&factory, dir.path(), steps, vec!["git_operations"]).await;
    assert_eq!(allowed.status, vw_api_workflow::WorkflowRunStatus::Succeeded, "{allowed:?}");
    assert_ne!(repo.head().unwrap().target(), before);
    let merged = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(merged.parent_count(), 2);
    assert!(merged.tree().unwrap().get_name("source.txt").is_some());
    assert!(merged.tree().unwrap().get_name("output.txt").is_some());
}

pub async fn task_pool_actions_upstream_failure_skips_downstream_send<F: Fn(&std::path::Path, Vec<&str>) -> (WorkflowRuntime, tempfile::TempDir)>(factory: F) {
    let dir = tempfile::tempdir().unwrap();
    let response = action_graph(&factory, dir.path(), vec![
        json!({"type":"tool","provider":"shiju","tool_name":"git_operations","inputs":{"operation":"commit","message":"non git"}}),
        json!({"type":"tool","provider":"shiju","tool_name":"send_message_to_thread","inputs":{"thread_id":"unavailable","prompt":"never send"}}),
    ], vec!["git_operations","send_message_to_thread"]).await;
    assert_eq!(response.status, vw_api_workflow::WorkflowRunStatus::Failed);
    assert!(!response.nodes.iter().any(|node| node.node_id == "action1"));
}
