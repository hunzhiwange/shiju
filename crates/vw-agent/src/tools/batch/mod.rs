//! Host specialization of the shared batch tool.
pub type BatchTool = tool_services::batch::BatchTool<super::context::AgentContextHost>;
#[cfg(test)]
use tool_services::batch::MAX_CALLS;
#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../libs/tool-services/src/batch/tests.rs"));
}
