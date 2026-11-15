//! Host specialization retaining trusted tenant context.
pub type QuestionTool = tool_services::question::QuestionTool<super::context::AgentContextHost>;
pub use tool_services::question::header_from_question;
