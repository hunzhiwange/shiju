use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::tempdir;
use config_security::{EstopConfig, OtpConfig};
use crate::SecretStore;
use crate::estop::{EstopLevel, EstopManager, ResumeSelector};
use crate::otp::OtpValidator;

fn estop_config(path: &Path) -> EstopConfig {
    EstopConfig {
        enabled: true,
        state_file: path.display().to_string(),
        require_otp_to_resume: false,
    }
}

// 测试多个应急停止级别（域名封锁、工具冻结、网络断开）的组合使用及恢复功能
#[test]
fn estop_levels_compose_and_resume() {
    let dir = tempdir().unwrap();
    let state_path = dir.path().join("estop-state.json");
    let cfg = estop_config(&state_path);
    let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();

    manager.engage(EstopLevel::DomainBlock(vec!["*.chase.com".into()])).unwrap();
    manager.engage(EstopLevel::ToolFreeze(vec!["shell".into()])).unwrap();
    manager.engage(EstopLevel::NetworkKill).unwrap();
    assert!(manager.status().network_kill);
    assert_eq!(manager.status().blocked_domains, vec!["*.chase.com"]);
    assert_eq!(manager.status().frozen_tools, vec!["shell"]);

    manager.resume(ResumeSelector::Domains(vec!["*.chase.com".into()]), None, None).unwrap();
    assert!(manager.status().blocked_domains.is_empty());
    assert!(manager.status().network_kill);

    manager.resume(ResumeSelector::Tools(vec!["shell".into()]), None, None).unwrap();
    assert!(manager.status().frozen_tools.is_empty());
}

// 测试应急停止状态在管理器重新加载后是否能够正确保持
#[test]
fn estop_state_survives_reload() {
    let dir = tempdir().unwrap();
    let state_path = dir.path().join("estop-state.json");
    let cfg = estop_config(&state_path);

    {
        let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();
        manager.engage(EstopLevel::KillAll).unwrap();
        manager.engage(EstopLevel::DomainBlock(vec!["*.paypal.com".into()])).unwrap();
    }

    let reloaded = EstopManager::load(&cfg, dir.path()).unwrap();
    let state = reloaded.status();
    assert!(state.kill_all);
    assert_eq!(state.blocked_domains, vec!["*.paypal.com"]);
}

// 测试状态文件损坏时，系统默认采取安全的故障关闭策略（启用 KillAll）
#[test]
fn corrupted_state_defaults_to_fail_closed_kill_all() {
    let dir = tempdir().unwrap();
    let state_path = dir.path().join("estop-state.json");
    fs::write(&state_path, "{not-valid-json").unwrap();
    let cfg = estop_config(&state_path);
    let manager = EstopManager::load(&cfg, dir.path()).unwrap();
    assert!(manager.status().kill_all);
}

// 测试启用 OTP 验证时，恢复操作必须提供有效的 OTP 验证码
#[test]
fn resume_requires_valid_otp_when_enabled() {
    let dir = tempdir().unwrap();
    let state_path = dir.path().join("estop-state.json");
    let mut cfg = estop_config(&state_path);
    cfg.require_otp_to_resume = true;

    let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();
    manager.engage(EstopLevel::KillAll).unwrap();

    let err =
        manager.resume(ResumeSelector::KillAll, None, None).expect_err("resume should require OTP");
    assert!(err.to_string().contains("OTP code is required"));
}

// 测试提供有效的 OTP 验证码可以成功恢复被应急停止的系统
#[test]
fn resume_accepts_valid_otp_code() {
    let dir = tempdir().unwrap();
    let state_path = dir.path().join("estop-state.json");
    let mut cfg = estop_config(&state_path);
    cfg.require_otp_to_resume = true;

    let otp_cfg = OtpConfig { enabled: true, ..OtpConfig::default() };
    let store = SecretStore::new(dir.path(), true);
    let (validator, _) = OtpValidator::from_config(&otp_cfg, dir.path(), &store).unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let code = validator.code_for_timestamp(now);

    let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();
    manager.engage(EstopLevel::KillAll).unwrap();
    manager.resume(ResumeSelector::KillAll, Some(&code), Some(&validator)).unwrap();
    assert!(!manager.status().kill_all);
}
