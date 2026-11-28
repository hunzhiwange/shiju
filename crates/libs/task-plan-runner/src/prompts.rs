//! 规划 / 执行提示词与计划目录工具。

use crate::host::recursive_orchestration_policy;
use std::path::{Path, PathBuf};

const TASK_AUTOMATION_TOOL_GUARDRAIL: &str = "任务执行约束：不要调用名为 `node`、`node_repl` 或 `nodeRepl` 的 MCP/REPL 工具；解析 JSON、coverage 或报告数据时，优先使用 shell 命令、仓库已有工具或目标语言原生命令。";

pub(crate) const EXECUTION_SKILL_HINT: &str = "通过当前会话的技能入口调用内置技能 `do-task-plans`（`$do-task-plans`）。以下工作流要求优先于技能中的通用约定。技能不可用时，按本提示继续。";

/// 将任务 ID 规范化为可做目录名的片段。
pub fn sanitize_task_plan_id_part(value: &str) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            output.push(ch);
        } else if ch == '.' {
            output.push('-');
        }
    }
    if output.is_empty() { "task".to_string() } else { output }
}

/// 计划目录名：`task-<sanitized-id>`。
pub fn plan_dir_name(task_id: &str) -> String {
    format!("task-{}", sanitize_task_plan_id_part(task_id))
}

/// 用户级计划目录：`<home>/<HOME_CONFIG_DIR_NAME>/storage/plan/task-<sanitized-id>`。
pub fn plan_dir_abs(home: impl AsRef<Path>, task_id: &str) -> PathBuf {
    config_runtime_paths::task_plan_storage_dir(home).join(plan_dir_name(task_id))
}

/// 构建规划阶段提示词。
pub fn build_planning_prompt(task_id: &str, query: &str) -> String {
    let plan_dir = plan_dir_abs(&agent_paths::paths().home, task_id);
    build_planning_prompt_for_dir(task_id, query, &plan_dir)
}

pub fn build_planning_prompt_for_dir(task_id: &str, query: &str, plan_dir: &Path) -> String {
    let tasks_json_path = plan_dir.join("tasks.json");
    let plan_dir = plan_dir.display();
    let title = planning_title(query);
    let json_example = serde_json::json!({
        "plan_dir": plan_dir.to_string(),
        "tasks": [{
            "id": "1",
            "title": "实现变更",
            "file": "1-实现变更.md",
            "batch": 1,
            "depends_on_batches": [],
            "status": "未开始"
        }]
    });
    format!(
        "把这个开发需求拆分成可独立执行并跟踪的实施计划，只生成计划目录与文件。通过当前会话的技能入口调用内置技能 `create-task-plans`（`$create-task-plans`）。以下工作流要求优先于技能中的通用约定。技能不可用时，按本提示继续。

{recursive_policy}

## 计划文件

- 必须通过写文件工具实际创建文件。
- 唯一计划目录为已预创建的绝对路径 `{plan_dir}`。所有计划文件都写入该目录。
- 使用中文编写 `plan.md`、各编号任务文件与 `prompt.md`。`plan.md` 记录整体架构和公共规则。编号任务文件写实现范围、验收条件、批次、依赖批次及初始状态 `未开始`。
- `prompt.md` 要覆盖每个编号任务，并引用对应任务文件、当前批次和依赖批次。编号任务文件必须保留给工作流收集验收证据。

## 工作流附加要求：调度清单

- 编号任务文件确定后，将调度清单写入 `{tasks_json_path}`。`tasks.json` 顶层必须包含 `plan_dir` 和 `tasks`。
- `plan_dir` 填写指定的计划绝对路径。`tasks` 必须是非空数组，包含每个编号任务文件且每个文件仅出现一次。
- 每个条目的 `id` 使用唯一的字符串，`title` 使用任务标题，`file` 填写相对于计划目录的文件名，`batch` 使用整数，`depends_on_batches` 使用整数数组，`status` 填写 `未开始`。没有依赖时填 `[]`，有依赖时只引用清单中已存在且小于当前批次的批次。
- 以下示例只说明结构。请用实际任务替换示例条目，保存为可直接解析的 JSON。

```json
{json_example:#}
```

- 返回前重新读取清单，确认每条任务的文件真实存在，标题、批次和依赖与 Markdown 一致。

## 批次与依赖批次

- 在任务文件和 `prompt.md` 中写明当前批次和依赖批次，没有依赖时写“无”。
- 同一批次只安排能独立实现且无写入与验证资源冲突的任务。存在实际依赖时，安排到更晚批次并声明依赖。

## 自动化排障约束

- 用户无法实时干预。遇到问题时，先依靠仓库上下文、现有实现与相邻测试查证。
- 只有遇到外部依赖缺失、权限不足或安全风险等无法自行解决的情况，才在对应任务文件中记录阻塞原因。
- {TASK_AUTOMATION_TOOL_GUARDRAIL}

## 本次需求

任务 ID 为 {task_id}
标题为 {title}

{}",
        query.trim(),
        tasks_json_path = tasks_json_path.display(),
        recursive_policy = recursive_orchestration_policy(),
    )
}

/// 构建目标模式规划提示词，要求计划最后包含独立验收关卡。
pub fn build_goal_planning_prompt(task_id: &str, query: &str) -> String {
    let plan_dir = plan_dir_abs(&agent_paths::paths().home, task_id);
    build_goal_planning_prompt_for_dir(task_id, query, &plan_dir)
}

pub fn build_goal_planning_prompt_for_dir(
    task_id: &str,
    query: &str,
    plan_dir: &Path,
) -> String {
    format!(
        "{}\
\n\n## 目标模式附加要求\
\n\n- 这是持续目标模式：未达到目标或验证未通过时，运行时会自动续轮。\
\n- 计划最后必须包含一个独立的目标验收任务，依赖全部实现批次。\
\n- 验收任务必须运行仓库要求和本次变更相关的确切检查，并在输出中展示命令、退出码和结果。\
\n- 编号任务文件是目标验收清单，不得删除。完成时必须保留文件，并填写 `状态：已完成`、非空 `完成条件：`、`验证命令：` 和包含 `退出码：0` 的 `验证结果：`；只有外部凭据、权限或安全风险无法自行解决时才能标记 `状态：已阻塞`。",
        build_planning_prompt_for_dir(task_id, query, plan_dir)
    )
}

/// 构建子任务执行提示词。
pub fn build_execution_prompt(
    query: &str,
    batch: u32,
    depends_on_batches: &[u32],
    plan_abs: &Path,
    task_file_abs: &Path,
    plan_markdown: &str,
    subtask_markdown: &str,
) -> String {
    let depends_on = format_depends_on_batches(depends_on_batches);
    format!(
        "单独执行当前子任务.md 中定义的子任务。{EXECUTION_SKILL_HINT}不要开始后续外层子任务。不得改写外层编号任务、批次依赖或任务文件所有权；可在当前任务边界内自主拆分内部工作流。\
\n\n{recursive_policy}\
\n\n## 任务池自动化执行规则\
\n\n- 这是自动化执行，用户无法实时干预；不要询问用户、不要等待澄清。先用仓库上下文、现有模块、相邻测试和搜索自行查证、自行排障并继续。\
\n- 能自行解决的小问题就直接解决，不要因为可以自行排障的小障碍立即停下来询问用户。\
\n- 只有遇到外部凭据缺失、权限不足、安全风险、破坏性操作或连续验证始终无法推进时，才在对应编号任务文件中标记 `已阻塞` 并记录具体原因和已运行的检查；不要把阻塞原因写进 plan.md。\
\n- `plan.md` 为只读整体架构指南，任何情况下都不要修改、清理或往其中记录进度。\
\n- 子任务获得所需验证证据后，务必保留对应编号任务文件，并填写 `状态：已完成`、非空 `完成条件：`、`验证命令：` 与包含成功退出码的 `验证结果：`；被阻塞时只在对应编号任务文件中更新为 `已阻塞`。\
\n- 预期失败命令必须由退出 0 的外层检查证明；顶层 `验证命令：` 只能记录完成后实际运行且退出 0 的检查，内层预期 `EXIT=1` 不得写成完成证据。\
\n- {TASK_AUTOMATION_TOOL_GUARDRAIL}\
\n\n## 规范任务文件路径\
\n\n- 规范计划目录: `{plan_abs}`\
\n- 当前任务文件（唯一可写进度源）: `{task_file_abs}`\
\n- 只允许在记录进度、完成或阻塞时更新上述当前任务文件；不搜索、不新建、不更新仓库内同名 `.plans` 文件。\
\n\n## 当前批次上下文\
\n\n- 当前批次: {batch}\
\n- 依赖批次: {depends_on}\
\n- 同批次内的子任务可并发执行；依赖批次必须全部完成后，当前批次才会开始。\
\n\n# plan.md\n{}\
\n\n# 子任务.md\n{}\
\n\n# 原始需求\n{}",
        plan_markdown.trim(),
        subtask_markdown.trim(),
        query.trim(),
        plan_abs = plan_abs.display(),
        task_file_abs = task_file_abs.display(),
        recursive_policy = recursive_orchestration_policy()
    )
}

fn format_depends_on_batches(depends_on_batches: &[u32]) -> String {
    if depends_on_batches.is_empty() {
        "无".to_string()
    } else {
        depends_on_batches.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")
    }
}

fn planning_title(query: &str) -> String {
    let first_line = query.lines().next().unwrap_or(query).trim();
    if first_line.is_empty() {
        return "工作流执行".to_string();
    }
    let truncated: String = first_line.chars().take(40).collect();
    if first_line.chars().count() > 40 { format!("{truncated}…") } else { truncated }
}
