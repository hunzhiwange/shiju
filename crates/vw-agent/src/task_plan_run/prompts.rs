//! 规划 / 执行提示词与计划目录工具。

use crate::agent::prompt::multi_agent::recursive_orchestration_policy;
use std::path::{Path, PathBuf};

/// 框架运行时 create-task-plans 规划规范的来源路径（相对仓库）。
pub const CREATE_TASK_PLANS_SKILL_SOURCE: &str =
    "crates/vw-agent/assets/runtime-skills/create-task-plans-runtime/SKILL.md";

/// 框架运行时 do-task-plans 执行规范的来源路径（相对仓库）。
pub const DO_TASK_PLANS_SKILL_SOURCE: &str =
    "crates/vw-agent/assets/runtime-skills/do-task-plans-runtime/SKILL.md";

const CREATE_TASK_PLANS_SKILL: &str =
    include_str!("../../assets/runtime-skills/create-task-plans-runtime/SKILL.md");

const DO_TASK_PLANS_SKILL: &str =
    include_str!("../../assets/runtime-skills/do-task-plans-runtime/SKILL.md");

const TASK_AUTOMATION_TOOL_GUARDRAIL: &str = "任务执行约束：不要调用名为 `node`、`node_repl` 或 `nodeRepl` 的 MCP/REPL 工具；解析 JSON、coverage 或报告数据时，优先使用 shell 命令、仓库已有工具或目标语言原生命令。";

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
    vw_config_types::paths::task_plan_storage_dir(home).join(plan_dir_name(task_id))
}

/// 构建规划阶段提示词（对齐桌面 `build_task_split_prompt`）。
pub fn build_planning_prompt(task_id: &str, query: &str) -> String {
    let plan_dir = plan_dir_abs(&crate::global::paths().home, task_id);
    build_planning_prompt_for_dir(task_id, query, &plan_dir)
}

pub(super) fn build_planning_prompt_for_dir(task_id: &str, query: &str, plan_dir: &Path) -> String {
    let plan_dir = format!("{}/", plan_dir.display());
    let title = planning_title(query);
    format!(
        "请使用下面由框架运行时注入的 create-task-plans 规划规范，把这个开发需求拆分成可独立执行并跟踪的实施计划，不要直接写代码，只生成计划目录与文件。\n\n{recursive_policy}\n\n## 硬性落盘要求（必须先满足）\n\n- 必须通过写文件工具实际创建文件；禁止只在聊天回复中粘贴计划内容而不落盘。\n- 计划目录固定且唯一：绝对路径 `{plan_dir}`。忽略技能模板里的 `YYYY-MM-DD-<功能名称>` 路径，不要另起日期目录。\n- 目录已预创建；你必须在该目录下写出全部文件后才算完成：`plan.md`、`prompt.md`、`tasks.json`、以及 `1-<任务名>.md`、`2-<任务名>.md` 等编号任务文件。\n- 调度器只会读取 `{plan_dir}tasks.json`；写到其他路径会导致执行失败。\n\n## 框架运行时规范来源\n\n- create-task-plans-runtime: {CREATE_TASK_PLANS_SKILL_SOURCE}\n- do-task-plans-runtime: {DO_TASK_PLANS_SKILL_SOURCE}\n\n## 输出要求\n\n- `tasks.json` 必须与 Markdown 文件一一对应；每个任务条目必须包含 `id`、`title`、`file`、`batch`、`depends_on_batches`、`status`。\n- 所有计划内容使用中文，并严格遵循下方技能模板的 plan.md / prompt.md / tasks.json / 任务文件结构。\n- prompt.md 必须覆盖每个编号任务，并引用对应任务文件、当前批次和依赖批次。\n\n## 批次与依赖批次\n\n- 为每个编号任务在任务文件、prompt.md 和 tasks.json 中写明`当前批次`和可为多个的`依赖批次`（无依赖时写“无”或空数组）。\n- 同一批次只允许放入彼此没有任务依赖、不会争用同一文件、且验证资源不冲突的任务；不能证明可并发时必须升高批次数，不要把不能并行的任务塞进同一批次。\n- 用批次表达线性顺序与可并行集合：后批次依赖前批次完成，同批次内的任务可并发执行。\n\n## 自动化排障约束\n\n- 这是自动化执行，用户无法实时干预；不要询问用户、不要等待澄清。先用仓库上下文、现有模块、相邻测试和搜索自行收敛假设。\n- 只有遇到外部依赖缺失、权限不足或安全风险等无法自行解决的情况，才在对应任务文件中记录阻塞原因；其余情况自行查证并继续。\n- {TASK_AUTOMATION_TOOL_GUARDRAIL}\n\n## 框架运行时 create-task-plans 规划规范\n\n{CREATE_TASK_PLANS_SKILL}\n\n## 本次需求\n\n任务ID: {task_id}\n标题: {title}\n需求:\n{}",
        query.trim(),
        recursive_policy = recursive_orchestration_policy()
    )
}

/// 构建目标模式规划提示词，要求计划最后包含独立验收关卡。
pub fn build_goal_planning_prompt(task_id: &str, query: &str) -> String {
    let plan_dir = plan_dir_abs(&crate::global::paths().home, task_id);
    build_goal_planning_prompt_for_dir(task_id, query, &plan_dir)
}

pub(super) fn build_goal_planning_prompt_for_dir(
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

/// 构建子任务执行提示词（对齐桌面 `build_subtask_execution_prompt`）。
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
        "请按下面由框架运行时注入的 do-task-plans 执行流程，单独执行当前子任务.md 中定义的子任务。不要开始后续外层子任务。不得改写外层编号任务、批次依赖或任务文件所有权；可在当前任务边界内自主拆分内部工作流。\
\n\n{recursive_policy}\
\n\n## 任务池自动化执行规则\
\n\n- 这是自动化执行，用户无法实时干预；不要询问用户、不要等待澄清。先用仓库上下文、现有模块、相邻测试和搜索自行查证、自行排障并继续。\
\n- 能自行解决的小问题就直接解决，不要因为可以自行排障的小障碍立即停下来询问用户。\
\n- 只有遇到外部凭据缺失、权限不足、安全风险、破坏性操作或连续验证始终无法推进时，才在对应编号任务文件中标记 `已阻塞` 并记录具体原因和已运行的检查；不要把阻塞原因写进 plan.md。\
\n- `plan.md` 为只读整体架构指南，任何情况下都不要修改、清理或往其中记录进度。\
\n- 子任务获得所需验证证据后，保留对应编号任务文件，并填写 `状态：已完成`、非空 `完成条件：`、`验证命令：` 与包含成功退出码的 `验证结果：`；被阻塞时只在对应编号任务文件中更新为 `已阻塞`。\
\n- 本流程由框架运行时直接注入，无需安装 do-task-plans 技能（不依赖用户项目安装该技能）即可执行。\
\n- {TASK_AUTOMATION_TOOL_GUARDRAIL}\
\n\n## 规范任务文件路径\
\n\n- 规范计划目录: `{plan_abs}`\
\n- 当前任务文件（唯一可写进度源）: `{task_file_abs}`\
\n- 只允许在记录进度、完成或阻塞时更新上述当前任务文件；不搜索、不新建、不更新仓库内同名 `.plans` 文件。\
\n\n## 当前批次上下文\
\n\n- 当前批次: {batch}\
\n- 依赖批次: {depends_on}\
\n- 同批次内的子任务可并发执行；依赖批次必须全部完成后，当前批次才会开始。\
\n\n## 框架运行时 do-task-plans 执行规范\
\n\n来源: {DO_TASK_PLANS_SKILL_SOURCE}\
\n\n{}\
\n\n# plan.md\n{}\
\n\n# 子任务.md\n{}\
\n\n# 原始需求\n{}",
        DO_TASK_PLANS_SKILL.trim(),
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
