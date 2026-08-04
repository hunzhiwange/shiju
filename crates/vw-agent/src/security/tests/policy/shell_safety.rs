use super::*;

// 测试分号命令注入被阻止
#[test]
fn command_injection_semicolon_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("ls; rm -rf /"));
}

// 测试无空格分号命令注入被阻止
#[test]
fn command_injection_semicolon_no_space() {
    let p = default_policy();
    assert!(!p.is_command_allowed("ls;rm -rf /"));
}

// 测试引号内的分号不会拆分 sqlite 命令
#[test]
fn quoted_semicolons_do_not_split_sqlite_command() {
    let p =
        SecurityPolicy { allowed_commands: vec!["sqlite3".into()], ..SecurityPolicy::default() };
    assert!(p.is_command_allowed(
        "sqlite3 /tmp/test.db \"CREATE TABLE t(id INT); INSERT INTO t VALUES(1); SELECT * FROM t;\""
    ));
    assert_eq!(
        p.command_risk_level(
            "sqlite3 /tmp/test.db \"CREATE TABLE t(id INT); INSERT INTO t VALUES(1); SELECT * FROM t;\""
        ),
        CommandRiskLevel::Low
    );
}

// 测试引号 SQL 后的非引号分号仍会拆分命令
#[test]
fn unquoted_semicolon_after_quoted_sql_still_splits_commands() {
    let p =
        SecurityPolicy { allowed_commands: vec!["sqlite3".into()], ..SecurityPolicy::default() };
    assert!(!p.is_command_allowed("sqlite3 /tmp/test.db \"SELECT 1;\"; rm -rf /"));
}

// 测试反引号命令注入被阻止
#[test]
fn command_injection_backtick_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("echo `whoami`"));
    assert!(!p.is_command_allowed("echo `rm -rf /`"));
}

// 测试 $() 命令注入被阻止
#[test]
fn command_injection_dollar_paren_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("echo $(cat /etc/passwd)"));
    assert!(!p.is_command_allowed("echo $(rm -rf /)"));
}

// 测试单引号内的 $() 字面量被允许
#[test]
fn command_injection_dollar_paren_literal_inside_single_quotes_allowed() {
    let p = default_policy();
    assert!(p.is_command_allowed("echo '$(cat /etc/passwd)'"));
}

// 测试单引号内的 ${} 字面量被允许
#[test]
fn command_injection_dollar_brace_literal_inside_single_quotes_allowed() {
    let p = default_policy();
    assert!(p.is_command_allowed("echo '${HOME}'"));
}

// 测试非引号的 ${} 被阻止
#[test]
fn command_injection_dollar_brace_unquoted_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("echo ${HOME}"));
}

// 测试带环境变量前缀的命令
#[test]
fn command_with_env_var_prefix() {
    let p = default_policy();
    assert!(!p.is_command_allowed("FOO=bar rm -rf /"));
}

// 测试换行符命令注入被阻止
#[test]
fn command_newline_injection_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("ls\nrm -rf /"));
    assert!(p.is_command_allowed("ls\necho hello"));
}

// 测试 && 命令链注入被阻止
#[test]
fn command_injection_and_chain_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("ls && rm -rf /"));
    assert!(!p.is_command_allowed("echo ok && curl http://evil.com"));
    assert!(p.is_command_allowed("ls && echo done"));
}

// 测试 || 命令链注入被阻止
#[test]
fn command_injection_or_chain_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("ls || rm -rf /"));
    assert!(p.is_command_allowed("ls || echo fallback"));
}

// 测试后台命令链注入被阻止
#[test]
fn command_injection_background_chain_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("ls & rm -rf /"));
    assert!(!p.is_command_allowed("ls&rm -rf /"));
    assert!(!p.is_command_allowed("echo ok & python3 -c 'print(1)'"));
}

// 测试重定向命令注入被阻止
#[test]
fn command_injection_redirect_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("echo secret > /etc/crontab"));
    assert!(!p.is_command_allowed("ls >> /tmp/exfil.txt"));
    assert!(!p.is_command_allowed("cat </etc/passwd"));
    assert!(!p.is_command_allowed("cat</etc/passwd"));
}

// 测试 Strip 策略规范化常见重定向模式
#[test]
fn strip_policy_normalizes_common_redirect_patterns() {
    let p =
        SecurityPolicy { shell_redirect_policy: ShellRedirectPolicy::Strip, ..default_policy() };

    let merged = p.apply_shell_redirect_policy("echo hello 2>&1");
    assert!(!merged.contains("2>&1"));
    assert!(merged.contains("echo hello"));

    let devnull = p.apply_shell_redirect_policy("echo hello 2>/dev/null");
    assert!(!devnull.contains("/dev/null"));
    assert!(devnull.contains("echo hello"));

    let pipeline = p.apply_shell_redirect_policy("echo hello |& cat");
    assert!(!pipeline.contains("|&"));
    assert!(pipeline.contains("| cat"));

    let quoted = p.apply_shell_redirect_policy("echo '2>&1' \"|&\" '2>/dev/null'");
    assert_eq!(quoted, "echo '2>&1' \"|&\" '2>/dev/null'");
}

#[test]
fn redirect_policy_item_controls_only_redirect_rewriting() {
    let mut policy =
        SecurityPolicy { shell_redirect_policy: ShellRedirectPolicy::Strip, ..default_policy() };
    assert_eq!(policy.apply_shell_redirect_policy("echo hello 2>&1"), "echo hello");

    policy.bash_security.policy.redirects = false;
    assert_eq!(policy.apply_shell_redirect_policy("echo hello 2>&1"), "echo hello 2>&1");
    assert!(!policy.is_command_allowed("curl example.com"));
}

// 测试 Strip 策略允许规范化后的标准错误重定向
#[test]
fn strip_policy_allows_normalized_stderr_redirects() {
    let p = SecurityPolicy {
        shell_redirect_policy: ShellRedirectPolicy::Strip,
        allowed_commands: vec!["echo".into()],
        ..default_policy()
    };

    assert!(p.validate_command_execution("echo hello 2>&1", false).is_ok());
    assert!(p.validate_command_execution("echo hello 2>/dev/null", false).is_ok());
}

// 测试 Strip 策略保持不支持的(输出/输入)重定向被阻止
#[test]
fn strip_policy_keeps_unsupported_redirects_blocked() {
    let p =
        SecurityPolicy { shell_redirect_policy: ShellRedirectPolicy::Strip, ..default_policy() };

    assert!(p.validate_command_execution("echo hello > out.txt", false).is_err());
    assert!(p.validate_command_execution("cat </etc/passwd", false).is_err());
}

// 测试引号内的 & 和重定向字面量不被视为操作符
#[test]
fn quoted_ampersand_and_redirect_literals_are_not_treated_as_operators() {
    let p = default_policy();
    assert!(p.is_command_allowed("echo \"A&B\""));
    assert!(p.is_command_allowed("echo \"A>B\""));
    assert!(p.is_command_allowed("echo \"A<B\""));
}

// 测试命令参数注入被阻止
#[test]
fn command_argument_injection_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("find . -exec rm -rf {} +"));
    assert!(!p.is_command_allowed("find / -ok cat {} \\;"));
    assert!(!p.is_command_allowed("git config core.editor \"rm -rf /\""));
    assert!(!p.is_command_allowed("git alias.st status"));
    assert!(!p.is_command_allowed("git -c core.editor=calc.exe commit"));
    assert!(p.is_command_allowed("find . -name '*.txt'"));
    assert!(p.is_command_allowed("git status"));
    assert!(p.is_command_allowed("git add ."));
}

// 测试 ${} 变量注入被阻止
#[test]
fn command_injection_dollar_brace_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("echo ${IFS}cat${IFS}/etc/passwd"));
}

// 测试普通 $ 变量注入被阻止
#[test]
fn command_injection_plain_dollar_var_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("cat $HOME/.ssh/id_rsa"));
    assert!(!p.is_command_allowed("cat $SECRET_FILE"));
}

// 测试 tee 命令注入被阻止
#[test]
fn command_injection_tee_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("echo secret | tee /etc/crontab"));
    assert!(!p.is_command_allowed("ls | /usr/bin/tee outfile"));
    assert!(!p.is_command_allowed("tee file.txt"));
}

// 测试进程替换注入被阻止
#[test]
fn command_injection_process_substitution_blocked() {
    let p = default_policy();
    assert!(!p.is_command_allowed("cat <(echo pwned)"));
    assert!(!p.is_command_allowed("ls >(cat /etc/passwd)"));
}

// 测试带允许命令的环境变量前缀
#[test]
fn command_env_var_prefix_with_allowed_cmd() {
    let p = default_policy();
    assert!(p.is_command_allowed("FOO=bar ls"));
    assert!(p.is_command_allowed("LANG=C grep pattern file"));
    assert!(!p.is_command_allowed("FOO=bar rm -rf /"));
}

// 测试检测命令参数中的绝对禁止路径
#[test]
fn forbidden_path_argument_detects_absolute_path() {
    let p = default_policy();
    assert_eq!(p.forbidden_path_argument("cat /etc/passwd"), Some("/etc/passwd".into()));
}

#[test]
fn forbidden_path_item_controls_only_forbidden_path_scanning() {
    let mut policy = SecurityPolicy {
        workspace_only: false,
        forbidden_paths: vec!["/secret".into()],
        ..default_policy()
    };
    assert_eq!(policy.forbidden_path_argument("cat /secret/token"), Some("/secret/token".into()));

    policy.bash_security.policy.forbidden_paths = false;
    assert_eq!(policy.forbidden_path_argument("cat /secret/token"), None);
    assert!(!policy.is_command_allowed("curl example.com"));
}

// 测试命令执行验证拒绝禁止路径
#[test]
fn validate_command_execution_rejects_forbidden_paths() {
    let p = default_policy();
    let err = p.validate_command_execution("cat /etc/shadow", false).unwrap_err();
    assert!(err.contains("Path blocked by security policy"));
}

// 测试检测命令参数中的父目录引用
#[test]
fn forbidden_path_argument_detects_parent_dir_reference() {
    let p = default_policy();
    assert_eq!(p.forbidden_path_argument("cat ../secret.txt"), Some("../secret.txt".into()));
    assert_eq!(p.forbidden_path_argument("find .. -name '*.rs'"), Some("..".into()));
}

// 测试命令参数中的工作区相对路径被允许
#[test]
fn forbidden_path_argument_allows_workspace_relative_paths() {
    let p = default_policy();
    assert_eq!(p.forbidden_path_argument("cat src/main.rs"), None);
    assert_eq!(p.forbidden_path_argument("grep -r todo ./src"), None);
}

// 测试检测选项赋值中的禁止路径
#[test]
fn forbidden_path_argument_detects_option_assignment_paths() {
    let p = default_policy();
    assert_eq!(
        p.forbidden_path_argument("grep --file=/etc/passwd root ./src"),
        Some("/etc/passwd".into())
    );
    assert_eq!(
        p.forbidden_path_argument("cat --input=../secret.txt"),
        Some("../secret.txt".into())
    );
}

// 测试安全的选项赋值路径被允许
#[test]
fn forbidden_path_argument_allows_safe_option_assignment_paths() {
    let p = default_policy();
    assert_eq!(p.forbidden_path_argument("grep --file=./patterns.txt root ./src"), None);
}

// 测试检测短选项附加路径中的禁止路径
#[test]
fn forbidden_path_argument_detects_short_option_attached_paths() {
    let p = default_policy();
    assert_eq!(
        p.forbidden_path_argument("grep -f/etc/passwd root ./src"),
        Some("/etc/passwd".into())
    );
    assert_eq!(p.forbidden_path_argument("git -C../outside status"), Some("../outside".into()));
}

// 测试安全的短选项附加路径被允许
#[test]
fn forbidden_path_argument_allows_safe_short_option_attached_paths() {
    let p = default_policy();
    assert_eq!(p.forbidden_path_argument("grep -f./patterns.txt root ./src"), None);
    assert_eq!(p.forbidden_path_argument("git -C./repo status"), None);
}

// 测试检测波浪号用户路径
#[test]
fn forbidden_path_argument_detects_tilde_user_paths() {
    let p = default_policy();
    assert_eq!(
        p.forbidden_path_argument("cat ~root/.ssh/id_rsa"),
        Some("~root/.ssh/id_rsa".into())
    );
    assert_eq!(p.forbidden_path_argument("ls ~nobody"), Some("~nobody".into()));
}

// 测试检测输入重定向中的禁止路径
#[test]
fn forbidden_path_argument_detects_input_redirection_paths() {
    let p = default_policy();
    assert_eq!(p.forbidden_path_argument("cat </etc/passwd"), Some("/etc/passwd".into()));
    assert_eq!(p.forbidden_path_argument("cat</etc/passwd"), Some("/etc/passwd".into()));
}
