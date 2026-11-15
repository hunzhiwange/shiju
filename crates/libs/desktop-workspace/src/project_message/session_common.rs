/// parse_clamped_u32 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub fn parse_clamped_u32(raw: &str, fallback: u32, min: u32, max: u32) -> u32 {
    raw.trim().parse::<u32>().map(|v| v.clamp(min, max)).unwrap_or(fallback.clamp(min, max))
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod common_tests;

/// parse_clamped_u64 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub fn parse_clamped_u64(raw: &str, fallback: u64, min: u64, max: u64) -> u64 {
    raw.trim().parse::<u64>().map(|v| v.clamp(min, max)).unwrap_or(fallback.clamp(min, max))
}

/// trim_to_option 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub fn trim_to_option(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}


pub fn reset_picker_selection(project: &mut Option<String>, options: &mut Vec<(String, String)>, worktree_name: &mut String) {
    *project = None;
    options.clear();
    worktree_name.clear();
}
pub fn clear_picker_messages(confirm_delete: &mut Option<String>, force_delete: &mut Option<String>, delete_error: &mut Option<String>, confirm_reset: &mut Option<String>, reset_error: &mut Option<String>) {
    *confirm_delete = None;
    *force_delete = None;
    *delete_error = None;
    *confirm_reset = None;
    *reset_error = None;
}
