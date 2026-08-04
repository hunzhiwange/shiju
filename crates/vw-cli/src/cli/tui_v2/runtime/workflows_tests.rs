use super::workflows::workflow_command_specs;
use super::super::commands::registry::{CommandAction, CommandRegistry, CommandSource};
use super::super::model::workflow_panel::WorkflowPanelItem;

#[test]
fn workflow_commands_register_stably_and_stream_events_into_tui_state() {
    let items = vec![
        WorkflowPanelItem {
            uuid: "wf-a".to_string(),
            name: "Code Review".to_string(),
            description: "review".to_string(),
        },
        WorkflowPanelItem {
            uuid: "wf-b".to_string(),
            name: "Code Review".to_string(),
            description: "duplicate".to_string(),
        },
    ];
    let specs = workflow_command_specs(&items);
    assert_eq!(specs.len(), 2);
    assert_ne!(specs[0].name, specs[1].name);
    assert!(specs
        .iter()
        .all(|spec| matches!(spec.action, CommandAction::Gateway(_))));
    let registry =
        CommandRegistry::builtin().replace_source(CommandSource::Workflow, Ok(specs));
    assert!(registry.resolve("help").is_some());
    assert_eq!(
        registry
            .commands()
            .iter()
            .filter(|spec| spec.source == CommandSource::Workflow)
            .count(),
        2
    );
}
