//! Construct native turn instances and their guards; the runner owns turn policy.
use super::*;

pub(super) fn admit_context(
    req: &Request,
    config: &Config,
    native_defaults: Option<session_engine::capability_policy::NativeAgentDefaults>,
    selected_model: &ResolvedRequestModel,
    model_selected_by_auto: bool,
    skill_workspace: &Path,
    sink: &mut impl FnMut(StreamEvent) -> bool,
) -> Option<(
    ToolRuntimeContext,
    <AgentRunnerHost as session_engine::runner_host::RunnerHost>::Admission,
)> {
    let mut plugin_config = config.clone();
    plugin_config.workspace_dir = skill_workspace.to_path_buf();
    let mut plugin_turn =
        match crate::plugins::session_snapshot::PluginTurnSnapshot::capture(&plugin_config) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                sink(StreamEvent::Error(format!("Plugin turn admission failed: {error:#}")));
                return None;
            }
        };
    let tool_use_context = session_engine::runner_admission::request_context::<AgentRunnerHost>(
        req, native_defaults, selected_model, model_selected_by_auto,
        plugin_turn.data.clone(), plugin_turn.data.hooks.clone(), sink,
    )?;
    plugin_turn.own_async_hooks(tool_use_context.async_hooks.clone());
    if let Some(root) = req.root.as_deref() {
        crate::plugins::mcp_app_execution::revoke(&req.session, std::path::Path::new(root));
    }
    if let Err(error) = prompt::block_on(crate::plugins::hook_session::start(
        std::sync::Arc::new(tool_use_context.clone()),
        !req.history.is_empty(),
    )) {
        sink(StreamEvent::Error(error.to_string()));
        return None;
    }
    let _async_hook_turn = tool_use_context.async_hooks.turn_guard();
    if let Err(error) = prompt::block_on(crate::plugins::hook_prompt::submit(
        std::sync::Arc::new(tool_use_context.clone()),
        &req.query,
    )) {
        sink(StreamEvent::Error(error.to_string()));
        return None;
    }
    let ctx = ToolRuntimeContext::new(req.session.clone(), req.root.clone())
        .with_tool_use_context(tool_use_context)
        .with_session_tools(req.session_tools.clone());
    Some((ctx, (_async_hook_turn, plugin_turn)))
}
