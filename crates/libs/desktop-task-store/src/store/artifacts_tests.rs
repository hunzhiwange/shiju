//! 任务产物测试，覆盖执行与评审内容格式、执行器标识和原生切换后的拒绝路径。
//! 已停用的本地写入必须失败关闭，并保证磁盘上不产生半成品日志或产物。

use super::*;

fn task_fixture(acp_agent: Option<&str>) -> Task {
    let mut task = Task::new(1);
    task.id = "TASK-artifact".to_string();
    task.acp_agent = acp_agent.map(str::to_string);
    task.model = "test-model".to_string();
    task.prompt = "build the artifact".to_string();
    task
}

fn assert_native_cutover(error: io::Error) {
    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert_eq!(
        error.to_string(),
        "native desktop task persistence moved to the global Agent Scheduler"
    );
}

#[test]
fn stored_task_executor_name_defaults_to_acp_and_preserves_selected_agent() {
    assert_eq!(stored_task_executor_name(&task_fixture(None)), "acp");
    assert_eq!(stored_task_executor_name(&task_fixture(Some("codex"))), "codex");
}

#[test]
fn execution_result_content_formats_success_context() {
    let content =
        execution_result_content(&task_fixture(Some("codex")), &Ok("artifact created".to_string()));

    assert_eq!(
        content,
        concat!(
            "task_id=TASK-artifact\n",
            "acp_agent=codex\n",
            "model=test-model\n",
            "prompt:\n",
            "build the artifact\n",
            "result=success\n",
            "output:\n",
            "artifact created\n",
        )
    );
}

#[test]
fn execution_result_content_formats_error_with_default_executor() {
    let content = execution_result_content(&task_fixture(None), &Err("disk full".to_string()));

    assert_eq!(
        content,
        concat!(
            "task_id=TASK-artifact\n",
            "acp_agent=acp\n",
            "model=test-model\n",
            "prompt:\n",
            "build the artifact\n",
            "result=error\n",
            "error:\n",
            "disk full\n",
        )
    );
}

#[test]
fn review_result_content_includes_full_prompt_for_success() {
    let content = review_result_content(
        &task_fixture(Some("reviewer")),
        &Ok("approved".to_string()),
        Some("review every changed line"),
    );

    assert_eq!(
        content,
        concat!(
            "task_id=TASK-artifact\n",
            "acp_agent=reviewer\n",
            "model=test-model\n",
            "review_system_prompt_full:\n",
            "review every changed line\n",
            "review_result=success\n",
            "review_output:\n",
            "approved\n",
        )
    );
}

#[test]
fn review_result_content_omits_prompt_and_formats_error() {
    let content =
        review_result_content(&task_fixture(None), &Err("changes requested".to_string()), None);

    assert_eq!(
        content,
        concat!(
            "task_id=TASK-artifact\n",
            "acp_agent=acp\n",
            "model=test-model\n",
            "review_result=error\n",
            "review_error:\n",
            "changes requested\n",
        )
    );
    assert!(!content.contains("review_system_prompt_full"));
}

#[test]
fn native_execution_log_write_fails_closed_without_creating_files() {
    let project = tempfile::tempdir().expect("temp project");
    let path = project.path().to_string_lossy();
    let task = Task::new(1);
    let error = write_task_execution_result_log(&path, &task, &Ok("done".to_string()))
        .expect_err("native artifact file disabled");

    assert_native_cutover(error);
    assert_eq!(project.path().read_dir().expect("read project").count(), 0);
}

#[test]
fn native_review_log_write_fails_closed_without_creating_files() {
    let project = tempfile::tempdir().expect("temp project");
    let path = project.path().to_string_lossy();
    let task = Task::new(1);
    let error = write_task_code_review_result_log(
        &path,
        &task,
        &Err("changes requested".to_string()),
        Some("review prompt"),
    )
    .expect_err("native review artifact file disabled");

    assert_native_cutover(error);
    assert_eq!(project.path().read_dir().expect("read project").count(), 0);
}

#[test]
fn native_execution_artifact_save_fails_closed_without_creating_files() {
    let project = tempfile::tempdir().expect("temp project");
    let path = project.path().to_string_lossy();
    let artifact_path = project.path().join("artifact.txt");
    let task = Task::new(1);
    let error = save_task_execution_result_artifact(
        &path,
        &task,
        &Ok("done".to_string()),
        Some(&artifact_path),
    )
    .expect_err("native execution artifact save disabled");

    assert_native_cutover(error);
    assert!(!artifact_path.exists());
}

#[test]
fn native_review_artifact_save_fails_closed_without_creating_files() {
    let project = tempfile::tempdir().expect("temp project");
    let path = project.path().to_string_lossy();
    let artifact_path = project.path().join("review.txt");
    let task = Task::new(1);
    let error = save_task_code_review_result_artifact(
        &path,
        &task,
        &Err("changes requested".to_string()),
        Some("review prompt"),
        Some(&artifact_path),
    )
    .expect_err("native review artifact save disabled");

    assert_native_cutover(error);
    assert!(!artifact_path.exists());
}
