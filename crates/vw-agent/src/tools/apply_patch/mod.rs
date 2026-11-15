//! Host specialization of the shared patch tool.
pub type ApplyPatchTool = tool_services::apply_patch::ApplyPatchTool<super::context::AgentContextHost>;
#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../libs/tool-services/src/apply_patch/tests.rs"));
}
