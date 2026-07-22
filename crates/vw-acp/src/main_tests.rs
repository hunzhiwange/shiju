use clap::{Arg, ArgAction, Command};
use vw_acp::PublicCliPlan;
use vw_acp::cli::flags::GlobalFlagOptions;
use vw_acp::{AuthPolicy, NonInteractivePermissionPolicy, OutputFormat};

use super::{extract_global_flags, resolve_cli_dispatch};

fn prompt_command(name: &'static str) -> Command {
    Command::new(name).arg(Arg::new("prompt").action(ArgAction::Append))
}

fn session_command() -> Command {
    Command::new("sessions")
        .subcommand(Command::new("new").arg(Arg::new("name").long("name")))
        .subcommand(Command::new("ensure").arg(Arg::new("name").long("name")))
        .subcommand(Command::new("close").arg(Arg::new("name").required(true)))
        .subcommand(Command::new("list"))
        .subcommand(Command::new("show").arg(Arg::new("name")))
        .subcommand(
            Command::new("history").arg(Arg::new("name")).arg(Arg::new("limit").long("limit")),
        )
        .subcommand(Command::new("read").arg(Arg::new("name")))
}

fn agent_command(name: &'static str) -> Command {
    Command::new(name)
        .arg(Arg::new("prompt").action(ArgAction::Append))
        .subcommand(prompt_command("prompt"))
        .subcommand(prompt_command("exec"))
        .subcommand(Command::new("cancel"))
        .subcommand(Command::new("set-mode").arg(Arg::new("mode").required(true)))
        .subcommand(
            Command::new("set")
                .arg(Arg::new("key").required(true))
                .arg(Arg::new("value").required(true)),
        )
        .subcommand(session_command())
        .subcommand(Command::new("status"))
}

fn command() -> Command {
    Command::new("vwacp")
        .arg(Arg::new("agent").long("agent").global(true))
        .arg(Arg::new("cwd").long("cwd").global(true))
        .arg(Arg::new("auth-policy").long("auth-policy").global(true))
        .arg(
            Arg::new("non-interactive-permissions")
                .long("non-interactive-permissions")
                .global(true),
        )
        .arg(Arg::new("json-strict").long("json-strict").global(true).action(ArgAction::SetTrue))
        .arg(
            Arg::new("suppress-reads")
                .long("suppress-reads")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new("timeout").long("timeout").global(true))
        .arg(Arg::new("ttl").long("ttl").global(true))
        .arg(Arg::new("verbose").long("verbose").global(true).action(ArgAction::SetTrue))
        .arg(Arg::new("format").long("format").global(true))
        .arg(Arg::new("model").long("model").global(true))
        .arg(Arg::new("allowed-tools").long("allowed-tools").global(true))
        .arg(Arg::new("max-turns").long("max-turns").global(true))
        .arg(Arg::new("prompt-retries").long("prompt-retries").global(true))
        .arg(Arg::new("approve-all").long("approve-all").global(true).action(ArgAction::SetTrue))
        .arg(
            Arg::new("approve-reads").long("approve-reads").global(true).action(ArgAction::SetTrue),
        )
        .arg(Arg::new("deny-all").long("deny-all").global(true).action(ArgAction::SetTrue))
        .arg(Arg::new("session").short('s').long("session").global(true))
        .arg(Arg::new("no-wait").long("no-wait").global(true).action(ArgAction::SetTrue))
        .arg(Arg::new("file").short('f').long("file").global(true))
        .arg(Arg::new("root_prompt").action(ArgAction::Append).hide(true))
        .subcommand(agent_command("codex"))
        .subcommand(agent_command("dynamic-agent"))
        .subcommand(prompt_command("prompt"))
        .subcommand(prompt_command("exec"))
        .subcommand(Command::new("cancel"))
        .subcommand(Command::new("set-mode").arg(Arg::new("mode").required(true)))
        .subcommand(
            Command::new("set")
                .arg(Arg::new("key").required(true))
                .arg(Arg::new("value").required(true)),
        )
        .subcommand(session_command())
        .subcommand(Command::new("status"))
        .subcommand(
            Command::new("config")
                .subcommand(Command::new("show"))
                .subcommand(Command::new("init")),
        )
}

fn public_cli_plan() -> PublicCliPlan {
    PublicCliPlan {
        agent_commands: vec!["codex".to_string()],
        dynamic_agent_command: Some("dynamic-agent".to_string()),
    }
}

#[test]
fn extract_global_flags_parses_supported_values() {
    let matches = command().get_matches_from([
        "vwacp",
        "--agent",
        "codex",
        "--cwd",
        "/tmp/project",
        "--auth-policy",
        "fail",
        "--non-interactive-permissions",
        "fail",
        "--json-strict",
        "--suppress-reads",
        "--timeout",
        "30",
        "--ttl",
        "45",
        "--verbose",
        "--format",
        "json",
        "--model",
        "gpt",
        "--allowed-tools",
        "Read, Write",
        "--max-turns",
        "4",
        "--prompt-retries",
        "2",
        "--approve-reads",
    ]);

    let flags = extract_global_flags(&matches);

    assert_eq!(
        flags,
        GlobalFlagOptions {
            agent: Some("codex".to_string()),
            cwd: Some("/tmp/project".to_string()),
            auth_policy: Some(AuthPolicy::Fail),
            non_interactive_permissions: Some(NonInteractivePermissionPolicy::Fail),
            json_strict: true,
            suppress_reads: true,
            timeout: Some(30_000),
            ttl: Some(45_000),
            verbose: true,
            format: Some(OutputFormat::Json),
            model: Some("gpt".to_string()),
            allowed_tools: Some(vec!["Read".to_string(), "Write".to_string()]),
            max_turns: Some(4),
            prompt_retries: Some(2),
            approve_all: false,
            approve_reads: true,
            deny_all: false,
        }
    );
}

#[test]
fn extract_global_flags_ignores_invalid_optional_values() {
    let matches = command().get_matches_from([
        "vwacp",
        "--auth-policy",
        "unknown",
        "--non-interactive-permissions",
        "unknown",
        "--format",
        "xml",
        "--timeout",
        "not-a-number",
        "--ttl",
        "not-a-number",
        "--allowed-tools",
        " ",
        "--max-turns",
        "0",
        "--prompt-retries",
        "not-a-number",
    ]);

    let flags = extract_global_flags(&matches);

    assert_eq!(flags.auth_policy, None);
    assert_eq!(flags.non_interactive_permissions, None);
    assert_eq!(flags.format, None);
    assert_eq!(flags.timeout, None);
    assert_eq!(flags.ttl, None);
    assert_eq!(flags.allowed_tools, None);
    assert_eq!(flags.max_turns, None);
    assert_eq!(flags.prompt_retries, None);
}

#[test]
fn extract_global_flags_uses_empty_defaults() {
    let matches = command().get_matches_from(["vwacp"]);

    let flags = extract_global_flags(&matches);

    assert_eq!(flags, GlobalFlagOptions::default());
}

#[test]
fn extract_global_flags_preserves_permission_switches() {
    let matches = command().get_matches_from(["vwacp", "--approve-all", "--deny-all"]);

    let flags = extract_global_flags(&matches);

    assert!(flags.approve_all);
    assert!(!flags.approve_reads);
    assert!(flags.deny_all);
}

#[test]
fn extract_global_flags_reads_values_after_agent_subcommand() {
    let matches = command().get_matches_from([
        "vwacp",
        "codex",
        "status",
        "--cwd",
        "/tmp/nested-project",
        "--format",
        "json",
        "--verbose",
    ]);

    let flags = extract_global_flags(&matches);

    assert_eq!(flags.cwd.as_deref(), Some("/tmp/nested-project"));
    assert_eq!(flags.format, Some(OutputFormat::Json));
    assert!(flags.verbose);
}

#[test]
fn resolve_cli_dispatch_defaults_empty_root_to_prompt() {
    let matches = command().get_matches_from(["vwacp"]);
    let dispatch = resolve_cli_dispatch(&matches, &public_cli_plan());

    assert_eq!(dispatch.command_to_run, "prompt");
    assert_eq!(dispatch.agent_name, None);
    assert!(dispatch.prompt_parts.is_empty());
    assert!(dispatch.command_matches.is_none());
}

#[test]
fn resolve_cli_dispatch_collects_root_prompt_parts() {
    let matches = command().get_matches_from(["vwacp", "fix", "tests"]);
    let dispatch = resolve_cli_dispatch(&matches, &public_cli_plan());

    assert_eq!(dispatch.command_to_run, "prompt");
    assert_eq!(dispatch.agent_name, None);
    assert_eq!(dispatch.prompt_parts, vec!["fix".to_string(), "tests".to_string()]);
    assert!(dispatch.command_matches.is_none());
}

#[test]
fn resolve_cli_dispatch_collects_top_level_prompt_parts() {
    let matches = command().get_matches_from(["vwacp", "prompt", "write", "summary"]);
    let dispatch = resolve_cli_dispatch(&matches, &public_cli_plan());

    assert_eq!(dispatch.command_to_run, "prompt");
    assert_eq!(dispatch.agent_name, None);
    assert_eq!(dispatch.prompt_parts, vec!["write".to_string(), "summary".to_string()]);
}

#[test]
fn resolve_cli_dispatch_preserves_empty_prompt_argument() {
    let matches = command().get_matches_from(["vwacp", "prompt", ""]);
    let dispatch = resolve_cli_dispatch(&matches, &public_cli_plan());

    assert_eq!(dispatch.command_to_run, "prompt");
    assert_eq!(dispatch.agent_name, None);
    assert_eq!(dispatch.prompt_parts, vec![String::new()]);
}

#[test]
fn resolve_cli_dispatch_accepts_empty_top_level_prompt() {
    let matches = command().get_matches_from(["vwacp", "prompt"]);
    let dispatch = resolve_cli_dispatch(&matches, &public_cli_plan());

    assert_eq!(dispatch.command_to_run, "prompt");
    assert_eq!(dispatch.agent_name, None);
    assert!(dispatch.prompt_parts.is_empty());
    assert!(dispatch.command_matches.is_some());
}

#[test]
fn resolve_cli_dispatch_collects_top_level_exec_prompt_parts() {
    let matches = command().get_matches_from(["vwacp", "exec", "inspect", "repo"]);
    let dispatch = resolve_cli_dispatch(&matches, &public_cli_plan());

    assert_eq!(dispatch.command_to_run, "exec");
    assert_eq!(dispatch.agent_name, None);
    assert_eq!(dispatch.prompt_parts, vec!["inspect".to_string(), "repo".to_string()]);
}

#[test]
fn resolve_cli_dispatch_maps_agent_without_subcommand_to_prompt() {
    let matches = command().get_matches_from(["vwacp", "codex", "fix", "coverage"]);
    let dispatch = resolve_cli_dispatch(&matches, &public_cli_plan());

    assert_eq!(dispatch.command_to_run, "prompt");
    assert_eq!(dispatch.agent_name, Some("codex".to_string()));
    assert_eq!(dispatch.prompt_parts, vec!["fix".to_string(), "coverage".to_string()]);
}

#[test]
fn resolve_cli_dispatch_maps_empty_agent_command_to_prompt() {
    let matches = command().get_matches_from(["vwacp", "codex"]);
    let dispatch = resolve_cli_dispatch(&matches, &public_cli_plan());

    assert_eq!(dispatch.command_to_run, "prompt");
    assert_eq!(dispatch.agent_name, Some("codex".to_string()));
    assert!(dispatch.prompt_parts.is_empty());
}

#[test]
fn resolve_cli_dispatch_collects_agent_nested_prompt_parts() {
    let matches = command().get_matches_from(["vwacp", "codex", "prompt", "ship", "it"]);
    let dispatch = resolve_cli_dispatch(&matches, &public_cli_plan());

    assert_eq!(dispatch.command_to_run, "prompt");
    assert_eq!(dispatch.agent_name, Some("codex".to_string()));
    assert_eq!(dispatch.prompt_parts, vec!["ship".to_string(), "it".to_string()]);
}

#[test]
fn resolve_cli_dispatch_collects_agent_nested_exec_prompt_parts() {
    let matches = command().get_matches_from(["vwacp", "codex", "exec", "explain", "main"]);
    let dispatch = resolve_cli_dispatch(&matches, &public_cli_plan());

    assert_eq!(dispatch.command_to_run, "exec");
    assert_eq!(dispatch.agent_name, Some("codex".to_string()));
    assert_eq!(dispatch.prompt_parts, vec!["explain".to_string(), "main".to_string()]);
}

#[test]
fn resolve_cli_dispatch_preserves_agent_command_matches() {
    let matches = command().get_matches_from(["vwacp", "codex", "set-mode", "plan"]);
    let dispatch = resolve_cli_dispatch(&matches, &public_cli_plan());
    let command_matches = dispatch.command_matches.expect("command matches");

    assert_eq!(dispatch.command_to_run, "set-mode");
    assert_eq!(dispatch.agent_name, Some("codex".to_string()));
    assert_eq!(command_matches.get_one::<String>("mode").map(String::as_str), Some("plan"));
}

#[test]
fn resolve_cli_dispatch_respects_dynamic_agent_command() {
    let matches = command().get_matches_from(["vwacp", "dynamic-agent", "status"]);
    let dispatch = resolve_cli_dispatch(&matches, &public_cli_plan());

    assert_eq!(dispatch.command_to_run, "status");
    assert_eq!(dispatch.agent_name, Some("dynamic-agent".to_string()));
}

#[test]
fn resolve_cli_dispatch_preserves_agent_sessions_subcommand() {
    let matches = command().get_matches_from([
        "vwacp",
        "codex",
        "sessions",
        "history",
        "named-session",
        "--limit",
        "5",
    ]);
    let dispatch = resolve_cli_dispatch(&matches, &public_cli_plan());
    let command_matches = dispatch.command_matches.expect("sessions command matches");
    let (subcommand, subcommand_matches) =
        command_matches.subcommand().expect("sessions subcommand");

    assert_eq!(dispatch.command_to_run, "sessions");
    assert_eq!(dispatch.agent_name, Some("codex".to_string()));
    assert_eq!(subcommand, "history");
    assert_eq!(
        subcommand_matches.get_one::<String>("name").map(String::as_str),
        Some("named-session")
    );
    assert_eq!(subcommand_matches.get_one::<String>("limit").map(String::as_str), Some("5"));
}

#[test]
fn resolve_cli_dispatch_does_not_infer_unplanned_agent_command() {
    let matches = command().get_matches_from(["vwacp", "codex", "status"]);
    let plan = PublicCliPlan { agent_commands: vec![], dynamic_agent_command: None };
    let dispatch = resolve_cli_dispatch(&matches, &plan);

    assert_eq!(dispatch.command_to_run, "codex");
    assert_eq!(dispatch.agent_name, None);
    assert!(dispatch.prompt_parts.is_empty());
    assert_eq!(
        dispatch.command_matches.as_ref().and_then(|matches| matches.subcommand_name()),
        Some("status")
    );
}

#[test]
fn resolve_cli_dispatch_keeps_top_level_sessions_command() {
    let matches = command().get_matches_from(["vwacp", "sessions", "list"]);
    let dispatch = resolve_cli_dispatch(&matches, &public_cli_plan());
    let command_matches = dispatch.command_matches.expect("command matches");

    assert_eq!(dispatch.command_to_run, "sessions");
    assert_eq!(dispatch.agent_name, None);
    assert_eq!(command_matches.subcommand_name(), Some("list"));
    assert!(dispatch.prompt_parts.is_empty());
}
