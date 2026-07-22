//! 提供任务看板消息处理过程中复用的局部辅助逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;
use crate::app::task::TaskExecutorBackend;

const DEFAULT_TASK_EXECUTOR_LABEL: &str = "未使用 ACP";

/// 执行 create_task_draft_with_preferences 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn create_task_draft_with_preferences(
    model: String,
    acp_agent: Option<String>,
) -> TaskDraft {
    let mut draft = TaskDraft::default();
    draft.model = normalize_task_model(&model);
    set_draft_executor_selection(&mut draft, acp_agent);
    draft
}

/// 执行 normalize_task_model 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn normalize_task_model(model: &str) -> String {
    normalize_task_model_input(model)
}

/// 执行 reset_create_draft 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn reset_create_draft(app: &mut crate::app::App) {
    app.task_board_draft = create_task_draft_with_preferences(
        app.task_board_last_model.clone(),
        app.task_board_last_acp_agent.clone(),
    );
    app.task_board_prompt_editor =
        iced::widget::text_editor::Content::with_text(&app.task_board_draft.prompt);
}

/// 执行 normalize_task_acp_agent 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn normalize_task_acp_agent(value: Option<&str>) -> Option<String> {
    value.and_then(normalize_task_acp_agent_input)
}

/// 执行 set_draft_executor_selection 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn set_draft_executor_selection(draft: &mut TaskDraft, value: Option<String>) {
    draft.acp_agent = normalize_task_acp_agent(value.as_deref());
}

/// 执行 set_task_executor_selection 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn set_task_executor_selection(task: &mut Task, value: Option<String>) {
    task.acp_agent = normalize_task_acp_agent(value.as_deref());
}

/// 执行 initial_task_status_for_workflow_id 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn initial_task_status_for_workflow_id(workflow_id: &str) -> TaskStatus {
    TaskStatus::all_for_workflow_id(workflow_id).into_iter().next().unwrap_or(TaskStatus::Pool)
}

/// 执行 task_acp_agent_label 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn task_acp_agent_label(value: Option<&str>) -> String {
    match normalize_task_acp_agent(value) {
        Some(agent) => TaskExecutorBackend::from_id(&agent)
            .map(|backend| backend.label().to_string())
            .unwrap_or(agent),
        None => DEFAULT_TASK_EXECUTOR_LABEL.to_string(),
    }
}

/// 执行 parse_priority_or_default 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn parse_priority_or_default(priority: &str, default_priority: u32) -> u32 {
    priority.trim().parse::<u32>().ok().filter(|value| *value > 0).unwrap_or(default_priority)
}

/// 执行 import_demo_content 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn import_demo_content(template: &str) -> Option<&'static str> {
    match template {
        "JSON 示例" => Some(
            r#"[
  {
    "priority": 900,
    "prompt": "定位登录接口 500 错误的根因，并给出修复方案",
	    "model": "auto",
	        "acp_agent": "opencode",
	        "reasoning_effort": "high",
	        "task_split_mode": "follow_system"
  },
  {
    "priority": 700,
    "prompt": "为登录接口补充单元测试和错误码断言",
	    "model": "auto",
	        "acp_agent": "claude",
	        "reasoning_effort": "max",
	        "task_split_mode": "follow_system"
  }
]"#,
        ),
        "CSV 示例" => Some(
            "priority,prompt,model,acp_agent,reasoning_effort,task_split_mode\n900,定位登录接口 500 错误的根因，并给出修复方案,auto,opencode,high,follow_system\n700,为登录接口补充单元测试和错误码断言,auto,claude,max,follow_system",
        ),
        "TSV 示例" => Some(
            "priority\tprompt\tmodel\tacp_agent\treasoning_effort\ttask_split_mode\n900\t定位登录接口 500 错误的根因，并给出修复方案\tauto\topencode\thigh\tfollow_system\n700\t为登录接口补充单元测试和错误码断言\tauto\tclaude\tmax\tfollow_system",
        ),
        _ => None,
    }
}

/// 执行 import_prompt_template 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn import_prompt_template(
    format: TaskImportPromptFormat,
    selected_priority: u32,
    selected_model: &str,
    selected_acp_agent: Option<&str>,
) -> String {
    let model = if selected_model.trim().is_empty() { "auto" } else { selected_model.trim() };
    let acp_agent = selected_acp_agent.unwrap_or("default");
    let template = match format {
        TaskImportPromptFormat::Json => format!(
            "你是任务拆解助手。请把我的需求拆成任务列表，并严格只输出 JSON 数组。\n\n# 工作流程（必须执行）\n1. 先搜索代码库，找到所有相关的文件位置和关键代码行号\n2. 理解每个需求的现状和实现上下文\n3. 将完整的上下文信息写入每个任务的 prompt 字段\n\n# 任务拆分规则\n1. 按功能点拆分，每个任务独立可执行\n2. 如果多个需求有依赖关系，按依赖顺序排序\n\n# 任务内容要求（每个任务的 prompt 必须包含）\n1. 【任务标题】简短描述\n2. 相关文件路径和关键代码行号\n3. 当前实现现状（现在是怎么做的）\n4. 具体实现要求（需要改成什么样）\n5. 参考组件/函数（如果有现成的可复用）\n6. 技术栈说明（如适用）\n\n# 输出格式\n1. 第一行只能是 JSON 数组起始符号 [\n2. 数组中每个对象必须包含 priority,prompt,model,acp_agent,task_split_mode\n3. priority 使用数字，建议优先使用 {selected_priority}\n4. model 默认使用 {model}\n5. acp_agent 默认使用 {acp_agent}\n6. task_split_mode 默认使用 follow_system\n7. 不要输出 Markdown 代码块，不要输出解释\n\n输出示例：\n[\n  {{\n    \"priority\": {selected_priority},\n    \"prompt\": \"【登录页面优化】\\n文件：src/views/login.rs，第45-80行 login_form 函数\\n现状：使用普通 input 组件，无密码显示切换\\n需求：增加密码明文/密文切换按钮\\n参考：src/components/password_input.rs 已有实现\\n技术栈：Rust + Leptos\",\n    \"model\": \"{model}\",\n    \"acp_agent\": \"{acp_agent}\",\n    \"task_split_mode\": \"follow_system\"\n  }}\n]"
        ),
        TaskImportPromptFormat::Csv => format!(
            "你是任务拆解助手。请把我的需求拆成任务列表，并严格只输出 CSV。\n\n# 工作流程（必须执行）\n1. 先搜索代码库，找到所有相关的文件位置和关键代码行号\n2. 理解每个需求的现状和实现上下文\n3. 将完整的上下文信息写入每个任务的 prompt 字段\n\n# 任务拆分规则\n1. 按功能点拆分，每个任务独立可执行\n2. 如果多个需求有依赖关系，按依赖顺序排序\n\n# 任务内容要求（每个任务的 prompt 必须包含）\n1. 【任务标题】简短描述\n2. 相关文件路径和关键代码行号\n3. 当前实现现状（现在是怎么做的）\n4. 具体实现要求（需要改成什么样）\n5. 参考组件/函数（如果有现成的可复用）\n6. 技术栈说明（如适用）\n\n# 输出格式\n1. 第一行固定表头：priority,prompt,model,acp_agent,task_split_mode\n2. priority 使用数字，建议优先使用 {selected_priority}\n3. model 默认使用 {model}\n4. acp_agent 默认使用 {acp_agent}\n5. task_split_mode 默认使用 follow_system\n6. 不要输出 Markdown 代码块，不要输出解释\n7. prompt 字段使用双引号包裹，内部换行使用实际换行\n\n输出示例：\npriority,prompt,model,acp_agent,task_split_mode\n{selected_priority},\"【登录页面优化】\n文件：src/views/login.rs，第45-80行 login_form 函数\n现状：使用普通 input 组件，无密码显示切换\n需求：增加密码明文/密文切换按钮\n参考：src/components/password_input.rs 已有实现\n技术栈：Rust + Leptos\",{model},{acp_agent},follow_system"
        ),
        TaskImportPromptFormat::Tsv => format!(
            "你是任务拆解助手。请把我的需求拆成任务列表，并严格只输出 TSV。\n\n# 工作流程（必须执行）\n1. 先搜索代码库，找到所有相关的文件位置和关键代码行号\n2. 理解每个需求的现状和实现上下文\n3. 将完整的上下文信息写入每个任务的 prompt 字段\n\n# 任务拆分规则\n1. 按功能点拆分，每个任务独立可执行\n2. 如果多个需求有依赖关系，按依赖顺序排序\n\n# 任务内容要求（每个任务的 prompt 必须包含）\n1. 【任务标题】简短描述\n2. 相关文件路径和关键代码行号\n3. 当前实现现状（现在是怎么做的）\n4. 具体实现要求（需要改成什么样）\n5. 参考组件/函数（如果有现成的可复用）\n6. 技术栈说明（如适用）\n\n# 输出格式\n1. 第一行固定表头：priority<TAB>prompt<TAB>model<TAB>acp_agent<TAB>task_split_mode\n2. priority 使用数字，建议优先使用 {selected_priority}\n3. model 默认使用 {model}\n4. acp_agent 默认使用 {acp_agent}\n5. task_split_mode 默认使用 follow_system\n6. 不要输出 Markdown 代码块，不要输出解释\n7. prompt 字段使用双引号包裹，内部换行使用实际换行\n\n输出示例：\npriority\tprompt\tmodel\tacp_agent\ttask_split_mode\n{selected_priority}\t\"【登录页面优化】\n文件：src/views/login.rs，第45-80行 login_form 函数\n现状：使用普通 input 组件，无密码显示切换\n需求：增加密码明文/密文切换按钮\n参考：src/components/password_input.rs 已有实现\n技术栈：Rust + Leptos\"\t{model}\t{acp_agent}\tfollow_system"
        ),
    };
    template
        .replace(
            "priority,prompt,model,acp_agent,task_split_mode",
            "priority,prompt,model,acp_agent,reasoning_effort,task_split_mode",
        )
        .replace(
            "priority<TAB>prompt<TAB>model<TAB>acp_agent<TAB>task_split_mode",
            "priority<TAB>prompt<TAB>model<TAB>acp_agent<TAB>reasoning_effort<TAB>task_split_mode",
        )
        .replace(
            "priority\tprompt\tmodel\tacp_agent\ttask_split_mode",
            "priority\tprompt\tmodel\tacp_agent\treasoning_effort\ttask_split_mode",
        )
        .replace(
            &format!("\"acp_agent\": \"{acp_agent}\",\n    \"task_split_mode\""),
            &format!(
                "\"acp_agent\": \"{acp_agent}\",\n    \"reasoning_effort\": null,\n    \"task_split_mode\""
            ),
        )
        .replace(
            &format!(",{acp_agent},follow_system"),
            &format!(",{acp_agent},,follow_system"),
        )
        .replace(
            &format!("\t{acp_agent}\tfollow_system"),
            &format!("\t{acp_agent}\t\tfollow_system"),
        )
        + "\n\nreasoning_effort 可留空表示默认，或使用 none、low、medium、high、xhigh、max；非法值必须拒绝。"
}
#[cfg(test)]
#[path = "draft_tests.rs"]
mod draft_tests;
