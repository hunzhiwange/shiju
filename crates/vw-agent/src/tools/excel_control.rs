//! Office tools use the shared capability library.
pub type ExcelControlTool = tool_office::ExcelControlTool<super::context::AgentContextHost>;
#[cfg(test)]
pub(crate) use tool_computer::office_owner;
