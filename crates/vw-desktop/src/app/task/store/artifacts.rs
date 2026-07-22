#[cfg(any(target_arch = "wasm32", test))]
use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};

use crate::app::task::models::Task;

#[cfg(target_arch = "wasm32")]
use super::paths::get_task_log_dir;
#[cfg(not(target_arch = "wasm32"))]
use super::paths::native_cutover_error;

#[cfg(any(target_arch = "wasm32", test))]
fn stored_task_executor_name(task: &Task) -> String {
    task.acp_agent.clone().unwrap_or_else(|| "acp".to_string())
}

#[cfg(any(target_arch = "wasm32", test))]
fn execution_result_content(task: &Task, result: &Result<String, String>) -> String {
    let mut content = String::new();
    let _ = writeln!(content, "task_id={}", task.id);
    let _ = writeln!(content, "acp_agent={}", stored_task_executor_name(task));
    let _ = writeln!(content, "model={}", task.model);
    let _ = writeln!(content, "prompt:\n{}", task.prompt);
    match result {
        Ok(output) => {
            let _ = writeln!(content, "result=success\noutput:\n{output}");
        }
        Err(error) => {
            let _ = writeln!(content, "result=error\nerror:\n{error}");
        }
    }
    content
}

#[cfg(any(target_arch = "wasm32", test))]
fn review_result_content(
    task: &Task,
    result: &Result<String, String>,
    full_system_prompt: Option<&str>,
) -> String {
    let mut content = String::new();
    let _ = writeln!(content, "task_id={}", task.id);
    let _ = writeln!(content, "acp_agent={}", stored_task_executor_name(task));
    let _ = writeln!(content, "model={}", task.model);
    if let Some(prompt) = full_system_prompt {
        let _ = writeln!(content, "review_system_prompt_full:\n{prompt}");
    }
    match result {
        Ok(output) => {
            let _ = writeln!(content, "review_result=success\nreview_output:\n{output}");
        }
        Err(error) => {
            let _ = writeln!(content, "review_result=error\nreview_error:\n{error}");
        }
    }
    content
}

pub fn write_task_execution_result_log(
    project_path: &str,
    task: &Task,
    result: &Result<String, String>,
) -> io::Result<PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (project_path, task, result);
        Err(native_cutover_error())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let directory = get_task_log_dir(project_path);
        std::fs::create_dir_all(&directory)?;
        let path = directory.join(format!("[{}].log", task.id));
        std::fs::write(&path, execution_result_content(task, result))?;
        Ok(path)
    }
}

pub fn write_task_code_review_result_log(
    project_path: &str,
    task: &Task,
    result: &Result<String, String>,
    full_system_prompt: Option<&str>,
) -> io::Result<PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (project_path, task, result, full_system_prompt);
        Err(native_cutover_error())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let directory = get_task_log_dir(project_path);
        std::fs::create_dir_all(&directory)?;
        let path = directory.join(format!("[{}].review.log", task.id));
        std::fs::write(&path, review_result_content(task, result, full_system_prompt))?;
        Ok(path)
    }
}

pub fn save_task_execution_result_artifact(
    project_path: &str,
    task: &Task,
    result: &Result<String, String>,
    file_path: Option<&Path>,
) -> io::Result<()> {
    let _ = (project_path, task, result, file_path);
    #[cfg(not(target_arch = "wasm32"))]
    return Err(native_cutover_error());
    #[cfg(target_arch = "wasm32")]
    Ok(())
}

pub fn save_task_code_review_result_artifact(
    project_path: &str,
    task: &Task,
    result: &Result<String, String>,
    full_system_prompt: Option<&str>,
    file_path: Option<&Path>,
) -> io::Result<()> {
    let _ = (project_path, task, result, full_system_prompt, file_path);
    #[cfg(not(target_arch = "wasm32"))]
    return Err(native_cutover_error());
    #[cfg(target_arch = "wasm32")]
    Ok(())
}

#[cfg(test)]
#[path = "artifacts_tests.rs"]
mod artifacts_tests;
