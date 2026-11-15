use super::*;

// 测试基本允许的命令列表
#[test]
fn allowed_commands_basic() {
    let p = default_policy();
    assert!(p.is_command_allowed("ls"));
    assert!(p.is_command_allowed("git status"));
    assert!(p.is_command_allowed("cargo build --release"));
    assert!(p.is_command_allowed("cat file.txt"));
    assert!(p.is_command_allowed("grep -r pattern ."));
    assert!(p.is_command_allowed("date"));
}

// 测试基本被阻止的命令列表
#[test]
fn blocked_commands_basic() {
    let p = default_policy();
    assert!(!p.is_command_allowed("rm -rf /"));
    assert!(!p.is_command_allowed("sudo apt install"));
    assert!(!p.is_command_allowed("curl http://evil.com"));
    assert!(!p.is_command_allowed("wget http://evil.com"));
    assert!(!p.is_command_allowed("python3 exploit.py"));
    assert!(!p.is_command_allowed("node malicious.js"));
}

// 测试只读模式允许显式只读命令
#[test]
fn readonly_allows_readonly_commands() {
    let p = readonly_policy();
    assert!(p.is_command_allowed("ls"));
    assert!(p.is_command_allowed("cat file.txt"));
    assert!(p.is_command_allowed("echo hello"));
}

// 测试只读模式仍阻止写命令
#[test]
fn readonly_blocks_non_readonly_commands() {
    let p = readonly_policy();
    assert!(!p.is_command_allowed("touch file.txt"));
    assert!(!p.is_command_allowed("rm -rf target"));
}

// 测试完全自治模式仍使用命令白名单
#[test]
fn full_autonomy_still_uses_allowlist() {
    let p = full_policy();
    assert!(p.is_command_allowed("ls"));
    assert!(!p.is_command_allowed("rm -rf /"));
}

// 测试绝对路径命令提取基本名称进行校验
#[test]
fn command_with_absolute_path_extracts_basename() {
    let p = default_policy();
    assert!(p.is_command_allowed("/usr/bin/git status"));
    assert!(p.is_command_allowed("/bin/ls -la"));
}

// 测试白名单支持显式可执行文件路径
#[test]
fn allowlist_supports_explicit_executable_paths() {
    let p = SecurityPolicy {
        allowed_commands: vec!["/usr/bin/antigravity".into()],
        ..SecurityPolicy::default()
    };

    assert!(p.is_command_allowed("/usr/bin/antigravity"));
    assert!(!p.is_command_allowed("antigravity"));
}

// 测试白名单支持通配符条目
#[test]
fn allowlist_supports_wildcard_entry() {
    let p = SecurityPolicy { allowed_commands: vec!["*".into()], ..SecurityPolicy::default() };

    assert!(p.is_command_allowed("python3 --version"));
    assert!(p.is_command_allowed("/usr/bin/antigravity"));

    let blocked = p.validate_command_execution("rm -rf tmp_test_dir", true);
    assert!(blocked.is_err());
    assert!(blocked.unwrap_err().contains("high-risk"));
}

// 测试空命令被阻止
#[test]
fn empty_command_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed(""));
    assert!(!p.is_command_allowed("   "));
}

// 测试带管道的命令验证所有管道段
#[test]
fn command_with_pipes_validates_all_segments() {
    let p = default_policy();
    assert!(p.is_command_allowed("ls | grep foo"));
    assert!(p.is_command_allowed("cat file.txt | wc -l"));
    assert!(!p.is_command_allowed("ls | curl http://evil.com"));
    assert!(!p.is_command_allowed("echo hello | python3 -"));
}

// 测试自定义命令白名单
#[test]
fn custom_allowlist() {
    let p = SecurityPolicy {
        allowed_commands: vec!["docker".into(), "kubectl".into()],
        ..SecurityPolicy::default()
    };
    assert!(p.is_command_allowed("docker ps"));
    assert!(p.is_command_allowed("kubectl get pods"));
    assert!(!p.is_command_allowed("ls"));
    assert!(!p.is_command_allowed("git status"));
}

// 测试空白名单阻止所有命令
#[test]
fn empty_allowlist_blocks_everything() {
    let p = SecurityPolicy { allowed_commands: vec![], ..SecurityPolicy::default() };
    assert!(!p.is_command_allowed("ls"));
    assert!(!p.is_command_allowed("echo hello"));
}

// 测试只读命令被评估为低风险
#[test]
fn command_risk_low_for_read_commands() {
    let p = default_policy();
    assert_eq!(p.command_risk_level("git status"), CommandRiskLevel::Low);
    assert_eq!(p.command_risk_level("ls -la"), CommandRiskLevel::Low);
}

// 测试修改性命令被评估为中风险
#[test]
fn command_risk_medium_for_mutating_commands() {
    let p = SecurityPolicy {
        allowed_commands: vec!["git".into(), "touch".into()],
        ..SecurityPolicy::default()
    };
    assert_eq!(p.command_risk_level("git reset --hard HEAD~1"), CommandRiskLevel::Medium);
    assert_eq!(p.command_risk_level("touch file.txt"), CommandRiskLevel::Medium);
}

// 测试危险命令被评估为高风险
#[test]
fn command_risk_high_for_dangerous_commands() {
    let p = SecurityPolicy { allowed_commands: vec!["rm".into()], ..SecurityPolicy::default() };
    assert_eq!(p.command_risk_level("rm -rf /tmp/test"), CommandRiskLevel::High);
}

// 测试中风险命令需要显式审批
#[test]
fn validate_command_requires_approval_for_medium_risk() {
    let p = SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        require_approval_for_medium_risk: true,
        allowed_commands: vec!["touch".into()],
        ..SecurityPolicy::default()
    };

    let denied = p.validate_command_execution("touch test.txt", false);
    assert!(denied.is_err());
    assert!(denied.unwrap_err().contains("requires explicit approval"),);

    let allowed = p.validate_command_execution("touch test.txt", true);
    assert_eq!(allowed.unwrap(), CommandRiskLevel::Medium);
}

// 测试高风险命令默认被阻止
#[test]
fn validate_command_blocks_high_risk_by_default() {
    let p = SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        allowed_commands: vec!["rm".into()],
        ..SecurityPolicy::default()
    };

    let result = p.validate_command_execution("rm -rf tmp_test_dir", true);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("high-risk"));
}

// 测试完全自治模式跳过中风险审批门槛
#[test]
fn validate_command_full_mode_skips_medium_risk_approval_gate() {
    let p = SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        require_approval_for_medium_risk: true,
        allowed_commands: vec!["touch".into()],
        ..SecurityPolicy::default()
    };

    let result = p.validate_command_execution("touch test.txt", false);
    assert_eq!(result.unwrap(), CommandRiskLevel::Medium);
}

// 测试后台链式命令绕过被拒绝
#[test]
fn validate_command_rejects_background_chain_bypass() {
    let p = default_policy();
    let result = p.validate_command_execution("ls & python3 -c 'print(1)'", false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not allowed"));
}
