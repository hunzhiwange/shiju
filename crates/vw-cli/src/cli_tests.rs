#![allow(unused_imports, dead_code)]

//! `clap` CLI 定义的端到端解析测试。
//! 覆盖全局选项、Agent、Gateway、安全停止、Doctor、Task 与 Matrix 等子命令的默认值和显式参数。
//! 还验证 shell 补全渲染及写入失败传播，并约束脚本引用当前 Rust worker 入口。

use clap::{CommandFactory, Parser};
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

struct FlushFailWriter {
    bytes: Vec<u8>,
}

impl Write for FlushFailWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("intentional flush failure"))
    }
}

#[test]
fn cli_definition_builds_without_panicking() {
    let command = crate::cli::Cli::command();
    assert_eq!(command.get_name(), "shiju");
}

#[test]
fn startup_help_lists_only_root_commands() {
    let error = crate::cli::Cli::command()
        .try_get_matches_from(["shiju"])
        .expect_err("a top-level command is required");
    let help = error.to_string();

    assert_eq!(help.matches("Commands:").count(), 1);
    assert!(help.contains("  apiql "));
    assert!(help.contains("  server "));
    assert!(!help.contains("apiql list"));
    assert!(!help.contains("server install"));

    let error = crate::cli::Cli::command()
        .try_get_matches_from(["shiju", "apiql"])
        .expect_err("an APIQL subcommand is required");
    let apiql_help = error.to_string();

    assert!(apiql_help.contains("list"));

    let error = crate::cli::Cli::command()
        .try_get_matches_from(["shiju", "apiql", "list", "--help"])
        .expect_err("leaf help should stop command execution");
    assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
    assert!(error.to_string().contains("Usage: shiju apiql list"));
}

#[test]
fn every_visible_nested_command_supports_contextual_help() {
    fn collect_paths(
        command: &clap::Command,
        prefix: &mut Vec<String>,
        paths: &mut Vec<Vec<String>>,
    ) {
        for subcommand in command.get_subcommands().filter(|command| !command.is_hide_set()) {
            prefix.push(subcommand.get_name().to_string());
            paths.push(prefix.clone());
            collect_paths(subcommand, prefix, paths);
            prefix.pop();
        }
    }

    let mut paths = Vec::new();
    collect_paths(&crate::cli::Cli::command(), &mut Vec::new(), &mut paths);
    assert!(paths.iter().any(|path| path == &["apiql", "list"]));

    for path in paths {
        let mut arguments = vec!["shiju".to_string(), "help".to_string()];
        arguments.extend(path.iter().cloned());
        let error = crate::cli::Cli::command()
            .try_get_matches_from(arguments)
            .expect_err("help should stop command execution");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::DisplayHelp,
            "contextual help failed for command path: {path:?}: {error}"
        );
    }
}

#[test]
fn completion_shell_names_parse_to_each_supported_variant() {
    use crate::cli::CompletionShell;

    for (name, expected) in [
        ("bash", CompletionShell::Bash),
        ("fish", CompletionShell::Fish),
        ("zsh", CompletionShell::Zsh),
        ("powershell", CompletionShell::PowerShell),
        ("elvish", CompletionShell::Elvish),
    ] {
        let cli = crate::cli::Cli::try_parse_from(["vibewindow", "completions", name])
            .expect("supported completion shell should parse");
        let crate::cli::Commands::Completions { shell } = cli.command else {
            panic!("expected completions command");
        };
        assert_eq!(shell, expected);
    }
}

#[test]
fn shell_completion_generation_covers_every_renderer() {
    use crate::cli::CompletionShell;

    let mut rendered_scripts = Vec::new();
    for shell in [
        CompletionShell::Bash,
        CompletionShell::Fish,
        CompletionShell::Zsh,
        CompletionShell::PowerShell,
        CompletionShell::Elvish,
    ] {
        let mut output = Vec::new();
        crate::cli::write_shell_completion(shell, &mut output)
            .expect("completion generation should write to memory");
        let script = String::from_utf8(output).expect("completion output should be UTF-8");
        assert!(script.contains("shiju"));
        rendered_scripts.push(script);
    }

    for (index, script) in rendered_scripts.iter().enumerate() {
        assert!(
            rendered_scripts[..index].iter().all(|other| other != script),
            "each shell should use a distinct completion renderer"
        );
    }
}

#[test]
fn shell_completion_propagates_writer_flush_failure() {
    let mut writer = FlushFailWriter { bytes: Vec::new() };

    let error = crate::cli::write_shell_completion(crate::cli::CompletionShell::Bash, &mut writer)
        .expect_err("flush failure should be returned");

    assert!(!writer.bytes.is_empty(), "generation should precede flushing");
    assert!(error.to_string().contains("intentional flush failure"));
}

#[test]
fn api_command_help_and_completions_cover_every_family() {
    let families = [
        "apiql",
        "config",
        "provider",
        "control",
        "operations",
        "interactive",
        "project",
        "realtime",
        "knowledge",
        "task-pool",
        "task-lifecycle",
        "scheduler",
        "session",
        "session-content",
        "shiju-apps",
        "skill-market",
        "extensions",
        "desktop",
        "staff",
        "workflow",
        "workspace",
        "system",
    ];
    let command = crate::cli::Cli::command();
    let names = command
        .get_subcommands()
        .map(clap::Command::get_name)
        .collect::<std::collections::HashSet<_>>();
    for family in families {
        assert!(names.contains(family), "root help omits API family {family}");
    }

    let mut completion = Vec::new();
    crate::cli::write_shell_completion(crate::cli::CompletionShell::Bash, &mut completion)
        .expect("Bash completion");
    let completion = String::from_utf8(completion).expect("UTF-8 completion");
    for family in families {
        assert!(completion.contains(family), "completion omits api family {family}");
    }
}

#[test]
fn global_config_dir_parses_before_or_after_subcommand() {
    for arguments in [
        ["vibewindow", "--config-dir", "/tmp/config", "status"],
        ["vibewindow", "status", "--config-dir", "/tmp/config"],
    ] {
        let cli = crate::cli::Cli::try_parse_from(arguments)
            .expect("global config directory should parse in either position");
        assert_eq!(cli.config_dir.as_deref(), Some("/tmp/config"));
        assert!(matches!(cli.command, crate::cli::Commands::Status));
    }
}

#[test]
fn agent_command_preserves_defaults_and_explicit_values() {
    let defaults = crate::cli::Cli::try_parse_from(["vibewindow", "agent"])
        .expect("agent defaults should parse");
    let crate::cli::Commands::Agent {
        message,
        provider,
        model,
        temperature,
        peripheral,
        autonomy_level,
        max_actions_per_hour,
        max_iterations,
        max_history_messages,
        compact_context,
        memory_backend,
    } = defaults.command
    else {
        panic!("expected agent command");
    };
    assert_eq!(message, None);
    assert_eq!(provider, None);
    assert_eq!(model, None);
    assert_eq!(temperature, 0.7);
    assert!(peripheral.is_empty());
    assert_eq!(autonomy_level, None);
    assert_eq!(max_actions_per_hour, None);
    assert_eq!(max_iterations, None);
    assert_eq!(max_history_messages, None);
    assert!(!compact_context);
    assert_eq!(memory_backend, None);

    let explicit = crate::cli::Cli::try_parse_from([
        "vibewindow",
        "agent",
        "--message",
        "hello",
        "--provider",
        "openai",
        "--model",
        "gpt-test",
        "--temperature",
        "2",
        "--peripheral",
        "board:/dev/one",
        "--peripheral",
        "board:/dev/two",
        "--max-actions-per-hour",
        "12",
        "--max-iterations",
        "7",
        "--max-history-messages",
        "33",
        "--compact-context",
        "--memory-backend",
        "none",
    ])
    .expect("explicit agent options should parse");
    let crate::cli::Commands::Agent {
        message,
        provider,
        model,
        temperature,
        peripheral,
        max_actions_per_hour,
        max_iterations,
        max_history_messages,
        compact_context,
        memory_backend,
        ..
    } = explicit.command
    else {
        panic!("expected agent command");
    };
    assert_eq!(message.as_deref(), Some("hello"));
    assert_eq!(provider.as_deref(), Some("openai"));
    assert_eq!(model.as_deref(), Some("gpt-test"));
    assert_eq!(temperature, 2.0);
    assert_eq!(peripheral, ["board:/dev/one", "board:/dev/two"]);
    assert_eq!(max_actions_per_hour, Some(12));
    assert_eq!(max_iterations, Some(7));
    assert_eq!(max_history_messages, Some(33));
    assert!(compact_context);
    assert_eq!(memory_backend.as_deref(), Some("none"));
}

#[test]
fn agent_temperature_rejects_non_finite_out_of_range_and_non_numeric_values() {
    for value in ["-0.1", "2.1", "NaN", "not-a-number"] {
        assert!(
            crate::cli::Cli::try_parse_from([
                "vibewindow",
                "agent",
                &format!("--temperature={value}"),
            ])
            .is_err(),
            "temperature {value:?} should be rejected"
        );
    }
}

#[test]
fn gateway_and_daemon_commands_parse_endpoints_and_reject_invalid_ports() {
    let gateway = crate::cli::Cli::try_parse_from([
        "vibewindow",
        "gateway",
        "--port",
        "0",
        "--host",
        "127.0.0.1",
    ])
    .expect("gateway endpoint should parse");
    assert!(matches!(
        gateway.command,
        crate::cli::Commands::Gateway {
            port: Some(0),
            host: Some(ref host),
        } if host == "127.0.0.1"
    ));

    let daemon = crate::cli::Cli::try_parse_from([
        "vibewindow",
        "daemon",
        "--port",
        "65535",
        "--host",
        "0.0.0.0",
    ])
    .expect("daemon endpoint should parse");
    assert!(matches!(
        daemon.command,
        crate::cli::Commands::Daemon {
            port: Some(65535),
            host: Some(ref host),
        } if host == "0.0.0.0"
    ));

    for command in ["gateway", "daemon"] {
        assert!(
            crate::cli::Cli::try_parse_from(["vibewindow", command, "--port", "65536"]).is_err(),
            "{command} should reject a port outside the u16 range"
        );
    }
}

#[test]
fn estop_command_parses_each_level_repeated_targets_and_resume_options() {
    use crate::cli::EstopLevelArg;

    for (name, expected) in [
        ("kill-all", EstopLevelArg::KillAll),
        ("network-kill", EstopLevelArg::NetworkKill),
        ("domain-block", EstopLevelArg::DomainBlock),
        ("tool-freeze", EstopLevelArg::ToolFreeze),
    ] {
        let cli = crate::cli::Cli::try_parse_from(["vibewindow", "estop", "--level", name])
            .expect("supported estop level should parse");
        assert!(matches!(
            cli.command,
            crate::cli::Commands::Estop {
                level: Some(level),
                ..
            } if level == expected
        ));
    }

    let targets = crate::cli::Cli::try_parse_from([
        "vibewindow",
        "estop",
        "--level",
        "domain-block",
        "--domain",
        "example.com",
        "--domain",
        "internal.test",
        "--tool",
        "shell",
        "--tool",
        "browser",
    ])
    .expect("repeated estop targets should parse");
    let crate::cli::Commands::Estop { domains, tools, .. } = targets.command else {
        panic!("expected estop command");
    };
    assert_eq!(domains, ["example.com", "internal.test"]);
    assert_eq!(tools, ["shell", "browser"]);

    let resume = crate::cli::Cli::try_parse_from([
        "vibewindow",
        "estop",
        "resume",
        "--network",
        "--domain",
        "example.com",
        "--tool",
        "shell",
        "--otp",
        "123456",
    ])
    .expect("estop resume options should parse");
    assert!(matches!(
        resume.command,
        crate::cli::Commands::Estop {
            estop_command: Some(crate::cli::EstopSubcommands::Resume {
                network: true,
                ref domains,
                ref tools,
                otp: Some(ref otp),
            }),
            ..
        } if domains.as_slice() == ["example.com"]
            && tools.as_slice() == ["shell"]
            && otp.as_str() == "123456"
    ));
}

#[test]
fn doctor_command_covers_optional_root_and_nested_defaults() {
    let root = crate::cli::Cli::try_parse_from(["vibewindow", "doctor"])
        .expect("doctor without a nested command should parse");
    assert!(matches!(root.command, crate::cli::Commands::Doctor { doctor_command: None }));

    let models = crate::cli::Cli::try_parse_from([
        "vibewindow",
        "doctor",
        "models",
        "--provider",
        "openai",
        "--use-cache",
    ])
    .expect("doctor models options should parse");
    assert!(matches!(
        models.command,
        crate::cli::Commands::Doctor {
            doctor_command: Some(crate::cli::DoctorCommands::Models {
                provider: Some(ref provider),
                use_cache: true,
            })
        } if provider == "openai"
    ));

    let traces = crate::cli::Cli::try_parse_from(["vibewindow", "doctor", "traces"])
        .expect("doctor traces defaults should parse");
    assert!(matches!(
        traces.command,
        crate::cli::Commands::Doctor {
            doctor_command: Some(crate::cli::DoctorCommands::Traces {
                id: None,
                event: None,
                contains: None,
                limit: 20,
            })
        }
    ));
}

#[test]
fn task_commands_parse_create_and_read_defaults_and_overrides() {
    let create = crate::cli::Cli::try_parse_from([
        "vibewindow",
        "task",
        "--project-dir",
        "/tmp/project",
        "create",
        "--prompt",
        "ship it",
        "--priority",
        "4",
        "--task-split",
        "follow-system",
        "--subtask",
        "first",
        "--subtask",
        "second",
    ])
    .expect("task create options should parse");
    let crate::cli::Commands::Task {
        project_dir,
        task_command:
            crate::cli::TaskCommands::Create { priority, prompt, task_split, subtasks, .. },
    } = create.command
    else {
        panic!("expected task create command");
    };
    assert_eq!(project_dir, "/tmp/project");
    assert_eq!(priority, 4);
    assert_eq!(prompt.as_deref(), Some("ship it"));
    assert_eq!(task_split, Some(crate::cli::TaskSplitCliMode::FollowSystem));
    assert_eq!(subtasks, ["first", "second"]);

    let read = crate::cli::Cli::try_parse_from(["vibewindow", "task", "read"])
        .expect("task read defaults should parse");
    assert!(matches!(
        read.command,
        crate::cli::Commands::Task {
            ref project_dir,
            task_command: crate::cli::TaskCommands::Read {
                id: None,
                status: None,
                include_archived: false,
                include_deleted: false,
                limit: 50,
            },
        } if project_dir == "."
    ));
}




#[test]
fn native_task_commands_use_global_agent_scheduler_store() {
    let source = include_str!("lib.rs");
    assert!(source.contains("create_native_task("));
    assert!(source.contains("load_native_task("));
    assert!(source.contains("list_native_tasks("));
    for forbidden in ["task::create_task(", "task::load_task(", "task::load_all_tasks("] {
        assert!(!source.contains(forbidden), "legacy native task call remains: {forbidden}");
    }
}
