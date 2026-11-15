//! Supplies host factory output and process flags to the registry policy.
use crate::flag;
#[cfg(test)]
use tool_assembly::registry::{should_enable_question_tool, should_use_apply_patch};
#[cfg(test)]
fn should_enable_search(provider: &str) -> bool { tool_assembly::registry::should_enable_search(provider, *flag::VIBEWINDOW_ENABLE_EXA) }
pub fn specs(model: Option<&str>) -> Vec<super::ToolSpec> {
    tool_assembly::registry::specs(super::tool_specs_for_context(&super::ToolRuntimeContext::for_specs()), model, &flag::vibewindow_client(), *flag::VIBEWINDOW_ENABLE_EXA)
}
pub fn spec_dtos(model: Option<&str>) -> Vec<vw_api_tools::ToolSpecDto> {
    tool_assembly::registry::spec_dtos(super::tool_specs_for_context(&super::ToolRuntimeContext::for_specs()), model, &flag::vibewindow_client(), *flag::VIBEWINDOW_ENABLE_EXA)
}
pub fn list_tool_specs_response(model: Option<&str>) -> vw_api_tools::ListToolSpecsResponse {
    tool_assembly::registry::list_tool_specs_response(super::tool_specs_for_context(&super::ToolRuntimeContext::for_specs()), model, &flag::vibewindow_client(), *flag::VIBEWINDOW_ENABLE_EXA)
}
pub fn specs_json(model: Option<&str>) -> serde_json::Value {
    tool_assembly::registry::specs_json(super::tool_specs_for_context(&super::ToolRuntimeContext::for_specs()), model, &flag::vibewindow_client(), *flag::VIBEWINDOW_ENABLE_EXA)
}
pub fn openai_tools_json(model: Option<&str>) -> serde_json::Value {
    tool_assembly::registry::openai_tools_json(super::tool_specs_for_context(&super::ToolRuntimeContext::for_specs()), model, &flag::vibewindow_client(), *flag::VIBEWINDOW_ENABLE_EXA)
}
#[cfg(test)]
mod tests;
