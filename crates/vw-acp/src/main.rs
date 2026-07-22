//! vw-acp 命令行程序入口。
//!
//! 本模块负责把进程启动参数接入到库层规划逻辑中，并根据运行模式决定：
//! - 输出版本信息
//! - 以前台 CLI 方式执行一次请求
//! - 以队列所有者模式启动后台进程
//!
//! 它本身尽量保持薄入口，只负责环境读取、命令树注册和最终分发，
//! 具体的参数解析、配置合并和会话执行逻辑下沉到库模块中实现。

use clap::ArgMatches;
#[cfg(not(test))]
use clap::{Arg, ArgAction, Command};
#[cfg(not(test))]
use std::{env, io::IsTerminal};
use vw_acp::cli::flags::{
    GlobalFlagOptions, parse_allowed_tools, parse_auth_policy, parse_max_turns,
    parse_non_interactive_permission_policy, parse_output_format, parse_prompt_retries,
    parse_timeout_seconds, parse_ttl_seconds,
};
#[cfg(not(test))]
use vw_acp::{
    ConfigurePublicCliOptions, FindSessionOptions, OutputFormatterOptions, SessionSendOptions,
    build_cli_bootstrap_plan, build_cli_runtime_plan,
    cli::{
        config_command::{ConfigCommand, handle_config_command},
        flags::{
            PermissionFlags, StatusFlags, resolve_agent_invocation, resolve_global_flags,
            resolve_permission_mode,
        },
        output_render::{
            print_cancel_result_by_format, print_closed_session_by_format,
            print_created_session_banner, print_ensured_session_by_format,
            print_new_session_by_format, print_prompt_session_banner, print_sessions_by_format,
            print_set_config_option_result_by_format, print_set_mode_result_by_format,
            print_set_model_result_by_format,
        },
        status_command::{handle_sessions_history, handle_sessions_show, handle_status},
    },
    close_session, configure_public_cli, create_output_formatter, default_global_config_path,
    find_session, load_resolved_config_from_paths, project_config_path, read_prompt,
    run_queue_owner_from_env, send_session, top_level_verbs,
};

fn extract_global_flags(matches: &ArgMatches) -> GlobalFlagOptions {
    GlobalFlagOptions {
        agent: matches.get_one::<String>("agent").cloned(),
        cwd: matches.get_one::<String>("cwd").cloned(),
        auth_policy: matches
            .get_one::<String>("auth-policy")
            .and_then(|v| parse_auth_policy(v).ok()),
        non_interactive_permissions: matches
            .get_one::<String>("non-interactive-permissions")
            .and_then(|v| parse_non_interactive_permission_policy(v).ok()),
        json_strict: matches.get_flag("json-strict"),
        suppress_reads: matches.get_flag("suppress-reads"),
        timeout: matches.get_one::<String>("timeout").and_then(|v| parse_timeout_seconds(v).ok()),
        ttl: matches.get_one::<String>("ttl").and_then(|v| parse_ttl_seconds(v).ok()),
        verbose: matches.get_flag("verbose"),
        format: matches.get_one::<String>("format").and_then(|v| parse_output_format(v).ok()),
        model: matches.get_one::<String>("model").cloned(),
        allowed_tools: matches
            .get_one::<String>("allowed-tools")
            .and_then(|v| parse_allowed_tools(v).ok())
            .filter(|tools| !tools.is_empty()),
        max_turns: matches.get_one::<String>("max-turns").and_then(|v| parse_max_turns(v).ok()),
        prompt_retries: matches
            .get_one::<String>("prompt-retries")
            .and_then(|v| parse_prompt_retries(v).ok()),
        approve_all: matches.get_flag("approve-all"),
        approve_reads: matches.get_flag("approve-reads"),
        deny_all: matches.get_flag("deny-all"),
    }
}

struct CliDispatch {
    command_to_run: String,
    agent_name: Option<String>,
    prompt_parts: Vec<String>,
    command_matches: Option<ArgMatches>,
}

fn collect_prompt_parts(matches: &ArgMatches) -> Vec<String> {
    matches
        .get_many::<String>("prompt")
        .map(|values| values.map(|value| value.to_string()).collect())
        .unwrap_or_default()
}

fn resolve_cli_dispatch(matches: &ArgMatches, plan: &vw_acp::PublicCliPlan) -> CliDispatch {
    let (subcommand_name, inner_matches) = match matches.subcommand() {
        Some((name, inner)) => (Some(name), Some(inner)),
        None => (None, None),
    };

    let mut command_to_run = subcommand_name.map(ToString::to_string);
    let mut agent_name = None;
    let mut prompt_parts: Vec<String> = vec![];
    let mut command_matches = inner_matches.cloned();

    if let Some(name) = subcommand_name {
        let is_agent_command = plan.agent_commands.iter().any(|cmd| cmd == name)
            || plan.dynamic_agent_command.as_deref() == Some(name);
        if is_agent_command {
            agent_name = Some(name.to_string());
            if let Some(inner) = inner_matches {
                if let Some((inner_name, inner_inner)) = inner.subcommand() {
                    command_to_run = Some(inner_name.to_string());
                    command_matches = Some(inner_inner.clone());
                    if inner_name == "prompt" || inner_name == "exec" {
                        prompt_parts = collect_prompt_parts(inner_inner);
                    }
                } else {
                    command_to_run = Some("prompt".to_string());
                    prompt_parts = collect_prompt_parts(inner);
                }
            }
        } else if name == "prompt" || name == "exec" {
            prompt_parts = command_matches.as_ref().map(collect_prompt_parts).unwrap_or_default();
        }
    } else {
        command_to_run = Some("prompt".to_string());
        if let Some(vals) = matches.get_many::<String>("root_prompt") {
            prompt_parts = vals.map(|s| s.to_string()).collect();
        }
    }

    CliDispatch {
        command_to_run: command_to_run.unwrap_or_else(|| "prompt".to_string()),
        agent_name,
        prompt_parts,
        command_matches,
    }
}

#[cfg(not(test))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let current_dir = env::current_dir()?;

    let bootstrap_plan = build_cli_bootstrap_plan(&args, &current_dir);

    if bootstrap_plan.print_version {
        println!("vw-acp {}", vw_acp::get_vwacp_version());
        return Ok(());
    }

    if bootstrap_plan.queue_owner_mode {
        run_queue_owner_from_env().await?;
        return Ok(());
    }

    let global_config_path = default_global_config_path()?;
    let project_config_path = project_config_path(&bootstrap_plan.initial_cwd)?;

    let config = load_resolved_config_from_paths(&global_config_path, &project_config_path).await?;

    let runtime_plan = build_cli_runtime_plan(&bootstrap_plan.cli_args, &config);
    let top_verbs = top_level_verbs();
    let version = vw_acp::get_vwacp_version();

    let mut program = Command::new("vwacp")
        .version(Box::leak(version.into_boxed_str()) as &'static str)
        .about("VibeWindow Agent Client Protocol (ACP) CLI")
        .disable_help_subcommand(true)
        .arg(Arg::new("agent").long("agent").global(true))
        .arg(Arg::new("cwd").long("cwd").global(true))
        .arg(Arg::new("auth-policy").long("auth-policy").global(true))
        .arg(Arg::new("approve-all").long("approve-all").global(true).action(ArgAction::SetTrue))
        .arg(
            Arg::new("approve-reads").long("approve-reads").global(true).action(ArgAction::SetTrue),
        )
        .arg(Arg::new("deny-all").long("deny-all").global(true).action(ArgAction::SetTrue))
        .arg(
            Arg::new("non-interactive-permissions")
                .long("non-interactive-permissions")
                .global(true),
        )
        .arg(Arg::new("format").long("format").global(true))
        .arg(
            Arg::new("suppress-reads")
                .long("suppress-reads")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new("model").long("model").global(true))
        .arg(Arg::new("allowed-tools").long("allowed-tools").global(true))
        .arg(Arg::new("max-turns").long("max-turns").global(true))
        .arg(Arg::new("prompt-retries").long("prompt-retries").global(true))
        .arg(Arg::new("json-strict").long("json-strict").global(true).action(ArgAction::SetTrue))
        .arg(Arg::new("timeout").long("timeout").global(true))
        .arg(Arg::new("ttl").long("ttl").global(true))
        .arg(Arg::new("verbose").long("verbose").global(true).action(ArgAction::SetTrue))
        .arg(Arg::new("session").short('s').long("session").global(true))
        .arg(Arg::new("no-wait").long("no-wait").global(true).action(ArgAction::SetTrue))
        .arg(Arg::new("file").short('f').long("file").global(true));

    let plan = configure_public_cli(ConfigurePublicCliOptions {
        program: &mut program,
        argv: &bootstrap_plan.cli_args,
        config: &config,
        requested_json_strict: bootstrap_plan.requested_json_strict,
        top_level_verbs: &top_verbs,
        register_agent_command: |prog, agent_name, _cfg| {
            let agent_name = Box::leak(agent_name.to_string().into_boxed_str()) as &'static str;
            *prog = prog.clone().subcommand(
                Command::new(agent_name)
                    .about(format!("Run agent {}", agent_name))
                    .arg(Arg::new("prompt").action(ArgAction::Append))
                    .subcommand(
                        Command::new("prompt")
                            .about("Run a prompt")
                            .arg(Arg::new("prompt").action(ArgAction::Append)),
                    )
                    .subcommand(
                        Command::new("exec")
                            .about("Execute a command")
                            .arg(Arg::new("prompt").action(ArgAction::Append)),
                    )
                    .subcommand(Command::new("cancel").about("Cancel an active session"))
                    .subcommand(
                        Command::new("set-mode")
                            .about("Set session mode")
                            .arg(Arg::new("mode").required(true)),
                    )
                    .subcommand(
                        Command::new("set")
                            .about("Set session config option")
                            .arg(Arg::new("key").required(true))
                            .arg(Arg::new("value").required(true)),
                    )
                    .subcommand(
                        Command::new("sessions")
                            .about("Manage sessions")
                            .subcommand(Command::new("new").arg(Arg::new("name").long("name")))
                            .subcommand(Command::new("ensure").arg(Arg::new("name").long("name")))
                            .subcommand(Command::new("close").arg(Arg::new("name").required(true)))
                            .subcommand(Command::new("list"))
                            .subcommand(Command::new("show").arg(Arg::new("name")))
                            .subcommand(
                                Command::new("history")
                                    .arg(Arg::new("name"))
                                    .arg(Arg::new("limit").long("limit")),
                            )
                            .subcommand(Command::new("read").arg(Arg::new("name"))),
                    )
                    .subcommand(Command::new("status").about("Show status")),
            );
        },
        register_default_commands: |prog, _cfg| {
            *prog = prog
                .clone()
                .subcommand(
                    Command::new("prompt")
                        .about("Run a prompt")
                        .arg(Arg::new("prompt").action(ArgAction::Append)),
                )
                .subcommand(
                    Command::new("exec")
                        .about("Execute a command")
                        .arg(Arg::new("prompt").action(ArgAction::Append)),
                )
                .subcommand(Command::new("cancel").about("Cancel an active session"))
                .subcommand(
                    Command::new("set-mode")
                        .about("Set session mode")
                        .arg(Arg::new("mode").required(true)),
                )
                .subcommand(
                    Command::new("set")
                        .about("Set session config option")
                        .arg(Arg::new("key").required(true))
                        .arg(Arg::new("value").required(true)),
                )
                .subcommand(
                    Command::new("sessions")
                        .about("Manage sessions")
                        .subcommand(Command::new("new").arg(Arg::new("name").long("name")))
                        .subcommand(Command::new("ensure").arg(Arg::new("name").long("name")))
                        .subcommand(Command::new("close").arg(Arg::new("name").required(true)))
                        .subcommand(Command::new("list"))
                        .subcommand(Command::new("show").arg(Arg::new("name")))
                        .subcommand(
                            Command::new("history")
                                .arg(Arg::new("name"))
                                .arg(Arg::new("limit").long("limit")),
                        )
                        .subcommand(Command::new("read").arg(Arg::new("name"))),
                )
                .subcommand(Command::new("status").about("Show status"))
                .subcommand(
                    Command::new("config")
                        .about("Manage configuration")
                        .subcommand(Command::new("show"))
                        .subcommand(Command::new("init")),
                );
        },
        register_root_prompt: |prog, _strict| {
            *prog = prog.clone().arg(Arg::new("root_prompt").action(ArgAction::Append).hide(true));
        },
        add_help_text: |prog, text| {
            *prog = prog.clone().after_help(text);
        },
    });

    if args.len() <= 1 {
        let _ = program.print_help();
        return Ok(());
    }

    let matches = program.get_matches_from(&args);
    let CliDispatch { command_to_run, agent_name, prompt_parts, command_matches } =
        resolve_cli_dispatch(&matches, &plan);
    let command_to_run = command_to_run.as_str();

    let global_flag_options = extract_global_flags(&matches);
    let global_flags = resolve_global_flags(&global_flag_options, &config)?;
    let permission_mode = resolve_permission_mode(
        &PermissionFlags {
            approve_all: global_flag_options.approve_all,
            approve_reads: global_flag_options.approve_reads,
            deny_all: global_flag_options.deny_all,
        },
        config.default_permissions,
    )?;
    let output_policy = runtime_plan.requested_output_policy;
    let mut output_formatter = create_output_formatter(
        output_policy.format,
        std::io::stdout().lock(),
        OutputFormatterOptions {
            context: None,
            suppress_reads: output_policy.suppress_reads,
            is_tty: std::io::stdin().is_terminal(),
        },
    );

    let agent = resolve_agent_invocation(agent_name.as_deref(), &global_flags, &config)?;

    let session_name = matches.get_one::<String>("session").map(|s| s.to_string());

    if command_to_run == "prompt" {
        let prompt = read_prompt(
            &prompt_parts,
            matches.get_one::<String>("file").map(|s| s.as_str()),
            &global_flags.cwd,
            std::io::stdin().is_terminal(),
        )
        .await?;

        let git_root = vw_acp::find_git_repository_root(&agent.cwd)
            .map(|path| path.to_string_lossy().into_owned());
        let walk_boundary = git_root.unwrap_or_else(|| agent.cwd.clone());
        let existing =
            vw_acp::find_session_by_directory_walk(&vw_acp::FindSessionByDirectoryWalkOptions {
                agent_command: agent.agent_command.clone(),
                cwd: agent.cwd.clone(),
                name: session_name.clone(),
                boundary: Some(walk_boundary.clone()),
            })
            .await?;

        let record = match existing {
            Some(record) => record,
            None => {
                let create_cmd = if let Some(n) = &session_name {
                    format!(
                        "vwacp {} sessions new --name {}",
                        agent_name.as_deref().unwrap_or("codex"),
                        n
                    )
                } else {
                    format!("vwacp {} sessions new", agent_name.as_deref().unwrap_or("codex"))
                };
                eprintln!(
                    "⚠ No vwacp session found (searched up to {}).\nCreate one: {}",
                    walk_boundary, create_cmd
                );
                std::process::exit(1);
            }
        };

        print_prompt_session_banner(
            &record,
            &agent.cwd,
            output_policy.format,
            output_policy.json_strict,
        )
        .await
        .ok();

        let _ = send_session(SessionSendOptions {
            session_id: record.vwacp_record_id,
            prompt,
            resume_policy: None,
            mcp_servers: None,
            permission_mode,
            non_interactive_permissions: Some(global_flags.non_interactive_permissions),
            auth_credentials: None,
            auth_policy: global_flags.auth_policy,
            output_formatter: &mut output_formatter,
            on_acp_message: None,
            on_session_update: None,
            on_client_operation: None,
            error_emission_policy: None,
            suppress_sdk_console_errors: output_policy.suppress_sdk_console_errors,
            verbose: global_flags.verbose,
            wait_for_completion: !matches.get_flag("no-wait"),
            ttl_ms: Some(global_flags.ttl),
            max_queue_depth: None,
            prompt_retries: global_flags.prompt_retries,
            timeout_ms: global_flags.timeout,
        })
        .await?;
    } else if command_to_run == "exec" {
        let prompt = read_prompt(
            &prompt_parts,
            matches.get_one::<String>("file").map(|s| s.as_str()),
            &global_flags.cwd,
            std::io::stdin().is_terminal(),
        )
        .await?;

        let outcome = vw_acp::run_once(vw_acp::RunOnceOptions {
            agent_command: agent.agent_command.clone(),
            agent_config: agent.agent_config.clone(),
            cwd: agent.cwd.clone(),
            prompt,
            mcp_servers: None,
            permission_mode,
            non_interactive_permissions: Some(global_flags.non_interactive_permissions),
            auth_credentials: None,
            auth_policy: global_flags.auth_policy,
            output_formatter: &mut output_formatter,
            on_acp_message: None,
            on_session_update: None,
            on_client_operation: None,
            suppress_sdk_console_errors: output_policy.suppress_sdk_console_errors,
            verbose: global_flags.verbose,
            session_options: None,
            prompt_retries: global_flags.prompt_retries,
            timeout_ms: global_flags.timeout,
        })
        .await?;

        println!("Outcome: {:?}", outcome);
    } else if command_to_run == "status" {
        handle_status(agent_name.as_deref(), &StatusFlags::default(), &global_flags, &config)
            .await?;
    } else if command_to_run == "cancel" {
        let session_id = session_name.clone().unwrap_or_default();
        if session_id.is_empty() {
            eprintln!("⚠ Error: Session ID or name is required for cancel.");
            std::process::exit(1);
        }
        let outcome = vw_acp::cancel_session_prompt(vw_acp::SessionCancelOptions {
            session_id,
            verbose: global_flags.verbose,
        })
        .await?;
        print_cancel_result_by_format(outcome.cancelled, output_policy.format).ok();
    } else if command_to_run == "set-mode" {
        let session_id = session_name.clone().unwrap_or_default();
        let mode_id =
            command_matches.as_ref().unwrap().get_one::<String>("mode").unwrap().to_string();
        if session_id.is_empty() {
            eprintln!("⚠ Error: Session ID or name is required for set-mode.");
            std::process::exit(1);
        }
        let _outcome = vw_acp::set_session_mode(vw_acp::SessionSetModeOptions {
            session_id,
            mode_id: mode_id.clone(),
            mcp_servers: None,
            non_interactive_permissions: Some(global_flags.non_interactive_permissions),
            auth_credentials: None,
            auth_policy: global_flags.auth_policy,
            timeout_ms: global_flags.timeout,
            verbose: global_flags.verbose,
        })
        .await?;
        print_set_mode_result_by_format(&mode_id, output_policy.format).ok();
    } else if command_to_run == "set" {
        let session_id = session_name.clone().unwrap_or_default();
        let key = command_matches.as_ref().unwrap().get_one::<String>("key").unwrap().to_string();
        let value =
            command_matches.as_ref().unwrap().get_one::<String>("value").unwrap().to_string();
        if session_id.is_empty() {
            eprintln!("⚠ Error: Session ID or name is required for set.");
            std::process::exit(1);
        }
        if key == "model" {
            let _outcome = vw_acp::set_session_model(vw_acp::SessionSetModelOptions {
                session_id,
                model_id: value.clone(),
                mcp_servers: None,
                non_interactive_permissions: Some(global_flags.non_interactive_permissions),
                auth_credentials: None,
                auth_policy: global_flags.auth_policy,
                timeout_ms: global_flags.timeout,
                verbose: global_flags.verbose,
            })
            .await?;
            print_set_model_result_by_format(&value, output_policy.format).ok();
        } else {
            let _outcome =
                vw_acp::set_session_config_option(vw_acp::SessionSetConfigOptionOptions {
                    session_id,
                    config_id: key.clone(),
                    value: value.clone(),
                    mcp_servers: None,
                    non_interactive_permissions: Some(global_flags.non_interactive_permissions),
                    auth_credentials: None,
                    auth_policy: global_flags.auth_policy,
                    timeout_ms: global_flags.timeout,
                    verbose: global_flags.verbose,
                })
                .await?;
            print_set_config_option_result_by_format(&key, &value, output_policy.format).ok();
        }
    } else if command_to_run == "sessions" {
        if let Some(inner) = command_matches.as_ref().and_then(|m| m.subcommand()) {
            let (subcmd, sub_matches) = inner;
            if subcmd == "new" {
                let name = sub_matches.get_one::<String>("name").map(|s| s.to_string());
                let replaced = find_session(&FindSessionOptions {
                    agent_command: agent.agent_command.clone(),
                    cwd: agent.cwd.clone(),
                    name: name.clone(),
                    include_closed: false,
                })
                .await?;

                if let Some(existing) = replaced.as_ref() {
                    close_session(&existing.vwacp_record_id).await?;
                    if global_flags.verbose {
                        eprintln!(
                            "[vwacp] soft-closed prior session: {}",
                            existing.vwacp_record_id
                        );
                    }
                }

                let result = vw_acp::create_session(vw_acp::SessionCreateOptions {
                    agent_command: agent.agent_command.clone(),
                    agent_config: agent.agent_config.clone(),
                    cwd: agent.cwd.clone(),
                    name,
                    resume_session_id: None,
                    mcp_servers: None,
                    permission_mode,
                    non_interactive_permissions: Some(global_flags.non_interactive_permissions),
                    auth_credentials: None,
                    auth_policy: global_flags.auth_policy,
                    timeout_ms: global_flags.timeout,
                    verbose: global_flags.verbose,
                    session_options: None,
                })
                .await?;
                print_created_session_banner(
                    &result,
                    &agent.agent_name,
                    output_policy.format,
                    global_flags.json_strict,
                )
                .ok();
                print_new_session_by_format(&result, replaced.as_ref(), output_policy.format).ok();
            } else if subcmd == "ensure" {
                let name = sub_matches.get_one::<String>("name").map(|s| s.to_string());
                let result = vw_acp::ensure_session(vw_acp::SessionEnsureOptions {
                    agent_command: agent.agent_command.clone(),
                    agent_config: agent.agent_config.clone(),
                    cwd: agent.cwd.clone(),
                    name,
                    resume_session_id: None,
                    mcp_servers: None,
                    permission_mode,
                    non_interactive_permissions: Some(global_flags.non_interactive_permissions),
                    auth_credentials: None,
                    auth_policy: global_flags.auth_policy,
                    timeout_ms: global_flags.timeout,
                    verbose: global_flags.verbose,
                    walk_boundary: None,
                    session_options: None,
                })
                .await?;
                if result.created {
                    print_created_session_banner(
                        &result.record,
                        &agent.agent_name,
                        output_policy.format,
                        global_flags.json_strict,
                    )
                    .ok();
                }
                print_ensured_session_by_format(
                    &result.record,
                    result.created,
                    output_policy.format,
                )
                .ok();
            } else if subcmd == "close" {
                let name = sub_matches.get_one::<String>("name").unwrap().to_string();
                let result = vw_acp::close_session(&name).await?;
                print_closed_session_by_format(&result, output_policy.format).ok();
            } else if subcmd == "list" {
                let sessions = if let Some(ag) = agent_name.as_deref() {
                    vw_acp::list_sessions_for_agent(ag).await?
                } else {
                    vw_acp::list_sessions().await?
                };
                print_sessions_by_format(&sessions, output_policy.format).ok();
            } else if subcmd == "show" || subcmd == "read" {
                let name = sub_matches
                    .get_one::<String>("name")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| session_name.clone().unwrap_or_default());
                if name.is_empty() {
                    eprintln!("⚠ Error: Session ID or name is required.");
                    std::process::exit(1);
                }
                handle_sessions_show(agent_name.as_deref(), Some(&name), &global_flags, &config)
                    .await?;
            } else if subcmd == "history" {
                let name = sub_matches
                    .get_one::<String>("name")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| session_name.clone().unwrap_or_default());
                if name.is_empty() {
                    eprintln!("⚠ Error: Session ID or name is required.");
                    std::process::exit(1);
                }
                let limit = sub_matches
                    .get_one::<String>("limit")
                    .and_then(|l| l.parse::<usize>().ok())
                    .unwrap_or(20);
                handle_sessions_history(
                    agent_name.as_deref(),
                    Some(&name),
                    limit,
                    &global_flags,
                    &config,
                )
                .await?;
            } else {
                println!("Command sessions {} not fully implemented yet in main.rs", subcmd);
            }
        } else {
            let sessions = if let Some(ag) = agent_name.as_deref() {
                vw_acp::list_sessions_for_agent(ag).await?
            } else {
                vw_acp::list_sessions().await?
            };
            print_sessions_by_format(&sessions, output_policy.format).ok();
        }
    } else if command_to_run == "config" {
        if let Some(inner) = command_matches.as_ref().and_then(|m| m.subcommand_name()) {
            if inner == "show" {
                handle_config_command(ConfigCommand::Show, &global_flags, &config).await?;
            } else if inner == "init" {
                handle_config_command(ConfigCommand::Init, &global_flags, &config).await?;
            }
        } else {
            handle_config_command(ConfigCommand::Show, &global_flags, &config).await?;
        }
    } else {
        println!("Command {} not fully implemented yet in main.rs", command_to_run);
    }

    Ok(())
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod main_tests;
