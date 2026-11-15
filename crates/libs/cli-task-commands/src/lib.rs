//! 本地项目任务的创建、读取与 JSON 输出。

use anyhow::{Context, Result, bail};
use cli_command_schema::{self as cli, TaskCommands};
use vw_agent::agent_scheduler::domain::TaskId;
use vw_agent::agent_scheduler::store::AgentSchedulerStore;
use vw_agent::agent_scheduler::task_board::{
    create_native_task, list_native_tasks, load_native_task,
};
use task_types::{SubTask, Task, TaskExecutorBackend, TaskSplitMode, TaskStatus};

/// 执行本地项目任务命令。
pub fn handle_command(project_dir: &str, task_command: TaskCommands) -> Result<()> {
    // 解析项目目录路径
    let project_path = resolve_project_dir(project_dir)?;
    match task_command {
        // 创建新任务
        TaskCommands::Create {
            priority,
            prompt,
            description,
            assignee,
            model,
            executor,
            task_split,
            subtasks,
        } => {
            // 清理和准备任务内容
            let prompt = prompt.map(|value| value.trim().to_string());
            let description = description.map(|value| value.trim().to_string());
            let cleaned_subtasks = subtasks
                .into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();

            // 确定任务的种子内容（从 prompt、description 或 subtasks 中获取第一个非空行）
            let mut task_seed = first_non_empty_line(prompt.as_deref().unwrap_or_default());
            if task_seed.is_empty() {
                task_seed =
                    first_non_empty_line(description.as_deref().unwrap_or_default());
            }
            if task_seed.is_empty() {
                task_seed = cleaned_subtasks.first().cloned().unwrap_or_default();
            }
            // 确保至少有一种内容来源
            if task_seed.is_empty() {
                bail!(
                    "task content is empty; provide at least one of --prompt, --description, or --subtask"
                );
            }

            // 创建任务对象并设置各个字段
            let mut task = Task::new(priority);

            if let Some(description) = description {
                task.description = description;
            }
            if let Some(assignee) = assignee {
                let assignee = assignee.trim();
                if !assignee.is_empty() {
                    task.assignee = assignee.to_string();
                }
            }
            if let Some(model) = model {
                let model = model.trim();
                if !model.is_empty() {
                    task.model = model.to_string();
                }
            }
            // 解析并设置执行器后端
            if let Some(executor) = executor {
                let parsed = TaskExecutorBackend::from_id(executor.trim())
                    .with_context(|| {
                        format!(
                            "Invalid --executor value: {} (supported: internal, opencode, claude, codex)",
                            executor
                        )
                    })?;
                task.executor = parsed;
            }
            if let Some(task_split) = task_split {
                task.task_split_mode = match task_split {
                    cli::TaskSplitCliMode::FollowSystem => TaskSplitMode::FollowSystem,
                    cli::TaskSplitCliMode::On => TaskSplitMode::Enabled,
                    cli::TaskSplitCliMode::Off => TaskSplitMode::Disabled,
                };
            }
            // 设置子任务列表
            task.subtasks = cleaned_subtasks.into_iter().map(SubTask::new).collect();

            // 设置任务的提示内容
            match prompt {
                Some(prompt) if !prompt.is_empty() => {
                    task.prompt = prompt;
                }
                _ => {
                    task.prompt = first_non_empty_line(&task.description);
                }
            }
            // 如果 prompt 仍为空，使用第一个子任务的内容
            if task.prompt.is_empty()
                && let Some(subtask) = task.subtasks.first()
            {
                task.prompt = subtask.content.clone();
            }

            // 创建任务并打印结果
            let scheduler = AgentSchedulerStore::new();
            let created =
                create_native_task(&scheduler, std::path::Path::new(&project_path), task)
                    .with_context(|| {
                    format!("Failed to create task in {}", project_path.as_str())
                })?;
            print_task_json(&created)
        }
        // 读取任务
        TaskCommands::Read { id, status, include_archived, include_deleted, limit } => {
            // 如果指定了任务 ID，直接加载该任务
            if let Some(task_id) = id {
                let scheduler = AgentSchedulerStore::new();
                let task_id_value = TaskId::new(task_id.trim())?;
                let task = load_native_task(
                    &scheduler,
                    std::path::Path::new(&project_path),
                    &task_id_value,
                )?
                .with_context(|| {
                    format!(
                        "Task not found in {} with id {}",
                        project_path.as_str(),
                        task_id
                    )
                })?;
                return print_task_json(&task);
            }

            // 解析状态过滤器
            let status_filter = status
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| {
                    TaskStatus::parse_key(value)
                        .with_context(|| format!("Invalid --status value: {value}"))
                })
                .transpose()?;

            // 加载所有任务并应用过滤器
            let scheduler = AgentSchedulerStore::new();
            let mut tasks =
                list_native_tasks(&scheduler, std::path::Path::new(&project_path))?;
            tasks.retain(|task| {
                let status_ok = status_filter.is_none_or(|s| task.status == s);
                let archived_ok = include_archived || !task.archived;
                let deleted_ok = include_deleted || !task.deleted;
                status_ok && archived_ok && deleted_ok
            });
            // 按创建时间（降序）和优先级（升序）排序
            tasks.sort_by(|a, b| {
                b.created_at_ms
                    .cmp(&a.created_at_ms)
                    .then_with(|| a.priority.cmp(&b.priority))
            });
            // 限制返回的任务数量
            if tasks.len() > limit {
                tasks.truncate(limit);
            }
            print_tasks_json(&tasks)
        }
    }
}

/// 解析并验证项目目录路径
///
/// 将用户提供的项目目录路径转换为规范的绝对路径，并进行以下验证：
/// 1. 路径不能为空（去除空白后）
/// 2. 路径必须存在且可访问
/// 3. 路径必须指向一个目录而非文件
///
/// # 参数
///
/// * `project_dir` - 用户提供的项目目录路径字符串
///
/// # 返回值
///
/// - `Ok(String)` - 规范化后的绝对路径字符串
/// - `Err(anyhow::Error)` - 路径为空、不存在或不是目录时的错误
///
/// # 错误
///
/// - 路径为空或仅包含空白字符
/// - 路径无法解析（权限不足或不存在）
/// - 路径指向的不是目录
///
/// # 示例
///
/// ```ignore
/// let path = resolve_project_dir("./my-project").unwrap();
/// // 返回类似 "/Users/username/projects/my-project" 的绝对路径
/// ```
fn resolve_project_dir(project_dir: &str) -> Result<String> {
    let trimmed = project_dir.trim();
    if trimmed.is_empty() {
        bail!("--project-dir cannot be empty");
    }
    let path = std::fs::canonicalize(trimmed)
        .with_context(|| format!("Failed to resolve project directory: {trimmed}"))?;
    if !path.is_dir() {
        bail!("project directory is not a folder: {}", path.display());
    }
    Ok(path.to_string_lossy().to_string())
}

/// 提取文本内容中的第一个非空行
///
/// 遍历文本的所有行，跳过空白行，返回第一个包含内容的行。
/// 如果所有行都为空，则返回空字符串。
///
/// # 参数
///
/// * `content` - 要处理的文本内容
///
/// # 返回值
///
/// 第一个非空行的字符串，如果不存在则返回空字符串
///
/// # 示例
///
/// ```ignore
/// let text = "\n  \n第一行内容\n第二行内容";
/// let first = first_non_empty_line(text); // 返回 "第一行内容"
/// ```
fn first_non_empty_line(content: &str) -> String {
    content.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default().to_string()
}

/// 以 JSON 格式打印单个任务信息
///
/// 将任务对象序列化为格式化的 JSON 字符串并输出到标准输出。
/// 使用 `serde_json::to_string_pretty` 进行美化格式化，便于人类阅读。
///
/// # 参数
///
/// * `task` - 要打印的任务对象引用
///
/// # 返回值
///
/// - `Ok(())` - 打印成功
/// - `Err(anyhow::Error)` - JSON 序列化失败
///
/// # 示例
///
/// ```ignore
/// let task = Task::new(1);
/// print_task_json(&task)?; // 输出格式化的 JSON
/// ```
fn print_task_json(task: &Task) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(task)?);
    Ok(())
}

/// 以 JSON 格式打印任务列表信息
///
/// 将任务对象数组序列化为格式化的 JSON 字符串并输出到标准输出。
/// 使用 `serde_json::to_string_pretty` 进行美化格式化，便于人类阅读。
///
/// # 参数
///
/// * `tasks` - 要打印的任务对象数组切片
///
/// # 返回值
///
/// - `Ok(())` - 打印成功
/// - `Err(anyhow::Error)` - JSON 序列化失败
///
/// # 示例
///
/// ```ignore
/// let tasks = vec![Task::new(1), Task::new(2)];
/// print_tasks_json(&tasks)?; // 输出格式化的 JSON 数组
/// ```
fn print_tasks_json(tasks: &[Task]) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(tasks)?);
    Ok(())
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod lib_tests;

#[cfg(test)]
mod native_task_contract_tests;
