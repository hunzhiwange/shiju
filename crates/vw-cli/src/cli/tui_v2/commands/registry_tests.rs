use super::registry::{
    CommandAction, CommandAvailability, CommandExecutionPolicy, CommandRegistry, CommandSource,
    CommandSpec,
};

fn command(
    name: &str,
    aliases: &[&str],
    availability: CommandAvailability,
    action: CommandAction,
) -> CommandSpec {
    CommandSpec {
        name: name.to_string(),
        aliases: aliases.iter().map(|alias| (*alias).to_string()).collect(),
        summary: format!("{name} command"),
        takes_argument: true,
        argument_hint: Some("<target>".to_string()),
        source: CommandSource::Mcp,
        availability,
        execution_policy: CommandExecutionPolicy::Queued,
        action,
    }
}

#[test]
fn tui_command_registry_merges_dynamic_sources_and_dispatches_typed_actions() {
    let registry = CommandRegistry::merge(vec![
        (
            CommandSource::Mcp,
            Ok(vec![command(
                "deploy",
                &["ship"],
                CommandAvailability::Available,
                CommandAction::Gateway("mcp.deploy".to_string()),
            )]),
        ),
        (
            CommandSource::Workflow,
            Ok(vec![command(
                "deploy",
                &["release"],
                CommandAvailability::Available,
                CommandAction::PromptExpansion("workflow deploy".to_string()),
            )]),
        ),
        (
            CommandSource::Skill,
            Ok(vec![command(
                "hidden",
                &[],
                CommandAvailability::Unavailable { reason: "server is disabled".to_string() },
                CommandAction::PromptExpansion(String::new()),
            )]),
        ),
        (
            CommandSource::Plugin,
            Ok(vec![command(
                "unsafe",
                &[],
                CommandAvailability::Available,
                CommandAction::LocalEffect("bypass".to_string()),
            )]),
        ),
        (CommandSource::Skill, Err("skill discovery failed".to_string())),
    ]);

    assert!(registry.resolve("hidden").is_some());
    assert!(registry.dispatch("hidden", None).unwrap_err().contains("server is disabled"));
    assert!(registry.resolve("unsafe").is_none());
    assert_eq!(registry.resolve("release").unwrap().source, CommandSource::Workflow);
    assert!(registry.resolve("ship").is_none(), "losing aliases do not survive conflicts");
    let invocation = registry.dispatch("release", Some("--prod now".to_string())).unwrap();
    assert_eq!(invocation.name, "deploy");
    assert_eq!(invocation.argument.as_deref(), Some("--prod now"));
    assert_eq!(invocation.action, CommandAction::PromptExpansion("workflow deploy".to_string()));
    assert!(registry.dispatch("missing", None).unwrap_err().contains("unknown slash command"));
    assert!(registry.errors().iter().any(|error| error.contains("privileged local effect")));
    assert!(registry.errors().iter().any(|error| error.contains("skill discovery failed")));
}

#[test]
fn merged_registry_preserves_availability_argument_and_execution_metadata() {
    let registry = CommandRegistry::merge(vec![
        (CommandSource::Builtin, Ok(CommandRegistry::builtin().commands().to_vec())),
        (
            CommandSource::Mcp,
            Ok(vec![CommandSpec {
                name: "inspect".to_string(),
                aliases: vec!["show".to_string()],
                summary: "inspect a gateway resource".to_string(),
                takes_argument: true,
                argument_hint: Some("<resource-id>".to_string()),
                source: CommandSource::Mcp,
                availability: CommandAvailability::Available,
                execution_policy: CommandExecutionPolicy::Immediate,
                action: CommandAction::Gateway("mcp.inspect".to_string()),
            }]),
        ),
    ]);

    let spec = registry.resolve("show").expect("alias should resolve");
    assert_eq!(spec.argument_hint.as_deref(), Some("<resource-id>"));
    assert_eq!(spec.execution_policy, CommandExecutionPolicy::Immediate);

    let invocation =
        registry.dispatch("show", Some("resource-7".to_string())).expect("dispatch should work");
    assert_eq!(invocation.source, CommandSource::Mcp);
    assert_eq!(invocation.execution_policy, CommandExecutionPolicy::Immediate);
    assert_eq!(invocation.argument.as_deref(), Some("resource-7"));
}

#[test]
fn dynamic_sources_cannot_register_builtin_or_external_effects() {
    let registry = CommandRegistry::merge(vec![(
        CommandSource::Plugin,
        Ok(vec![
            command(
                "pretend-exit",
                &[],
                CommandAvailability::Available,
                CommandAction::Builtin(crate::cli::tui_v2::input::TuiSlashCommandKind::Exit),
            ),
            command(
                "open-browser",
                &[],
                CommandAvailability::Available,
                CommandAction::ExternalEffect("browser".to_string()),
            ),
        ]),
    )]);

    assert!(registry.resolve("pretend-exit").is_none());
    assert!(registry.resolve("open-browser").is_none());
    assert_eq!(
        registry.errors().iter().filter(|error| error.contains("privileged local effect")).count(),
        2
    );
}

#[test]
fn builtin_registry_exposes_direct_model_and_acp_selection_entries() {
    let registry = CommandRegistry::builtin();

    let model = registry.resolve("model").expect("/model is registered");
    assert!(!model.takes_argument);
    assert_eq!(model.argument_hint, None);
    assert_eq!(model.execution_policy, CommandExecutionPolicy::Queued);

    let acp = registry.resolve("acp").expect("/acp is registered");
    assert!(!acp.takes_argument);
    assert_eq!(acp.argument_hint, None);
    assert_eq!(acp.execution_policy, CommandExecutionPolicy::Queued);
}
