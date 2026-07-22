use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{
    Config, build_engage_level, build_resume_selector, handle_estop_command, print_estop_status,
};
use crate::cli::{EstopLevelArg, EstopSubcommands};
use vw_agent::security::{EstopLevel, EstopState, ResumeSelector};

/// 测试临时目录，Drop 时自动清理。
struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!("vw-cli-estop-test-{n}"));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn config_path(&self) -> PathBuf {
        self.path.join("vibewindow.json")
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn make_enabled_config(dir: &TestDir) -> Config {
    let mut config = Config::default();
    config.config_path = dir.config_path();
    config.workspace_dir = dir.path.join("workspace");
    config.security.estop.enabled = true;
    config.security.estop.state_file = "estop_state.json".to_string();
    config.security.estop.require_otp_to_resume = false;
    config
}

// ============================================================
// build_engage_level
// ============================================================

#[test]
fn build_engage_level_validates_argument_combinations() {
    assert!(matches!(build_engage_level(None, vec![], vec![]).unwrap(), EstopLevel::KillAll));
    assert!(matches!(
        build_engage_level(Some(EstopLevelArg::NetworkKill), vec![], vec![]).unwrap(),
        EstopLevel::NetworkKill
    ));
    assert!(matches!(
        build_engage_level(
            Some(EstopLevelArg::DomainBlock),
            vec!["example.com".to_string()],
            vec![],
        )
        .unwrap(),
        EstopLevel::DomainBlock(domains) if domains == vec!["example.com".to_string()]
    ));
    assert!(build_engage_level(Some(EstopLevelArg::ToolFreeze), vec![], vec![]).is_err());
    assert!(build_engage_level(Some(EstopLevelArg::KillAll), vec!["x".into()], vec![]).is_err());
}

#[test]
fn build_engage_level_kill_all_rejects_domains_or_tools() {
    assert!(build_engage_level(Some(EstopLevelArg::KillAll), vec![], vec![]).is_ok());
    assert!(build_engage_level(Some(EstopLevelArg::KillAll), vec!["x".into()], vec![]).is_err());
    assert!(build_engage_level(Some(EstopLevelArg::KillAll), vec![], vec!["t".into()]).is_err());
    assert!(
        build_engage_level(Some(EstopLevelArg::KillAll), vec!["x".into()], vec!["t".into()],)
            .is_err()
    );
}

#[test]
fn build_engage_level_network_kill_rejects_extra_args() {
    assert!(matches!(
        build_engage_level(Some(EstopLevelArg::NetworkKill), vec![], vec![]).unwrap(),
        EstopLevel::NetworkKill
    ));
    assert!(
        build_engage_level(Some(EstopLevelArg::NetworkKill), vec!["x".into()], vec![]).is_err()
    );
    assert!(
        build_engage_level(Some(EstopLevelArg::NetworkKill), vec![], vec!["t".into()],).is_err()
    );
}

#[test]
fn build_engage_level_domain_block_validates_domains_and_rejects_tools() {
    assert!(build_engage_level(Some(EstopLevelArg::DomainBlock), vec![], vec![]).is_err());
    assert!(
        build_engage_level(Some(EstopLevelArg::DomainBlock), vec![], vec!["t".into()],).is_err()
    );
    let tool_error = build_engage_level(
        Some(EstopLevelArg::DomainBlock),
        vec!["example.com".into()],
        vec!["shell".into()],
    )
    .unwrap_err();
    assert!(tool_error.to_string().contains("--tool is not valid with --level domain-block"));

    assert!(matches!(
        build_engage_level(
            Some(EstopLevelArg::DomainBlock),
            vec!["example.com".into()],
            vec![],
        )
        .unwrap(),
        EstopLevel::DomainBlock(ref d) if d == &["example.com"]
    ));

    assert!(matches!(
        build_engage_level(
            Some(EstopLevelArg::DomainBlock),
            vec!["a.com".into(), "b.com".into()],
            vec![],
        )
        .unwrap(),
        EstopLevel::DomainBlock(ref d) if d.len() == 2
    ));
}

#[test]
fn build_engage_level_tool_freeze_validates_tools_and_rejects_domains() {
    assert!(build_engage_level(Some(EstopLevelArg::ToolFreeze), vec![], vec![]).is_err());
    assert!(
        build_engage_level(Some(EstopLevelArg::ToolFreeze), vec!["x".into()], vec![],).is_err()
    );
    let domain_error = build_engage_level(
        Some(EstopLevelArg::ToolFreeze),
        vec!["example.com".into()],
        vec!["shell".into()],
    )
    .unwrap_err();
    assert!(domain_error.to_string().contains("--domain is not valid with --level tool-freeze"));

    assert!(matches!(
        build_engage_level(
            Some(EstopLevelArg::ToolFreeze),
            vec![],
            vec!["my_tool".into()],
        )
        .unwrap(),
        EstopLevel::ToolFreeze(ref t) if t == &["my_tool"]
    ));

    assert!(matches!(
        build_engage_level(
            Some(EstopLevelArg::ToolFreeze),
            vec![],
            vec!["tool_a".into(), "tool_b".into()],
        )
        .unwrap(),
        EstopLevel::ToolFreeze(ref t) if t.len() == 2
    ));
}

// ============================================================
// build_resume_selector
// ============================================================

#[test]
fn build_resume_selector_allows_one_scope_only() {
    assert!(matches!(
        build_resume_selector(true, vec![], vec![]).unwrap(),
        ResumeSelector::Network
    ));
    assert!(matches!(
        build_resume_selector(false, vec!["example.com".into()], vec![]).unwrap(),
        ResumeSelector::Domains(domains) if domains == vec!["example.com".to_string()]
    ));
    assert!(matches!(
        build_resume_selector(false, vec![], vec![]).unwrap(),
        ResumeSelector::KillAll
    ));
    assert!(build_resume_selector(true, vec!["example.com".into()], vec![]).is_err());
}

#[test]
fn build_resume_selector_rejects_multiple_scopes() {
    assert!(build_resume_selector(true, vec!["x".into()], vec![]).is_err());
    assert!(build_resume_selector(true, vec![], vec!["x".into()]).is_err());
    assert!(build_resume_selector(false, vec!["x".into()], vec!["y".into()]).is_err());
    assert!(build_resume_selector(true, vec!["x".into()], vec!["y".into()]).is_err());
}

#[test]
fn build_resume_selector_selects_network() {
    assert!(matches!(
        build_resume_selector(true, vec![], vec![]).unwrap(),
        ResumeSelector::Network
    ));
}

#[test]
fn build_resume_selector_selects_domains() {
    assert!(matches!(
        build_resume_selector(false, vec!["example.com".into()], vec![]).unwrap(),
        ResumeSelector::Domains(ref d) if d == &["example.com"]
    ));
    assert!(matches!(
        build_resume_selector(false, vec!["a".into(), "b".into()], vec![]).unwrap(),
        ResumeSelector::Domains(ref d) if d.len() == 2
    ));
}

#[test]
fn build_resume_selector_selects_tools() {
    assert!(matches!(
        build_resume_selector(false, vec![], vec!["shell".into()]).unwrap(),
        ResumeSelector::Tools(ref t) if t == &["shell"]
    ));
}

#[test]
fn build_resume_selector_defaults_to_kill_all() {
    assert!(matches!(
        build_resume_selector(false, vec![], vec![]).unwrap(),
        ResumeSelector::KillAll
    ));
}

// ============================================================
// print_estop_status（smoke 测试，stdout 由测试框架捕获）
// ============================================================

#[test]
fn print_estop_status_with_default_state() {
    print_estop_status(&EstopState::default());
}

#[test]
fn print_estop_status_with_fail_closed_state() {
    print_estop_status(&EstopState::fail_closed());
}

#[test]
fn print_estop_status_with_all_active_state() {
    let state = EstopState {
        kill_all: true,
        network_kill: true,
        blocked_domains: vec!["evil.com".into()],
        frozen_tools: vec!["shell".into(), "file_ops".into()],
        updated_at: Some("2026-06-29T12:00:00Z".into()),
    };
    print_estop_status(&state);
}

#[test]
fn print_estop_status_with_empty_blocks_and_timestamp() {
    let state = EstopState {
        kill_all: true,
        network_kill: false,
        blocked_domains: vec![],
        frozen_tools: vec![],
        updated_at: Some("2026-06-29T12:00:00Z".into()),
    };
    print_estop_status(&state);
}

#[test]
fn print_estop_status_without_timestamp() {
    let state = EstopState {
        kill_all: true,
        network_kill: false,
        blocked_domains: vec!["malicious.com".into()],
        frozen_tools: vec!["rm".into()],
        updated_at: None,
    };
    print_estop_status(&state);
}

// ============================================================
// handle_estop_command
// ============================================================

#[test]
fn handle_estop_command_errors_when_disabled() {
    let config = Config::default();
    let err = handle_estop_command(&config, None, None, vec![], vec![]).unwrap_err();
    assert!(err.to_string().contains("Emergency stop is disabled"));
}

#[test]
fn handle_estop_command_status_reflects_initial_state() {
    let dir = TestDir::new();
    let config = make_enabled_config(&dir);
    let result =
        handle_estop_command(&config, Some(EstopSubcommands::Status), None, vec![], vec![]);
    assert!(result.is_ok());
}

#[test]
fn handle_estop_command_engage_kill_all() {
    let dir = TestDir::new();
    let config = make_enabled_config(&dir);
    assert!(handle_estop_command(&config, None, None, vec![], vec![]).is_ok());
}

#[test]
fn handle_estop_command_engage_network_kill() {
    let dir = TestDir::new();
    let config = make_enabled_config(&dir);
    assert!(
        handle_estop_command(&config, None, Some(EstopLevelArg::NetworkKill), vec![], vec![])
            .is_ok()
    );
}

#[test]
fn handle_estop_command_engage_domain_block() {
    let dir = TestDir::new();
    let config = make_enabled_config(&dir);
    assert!(
        handle_estop_command(
            &config,
            None,
            Some(EstopLevelArg::DomainBlock),
            vec!["evil.com".into()],
            vec![],
        )
        .is_ok()
    );
}

#[test]
fn handle_estop_command_engage_tool_freeze() {
    let dir = TestDir::new();
    let config = make_enabled_config(&dir);
    assert!(
        handle_estop_command(
            &config,
            None,
            Some(EstopLevelArg::ToolFreeze),
            vec![],
            vec!["rm".into()],
        )
        .is_ok()
    );
}

#[test]
fn handle_estop_command_engage_rejects_bad_combinations() {
    let dir = TestDir::new();
    let config = make_enabled_config(&dir);
    assert!(
        handle_estop_command(&config, None, Some(EstopLevelArg::DomainBlock), vec![], vec![])
            .is_err()
    );
}

#[test]
fn handle_estop_command_resume_kill_all() {
    let dir = TestDir::new();
    let config = make_enabled_config(&dir);

    handle_estop_command(&config, None, None, vec![], vec![]).unwrap();
    assert!(
        handle_estop_command(
            &config,
            Some(EstopSubcommands::Resume {
                network: false,
                domains: vec![],
                tools: vec![],
                otp: None,
            }),
            None,
            vec![],
            vec![],
        )
        .is_ok()
    );
}

#[test]
fn handle_estop_command_resume_network() {
    let dir = TestDir::new();
    let config = make_enabled_config(&dir);

    handle_estop_command(&config, None, Some(EstopLevelArg::NetworkKill), vec![], vec![]).unwrap();
    assert!(
        handle_estop_command(
            &config,
            Some(EstopSubcommands::Resume {
                network: true,
                domains: vec![],
                tools: vec![],
                otp: None,
            }),
            None,
            vec![],
            vec![],
        )
        .is_ok()
    );
}

#[test]
fn handle_estop_command_resume_domains() {
    let dir = TestDir::new();
    let config = make_enabled_config(&dir);

    handle_estop_command(
        &config,
        None,
        Some(EstopLevelArg::DomainBlock),
        vec!["evil.com".into()],
        vec![],
    )
    .unwrap();
    assert!(
        handle_estop_command(
            &config,
            Some(EstopSubcommands::Resume {
                network: false,
                domains: vec!["evil.com".into()],
                tools: vec![],
                otp: None,
            }),
            None,
            vec![],
            vec![],
        )
        .is_ok()
    );
}

#[test]
fn handle_estop_command_resume_tools() {
    let dir = TestDir::new();
    let config = make_enabled_config(&dir);

    handle_estop_command(
        &config,
        None,
        Some(EstopLevelArg::ToolFreeze),
        vec![],
        vec!["shell".into()],
    )
    .unwrap();
    assert!(
        handle_estop_command(
            &config,
            Some(EstopSubcommands::Resume {
                network: false,
                domains: vec![],
                tools: vec!["shell".into()],
                otp: None,
            }),
            None,
            vec![],
            vec![],
        )
        .is_ok()
    );
}

#[test]
fn handle_estop_command_rejects_required_otp_when_otp_is_disabled() {
    let dir = TestDir::new();
    let mut config = make_enabled_config(&dir);
    config.security.estop.require_otp_to_resume = true;
    config.security.otp.enabled = false;

    let error = handle_estop_command(
        &config,
        Some(EstopSubcommands::Resume {
            network: false,
            domains: vec![],
            tools: vec![],
            otp: Some("123456".to_string()),
        }),
        None,
        vec![],
        vec![],
    )
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("security.estop.require_otp_to_resume=true but security.otp.enabled=false")
    );
}

#[test]
fn handle_estop_command_rejects_invalid_supplied_otp_when_required() {
    let dir = TestDir::new();
    let mut config = make_enabled_config(&dir);
    config.security.estop.require_otp_to_resume = true;
    config.security.otp.enabled = true;

    let error = handle_estop_command(
        &config,
        Some(EstopSubcommands::Resume {
            network: false,
            domains: vec![],
            tools: vec![],
            otp: Some("000000".to_string()),
        }),
        None,
        vec![],
        vec![],
    )
    .unwrap_err();

    assert!(error.to_string().contains("Invalid OTP code; estop resume denied"));
}
