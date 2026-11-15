//! Host composition for gateway prompt and hook inputs.
use super::state::AppState;
use crate::channels::ChannelRuntimeContext;
use std::sync::Arc;
pub(super) fn channel_runtime(state: &AppState) -> anyhow::Result<Arc<ChannelRuntimeContext>> {
    agent_gateway::channel_runtime::channel_runtime(
        state,
        crate::session::ui_store::current_tenant_scope(),
        |config, model, native_tools| {
            let timezone = crate::util::locale::parse_timezone(&config.app_ui.system_settings.timezone)?;
            let skills = crate::skills::load_skills_with_config(&config.workspace_dir, &config);
            let system_prompt = crate::channels::build_system_prompt_with_mode(
                &config.workspace_dir,
                model,
                &[],
                &skills,
                Some(&config.identity),
                None,
                native_tools,
                config.skills.prompt_injection_mode,
                timezone,
            );
            Ok(system_prompt)
        },
        || state.hook_runtime.as_ref().and_then(|runtime| runtime.runner_snapshot()),
    )
}
