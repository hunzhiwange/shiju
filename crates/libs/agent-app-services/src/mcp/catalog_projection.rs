//! Only exact, complete static presets may leave private storage as importable JSON.
use vw_agent_apps::mcp::presets;
use vw_api_mcp_management::*;

fn public_preset(connection: &McpConnectionDefinition) -> Option<McpConnectionDefinition> {
    presets::list_presets().iter().find_map(|preset| {
        let draft = presets::create_preset_draft(preset.id, &[]).ok()??;
        (draft.connection == *connection).then_some(draft.connection)
    })
}

pub(super) fn project_summary(record: &mut McpCatalogRecord, connection: &McpConnectionDefinition) {
    if record.public_summary.is_none()
        && let Some(McpConnectionDefinition::Stdio { command, args, .. }) =
            public_preset(connection)
    {
        record.public_summary =
            Some(std::iter::once(command).chain(args).collect::<Vec<_>>().join(" "));
    }
}
