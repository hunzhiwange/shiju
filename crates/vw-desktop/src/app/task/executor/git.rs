//! 桌面配置到任务 Git 执行器的窄适配。
pub fn build_review_diff_context(
    project_path: &str,
    source_branch: Option<&str>,
    target_branch: Option<&str>,
) -> Result<(String, String, String, String, String), String> {
    desktop_task_executor::executor::git::build_review_diff_context(
        crate::app::config::gateway_client,
        project_path,
        source_branch,
        target_branch,
    )
}
