//! 面向用户命令行入口的公共规划接口。
//!
//! 本模块负责把配置中的 agent 规格、顶层动词和全局标志装配为稳定的
//! 用户可见命令行界面，并产出后续执行阶段需要的计划对象。
//!
//! 它关注“命令应该如何呈现和分流”，而不是“命令最终如何执行”。
//! 因此这里主要处理公共 CLI 布局、动态 agent 命令解析和根级 prompt 动作判断。

use std::collections::HashSet;

use crate::{ResolvedAcpxConfig, list_built_in_agents};

pub const PUBLIC_CLI_HELP_TEXT: &str = r#"
Examples:
  vwacp pi "review recent changes"
  vwacp openclaw exec "summarize active session state"
  vwacp codex sessions new
  vwacp codex "fix the tests"
  vwacp codex prompt "fix the tests"
  vwacp codex --no-wait "queue follow-up task"
  vwacp codex exec "what does this repo do"
  vwacp codex cancel
  vwacp codex set-mode plan
  vwacp codex set thought_level high
  vwacp codex -s backend "fix the API"
  vwacp codex sessions
  vwacp codex sessions new --name backend
  vwacp codex sessions ensure --name backend
  vwacp codex sessions close backend
  vwacp codex status
  vwacp config show
  vwacp config init
  vwacp --ttl 30 codex "investigate flaky tests"
  vwacp claude "refactor auth"
  vwacp --agent ./my-custom-server "do something""#;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentTokenScan {
    pub token: Option<String>,
    pub has_agent_override: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PublicCliPlan {
    pub agent_commands: Vec<String>,
    pub dynamic_agent_command: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootPromptAction {
    ShowHelp,
    HandlePrompt,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct PublicCliError {
    message: String,
}

impl PublicCliError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

pub struct ConfigurePublicCliOptions<
    'a,
    Program,
    RegisterAgentCommand,
    RegisterDefaultCommands,
    RegisterRootPrompt,
    AddHelpText,
> where
    RegisterAgentCommand: FnMut(&mut Program, &str, &ResolvedAcpxConfig),
    RegisterDefaultCommands: FnMut(&mut Program, &ResolvedAcpxConfig),
    RegisterRootPrompt: FnMut(&mut Program, bool),
    AddHelpText: FnMut(&mut Program, &'static str),
{
    pub program: &'a mut Program,
    pub argv: &'a [String],
    pub config: &'a ResolvedAcpxConfig,
    pub requested_json_strict: bool,
    pub top_level_verbs: &'a HashSet<String>,
    pub register_agent_command: RegisterAgentCommand,
    pub register_default_commands: RegisterDefaultCommands,
    pub register_root_prompt: RegisterRootPrompt,
    pub add_help_text: AddHelpText,
}

pub fn configure_public_cli<
    Program,
    RegisterAgentCommand,
    RegisterDefaultCommands,
    RegisterRootPrompt,
    AddHelpText,
>(
    mut options: ConfigurePublicCliOptions<
        '_,
        Program,
        RegisterAgentCommand,
        RegisterDefaultCommands,
        RegisterRootPrompt,
        AddHelpText,
    >,
) -> PublicCliPlan
where
    RegisterAgentCommand: FnMut(&mut Program, &str, &ResolvedAcpxConfig),
    RegisterDefaultCommands: FnMut(&mut Program, &ResolvedAcpxConfig),
    RegisterRootPrompt: FnMut(&mut Program, bool),
    AddHelpText: FnMut(&mut Program, &'static str),
{
    let plan = build_public_cli_plan(options.argv, options.config, options.top_level_verbs);

    for agent_name in &plan.agent_commands {
        (options.register_agent_command)(options.program, agent_name, options.config);
    }

    (options.register_default_commands)(options.program, options.config);

    if let Some(agent_name) = plan.dynamic_agent_command.as_deref() {
        (options.register_agent_command)(options.program, agent_name, options.config);
    }

    (options.register_root_prompt)(options.program, options.requested_json_strict);
    (options.add_help_text)(options.program, PUBLIC_CLI_HELP_TEXT);

    plan
}

pub fn build_public_cli_plan(
    argv: &[String],
    config: &ResolvedAcpxConfig,
    top_level_verbs: &HashSet<String>,
) -> PublicCliPlan {
    let agent_commands = list_built_in_agents(Some(&config.agents));
    let dynamic_agent_command =
        resolve_dynamic_agent_command(&detect_agent_token(argv), &agent_commands, top_level_verbs);

    PublicCliPlan { agent_commands, dynamic_agent_command }
}

pub fn resolve_dynamic_agent_command(
    scan: &AgentTokenScan,
    agent_commands: &[String],
    top_level_verbs: &HashSet<String>,
) -> Option<String> {
    if scan.has_agent_override {
        return None;
    }

    let token = scan.token.as_deref()?;
    if top_level_verbs.contains(token) || agent_commands.iter().any(|agent| agent == token) {
        return None;
    }
    Some(token.to_string())
}

pub fn resolve_root_prompt_action(
    prompt_parts: &[String],
    stdin_is_tty: bool,
    requested_json_strict: bool,
) -> Result<RootPromptAction, PublicCliError> {
    if prompt_parts.is_empty() && stdin_is_tty {
        if requested_json_strict {
            return Err(PublicCliError::new(
                "Prompt is required (pass as argument, --file, or pipe via stdin)",
            ));
        }
        return Ok(RootPromptAction::ShowHelp);
    }

    Ok(RootPromptAction::HandlePrompt)
}

pub fn detect_agent_token(argv: &[String]) -> AgentTokenScan {
    let mut has_agent_override = false;
    let mut index = 0;

    while index < argv.len() {
        let token = argv[index].as_str();

        if token == "--" {
            break;
        }

        if !token.starts_with('-') || token == "-" {
            return AgentTokenScan { token: Some(token.to_string()), has_agent_override };
        }

        if token == "--agent" {
            has_agent_override = true;
            index += 2;
            continue;
        }

        if token.starts_with("--agent=") {
            has_agent_override = true;
            index += 1;
            continue;
        }

        if matches!(
            token,
            "--cwd"
                | "--auth-policy"
                | "--non-interactive-permissions"
                | "--format"
                | "--model"
                | "--allowed-tools"
                | "--max-turns"
                | "--timeout"
                | "--ttl"
                | "--file"
        ) {
            index += 2;
            continue;
        }

        if token.starts_with("--cwd=")
            || token.starts_with("--auth-policy=")
            || token.starts_with("--non-interactive-permissions=")
            || token.starts_with("--format=")
            || token.starts_with("--model=")
            || token.starts_with("--allowed-tools=")
            || token.starts_with("--max-turns=")
            || token.starts_with("--json-strict=")
            || token.starts_with("--timeout=")
            || token.starts_with("--ttl=")
            || token.starts_with("--file=")
        {
            index += 1;
            continue;
        }

        if matches!(
            token,
            "--approve-all"
                | "--approve-reads"
                | "--deny-all"
                | "--json-strict"
                | "--verbose"
                | "--suppress-reads"
        ) {
            index += 1;
            continue;
        }

        return AgentTokenScan { token: None, has_agent_override };
    }

    AgentTokenScan { token: None, has_agent_override }
}

#[cfg(test)]
#[path = "cli_public_tests.rs"]
mod cli_public_tests;
