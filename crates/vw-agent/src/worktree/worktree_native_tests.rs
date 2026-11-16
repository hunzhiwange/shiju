use super::native::run_start_scripts;
use crate::project;

#[tokio::test]
async fn run_start_scripts_runs_extra_when_project_lookup_is_unavailable() {
    super::initialize_host();
    let dir = tempfile::tempdir().expect("worktree dir");

    run_start_scripts(&dir.path().to_string_lossy(), " \n ", "echo extra > extra.txt")
        .await
        .expect("run extra script");

    assert!(dir.path().join("extra.txt").exists());
}
#[tokio::test]
async fn run_start_scripts_runs_project_start_before_extra_script() {
    super::initialize_host();
    let dir = tempfile::tempdir().expect("worktree dir");
    let mut project = project_info(&dir.path().to_string_lossy());
    project.id = unique_project_id("worktree-start-success");
    project.commands = Some(project::Commands { start: Some("echo start > started.txt".into()) });
    project::store_local_test_project(&project).expect("store project");

    run_start_scripts(
        &dir.path().to_string_lossy(),
        &project.id,
        "test -f started.txt && echo extra > extra.txt",
    )
    .await
    .expect("run start scripts");

    assert!(dir.path().join("started.txt").exists());
    assert!(dir.path().join("extra.txt").exists());
    project::remove_local_test_project(&project.id).expect("remove project");
}
#[tokio::test]
async fn run_start_scripts_stops_when_project_start_fails() {
    super::initialize_host();
    let dir = tempfile::tempdir().expect("worktree dir");
    let mut project = project_info(&dir.path().to_string_lossy());
    project.id = unique_project_id("worktree-start-failure");
    project.commands = Some(project::Commands { start: Some(failing_shell_command().to_string()) });
    project::store_local_test_project(&project).expect("store project");

    run_start_scripts(&dir.path().to_string_lossy(), &project.id, "echo extra > extra.txt")
        .await
        .expect_err("failed project start must propagate");

    assert!(!dir.path().join("extra.txt").exists());
    project::remove_local_test_project(&project.id).expect("remove project");
}

fn project_info(worktree: &str) -> project::Info {
    project::Info {
        id: "project-1".to_string(),
        worktree: worktree.to_string(),
        vcs: Some(project::Vcs::Git),
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    }
}

fn unique_project_id(prefix: &str) -> String {
    format!("{}-{}", prefix, uuid::Uuid::new_v4())
}

fn failing_shell_command() -> &'static str {
    if cfg!(windows) { "exit /b 7" } else { "exit 7" }
}
