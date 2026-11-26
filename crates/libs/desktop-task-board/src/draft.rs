//! 提供任务看板消息处理过程中复用的局部辅助逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use desktop_task_model::{Task, TaskDraft, TaskStatus, normalize_task_model_input, normalize_task_acp_agent_input};
use desktop_task_model::TaskExecutorBackend;

const DEFAULT_TASK_EXECUTOR_LABEL: &str = "未使用 ACP";

/// 执行 create_task_draft_with_preferences 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn create_task_draft_with_preferences(
    model: String,
    acp_agent: Option<String>,
) -> TaskDraft {
    let mut draft = TaskDraft {
        model: normalize_task_model(&model),
        ..Default::default()
    };
    set_draft_executor_selection(&mut draft, acp_agent);
    draft
}

/// 执行 normalize_task_model 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn normalize_task_model(model: &str) -> String {
    normalize_task_model_input(model)
}

/// 执行 normalize_task_acp_agent 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn normalize_task_acp_agent(value: Option<&str>) -> Option<String> {
    value.and_then(normalize_task_acp_agent_input)
}

/// 执行 set_draft_executor_selection 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn set_draft_executor_selection(draft: &mut TaskDraft, value: Option<String>) {
    draft.acp_agent = normalize_task_acp_agent(value.as_deref());
    if draft.acp_agent.is_none() && draft.model == desktop_task_model::TASK_MODEL_ACP_NATIVE {
        draft.model = desktop_task_model::TASK_MODEL_AUTO.to_owned();
    }
}

/// 执行 set_task_executor_selection 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn set_task_executor_selection(task: &mut Task, value: Option<String>) {
    task.acp_agent = normalize_task_acp_agent(value.as_deref());
    if task.acp_agent.is_none() && task.model == desktop_task_model::TASK_MODEL_ACP_NATIVE {
        task.model = desktop_task_model::TASK_MODEL_AUTO.to_owned();
    }
}

/// 执行 initial_task_status_for_workflow_id 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn initial_task_status_for_workflow_id(workflow_id: &str) -> TaskStatus {
    TaskStatus::all_for_workflow_id(workflow_id).into_iter().next().unwrap_or(TaskStatus::Pool)
}

/// 执行 task_acp_agent_label 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn task_acp_agent_label(value: Option<&str>) -> String {
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
pub fn parse_priority_or_default(priority: &str, default_priority: u32) -> u32 {
    priority.trim().parse::<u32>().ok().filter(|value| *value > 0).unwrap_or(default_priority)
}

/// 执行 import_demo_content 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn import_demo_content(template: &str) -> Option<&'static str> {
    match template {
        "JSON 示例" => Some(
            r#"[
  {
    "priority": 900,
    "prompt": "定位登录接口 500 错误的根因，并给出修复方案",
	    "model": "auto",
	        "acp_agent": "opencode",
	        "reasoning_effort": "high"
  },
  {
    "priority": 700,
    "prompt": "为登录接口补充单元测试和错误码断言",
	    "model": "auto",
	        "acp_agent": "claude",
	        "reasoning_effort": "max"
  }
]"#,
        ),
        "CSV 示例" => Some(
            "priority,prompt,model,acp_agent,reasoning_effort\n900,定位登录接口 500 错误的根因，并给出修复方案,auto,opencode,high\n700,为登录接口补充单元测试和错误码断言,auto,claude,max",
        ),
        "TSV 示例" => Some(
            "priority\tprompt\tmodel\tacp_agent\treasoning_effort\n900\t定位登录接口 500 错误的根因，并给出修复方案\tauto\topencode\thigh\n700\t为登录接口补充单元测试和错误码断言\tauto\tclaude\tmax",
        ),
        _ => None,
    }
}

#[cfg(test)]
#[path = "draft_tests.rs"]
mod draft_tests;

/// 重建创建草稿并同步提示词编辑器。
pub fn reset_create_draft(
    draft: &mut TaskDraft,
    prompt_editor: &mut iced::widget::text_editor::Content,
    model: String,
    acp_agent: Option<String>,
) {
    *draft = create_task_draft_with_preferences(model, acp_agent);
    *prompt_editor = iced::widget::text_editor::Content::with_text(&draft.prompt);
}
