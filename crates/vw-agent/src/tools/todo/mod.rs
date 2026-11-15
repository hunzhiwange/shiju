//! Todo tools and state owned by tool-services.
pub use tool_services::todo::*;

pub fn read(ctx: &super::ToolRuntimeContext) -> Result<String, super::ToolCallError> {
    tool_services::todo::read_for_session(&ctx.session)
        .map_err(|error| super::ToolCallError::Failed(error.to_string()))
}

pub fn write(input: &str, ctx: &super::ToolRuntimeContext) -> Result<String, super::ToolCallError> {
    tool_services::todo::write_for_session(input, &ctx.session)
        .map_err(|error| super::ToolCallError::Failed(error.to_string()))
}
