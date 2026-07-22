//! 紧急停止（EStop）模块测试
//!
//! 本模块包含紧急停止系统的集成测试，验证以下核心功能：
//! - 多级别紧急停止的组合触发与恢复
//! - 状态持久化与重载后的状态保持
//! - 损坏状态的故障安全默认行为
//! - OTP（一次性密码）保护下的恢复操作
//!
//! ## 测试覆盖范围
//!
//! - 域名阻止（DomainBlock）：阻止特定域名的网络访问
//! - 工具冻结（ToolFreeze）：禁用特定工具的执行
//! - 网络断开（NetworkKill）：完全断开网络连接
//! - 全面停止（KillAll）：系统最高级别的紧急停止
//!
//! ## 安全原则
//!
//! 所有测试遵循"故障安全"（fail-safe）原则：
//! - 状态文件损坏时，系统默认进入最高保护级别（KillAll）
//! - OTP 保护确保只有授权用户才能恢复系统

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::config::OtpConfig;
    use crate::app::agent::security::SecretStore;
    use crate::app::agent::security::otp::OtpValidator;
    use tempfile::tempdir;

    /// 创建测试用的紧急停止配置
    ///
    /// # 参数
    ///
    /// - `path`: 状态文件的存储路径
    ///
    /// # 返回值
    ///
    /// 返回一个启用了紧急停止功能的配置实例，状态文件将写入指定路径。
    /// 该配置禁用了恢复时的 OTP 验证，以简化大多数测试场景。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let dir = tempdir().unwrap();
    /// let state_path = dir.path().join("estop-state.json");
    /// let cfg = estop_config(&state_path);
    /// ```
    fn estop_config(path: &Path) -> EstopConfig {
        EstopConfig {
            enabled: true,
            state_file: path.display().to_string(),
            require_otp_to_resume: false,
        }
    }

    #[test]
    fn default_state_is_not_engaged_and_fail_closed_is_engaged() {
        assert!(!EstopState::default().is_engaged());

        let state = EstopState::fail_closed();
        assert!(state.is_engaged());
        assert!(state.kill_all);
        assert!(state.updated_at.is_some());
    }

    #[test]
    fn load_missing_state_uses_default_and_resolves_relative_path() {
        let dir = tempdir().unwrap();
        let cfg = EstopConfig {
            enabled: true,
            state_file: "state/estop.json".into(),
            require_otp_to_resume: false,
        };

        let manager = EstopManager::load(&cfg, dir.path()).unwrap();

        assert_eq!(manager.state_path(), dir.path().join("state/estop.json"));
        assert!(!manager.status().is_engaged());
    }

    /// 测试多级别紧急停止的组合触发与选择性恢复
    ///
    /// # 测试场景
    ///
    /// 1. 按顺序触发三个不同级别的紧急停止：
    ///    - 域名阻止：阻止 *.chase.com
    ///    - 工具冻结：冻结 shell 工具
    ///    - 网络断开：完全断开网络
    ///
    /// 2. 验证所有触发的状态都正确记录：
    ///    - network_kill 应为 true
    ///    - blocked_domains 应包含 "*.chase.com"
    ///    - frozen_tools 应包含 "shell"
    ///
    /// 3. 选择性恢复域名阻止，验证：
    ///    - blocked_domains 应为空
    ///    - 其他状态（network_kill、frozen_tools）应保持不变
    ///
    /// 4. 选择性恢复工具冻结，验证：
    ///    - frozen_tools 应为空
    ///    - network_kill 仍保持激活状态
    ///
    /// # 设计原理
    ///
    /// 此测试验证了紧急停止系统的组合性和隔离性：
    /// - 多个停止级别可以同时激活
    /// - 恢复某个级别不应影响其他已激活的级别
    #[test]
    fn estop_levels_compose_and_resume() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("estop-state.json");
        let cfg = estop_config(&state_path);
        let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();

        // 按顺序触发多个紧急停止级别
        manager.engage(EstopLevel::DomainBlock(vec!["*.chase.com".into()])).unwrap();
        manager.engage(EstopLevel::ToolFreeze(vec!["shell".into()])).unwrap();
        manager.engage(EstopLevel::NetworkKill).unwrap();

        // 验证所有状态都正确记录
        assert!(manager.status().network_kill);
        assert_eq!(manager.status().blocked_domains, vec!["*.chase.com"]);
        assert_eq!(manager.status().frozen_tools, vec!["shell"]);

        // 选择性恢复域名阻止
        manager.resume(ResumeSelector::Domains(vec!["*.chase.com".into()]), None, None).unwrap();
        assert!(manager.status().blocked_domains.is_empty());

        // 验证其他状态未受影响
        assert!(manager.status().network_kill);

        // 选择性恢复工具冻结
        manager.resume(ResumeSelector::Tools(vec!["shell".into()]), None, None).unwrap();
        assert!(manager.status().frozen_tools.is_empty());
    }

    #[test]
    fn engage_normalizes_dedupes_and_resumes_network() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("estop-state.json");
        let cfg = estop_config(&state_path);
        let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();

        manager
            .engage(EstopLevel::DomainBlock(vec![
                " B.EXAMPLE.COM ".into(),
                "a.example.com".into(),
                "b.example.com".into(),
            ]))
            .unwrap();
        manager
            .engage(EstopLevel::ToolFreeze(vec![
                "Shell".into(),
                "apply_patch".into(),
                "shell".into(),
            ]))
            .unwrap();
        manager.engage(EstopLevel::NetworkKill).unwrap();

        let state = manager.status();
        assert_eq!(
            state.blocked_domains,
            vec!["a.example.com".to_string(), "b.example.com".to_string()]
        );
        assert_eq!(state.frozen_tools, vec!["apply_patch".to_string(), "shell".to_string()]);
        assert!(state.network_kill);

        manager.resume(ResumeSelector::Network, None, None).unwrap();
        assert!(!manager.status().network_kill);
    }

    #[test]
    fn engage_rejects_invalid_domains_and_tools() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("estop-state.json");
        let cfg = estop_config(&state_path);
        let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();

        let domain_err = manager
            .engage(EstopLevel::DomainBlock(vec!["bad domain.com".into()]))
            .expect_err("invalid domain should be rejected");
        assert!(domain_err.to_string().contains("invalid characters"));

        let tool_err = manager
            .engage(EstopLevel::ToolFreeze(vec!["shell;rm".into()]))
            .expect_err("invalid tool should be rejected");
        assert!(tool_err.to_string().contains("invalid characters"));
    }

    /// 测试紧急停止状态在重载后的持久化
    ///
    /// # 测试场景
    ///
    /// 1. 创建并配置紧急停止管理器
    /// 2. 触发 KillAll 和 DomainBlock 两个级别
    /// 3. 显式丢弃管理器实例（模拟进程重启）
    /// 4. 重新加载管理器
    /// 5. 验证所有状态都正确恢复
    ///
    /// # 设计原理
    ///
    /// 紧急停止状态必须在进程重启后依然保持，确保系统在
    /// 意外崩溃或重启后仍处于安全状态。状态文件作为唯一
    /// 的事实来源（source of truth），管理器加载时从中恢复。
    #[test]
    fn estop_state_survives_reload() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("estop-state.json");
        let cfg = estop_config(&state_path);

        // 在内部作用域中创建管理器并触发紧急停止
        // 作用域结束时，管理器被销毁，但状态已持久化
        {
            let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();
            manager.engage(EstopLevel::KillAll).unwrap();
            manager.engage(EstopLevel::DomainBlock(vec!["*.paypal.com".into()])).unwrap();
        }

        // 重新加载管理器，验证状态持久化
        let reloaded = EstopManager::load(&cfg, dir.path()).unwrap();
        let state = reloaded.status();
        assert!(state.kill_all);
        assert_eq!(state.blocked_domains, vec!["*.paypal.com"]);
    }

    /// 测试损坏状态文件的故障安全行为
    ///
    /// # 测试场景
    ///
    /// 1. 创建一个包含无效 JSON 的状态文件
    /// 2. 尝试加载管理器
    /// 3. 验证系统默认进入 KillAll 状态
    ///
    /// # 设计原理
    ///
    /// 遵循"故障安全"（fail-safe）原则：
    /// - 当状态文件损坏或无法解析时，系统不应继续运行
    /// - 最安全的默认行为是进入最高保护级别（KillAll）
    /// - 这防止了攻击者通过篡改状态文件来绕过安全机制
    ///
    /// # 安全考虑
    ///
    /// 这种设计确保即使存储介质故障或遭受篡改，
    /// 系统也不会意外进入不安全状态。
    #[test]
    fn corrupted_state_defaults_to_fail_closed_kill_all() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("estop-state.json");

        // 写入无效的 JSON 内容，模拟损坏的状态文件
        fs::write(&state_path, "{not-valid-json").unwrap();

        let cfg = estop_config(&state_path);
        let manager = EstopManager::load(&cfg, dir.path()).unwrap();

        // 验证系统默认进入最高保护级别
        assert!(manager.status().kill_all);
    }

    #[test]
    fn readable_state_is_normalized_on_load() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("estop-state.json");
        fs::write(
            &state_path,
            r#"{
                "kill_all": false,
                "network_kill": false,
                "blocked_domains": ["z.example.com", "a.example.com", "a.example.com", ""],
                "frozen_tools": ["shell", "apply_patch", "shell", " "]
            }"#,
        )
        .unwrap();

        let cfg = estop_config(&state_path);
        let manager = EstopManager::load(&cfg, dir.path()).unwrap();

        assert_eq!(
            manager.status().blocked_domains,
            vec!["a.example.com".to_string(), "z.example.com".to_string()]
        );
        assert_eq!(
            manager.status().frozen_tools,
            vec!["apply_patch".to_string(), "shell".to_string()]
        );
    }

    /// 测试启用 OTP 时恢复操作需要有效的 OTP 验证码
    ///
    /// # 测试场景
    ///
    /// 1. 配置启用了 OTP 验证的紧急停止管理器
    /// 2. 触发 KillAll 紧急停止
    /// 3. 尝试在未提供 OTP 验证码的情况下恢复
    /// 4. 验证恢复操作被拒绝，并返回正确的错误信息
    ///
    /// # 设计原理
    ///
    /// OTP（一次性密码）保护为恢复操作增加了第二层认证：
    /// - 防止未授权用户通过 API 或其他途径恢复系统
    /// - 确保只有持有验证器的人员才能解除紧急状态
    /// - 即使攻击者获得了系统访问权限，也无法绕过此保护
    #[test]
    fn resume_requires_valid_otp_when_enabled() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("estop-state.json");
        let mut cfg = estop_config(&state_path);

        // 启用 OTP 验证要求
        cfg.require_otp_to_resume = true;

        let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();
        manager.engage(EstopLevel::KillAll).unwrap();

        // 尝试在未提供 OTP 的情况下恢复，应返回错误
        let err = manager
            .resume(ResumeSelector::KillAll, None, None)
            .expect_err("resume should require OTP");

        // 验证错误信息包含 OTP 要求提示
        assert!(err.to_string().contains("OTP code is required"));
    }

    #[test]
    fn resume_requires_validator_when_otp_code_is_present() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("estop-state.json");
        let mut cfg = estop_config(&state_path);
        cfg.require_otp_to_resume = true;
        let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();

        let err = manager
            .resume(ResumeSelector::KillAll, Some("123456"), None)
            .expect_err("validator should be required");

        assert!(err.to_string().contains("OTP validator is required"));
    }

    #[test]
    fn resume_rejects_invalid_tool_selector_before_persisting() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("estop-state.json");
        let cfg = estop_config(&state_path);
        let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();
        manager.engage(EstopLevel::ToolFreeze(vec!["shell".into()])).unwrap();

        let err = manager
            .resume(ResumeSelector::Tools(vec!["bad/tool".into()]), None, None)
            .expect_err("invalid tool selector should fail");

        assert!(err.to_string().contains("invalid characters"));
        assert_eq!(manager.status().frozen_tools, vec!["shell".to_string()]);
    }

    /// 测试使用有效的 OTP 验证码成功恢复系统
    ///
    /// # 测试场景
    ///
    /// 1. 配置启用了 OTP 验证的紧急停止管理器
    /// 2. 创建并初始化 OTP 验证器
    /// 3. 基于当前时间戳生成有效的 OTP 验证码
    /// 4. 触发 KillAll 紧急停止
    /// 5. 使用有效的 OTP 验证码恢复系统
    /// 6. 验证系统成功恢复
    ///
    /// # 设计原理
    ///
    /// 此测试验证了完整的 OTP 保护恢复流程：
    /// - OTP 验证器基于时间生成验证码（TOTP 算法）
    /// - 验证码必须在有效时间窗口内
    /// - 提供正确的验证码和验证器实例后，恢复操作应成功
    ///
    /// # 安全考虑
    ///
    /// 时间戳的使用确保了：
    /// - 验证码有时效性，防止重放攻击
    /// - 验证码基于共享密钥生成，只有授权设备才能生成
    #[test]
    fn resume_accepts_valid_otp_code() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("estop-state.json");
        let mut cfg = estop_config(&state_path);

        // 启用 OTP 验证要求
        cfg.require_otp_to_resume = true;

        // 初始化 OTP 验证器
        let otp_cfg = OtpConfig { enabled: true, ..OtpConfig::default() };
        let store = SecretStore::new(dir.path(), true);
        let (validator, _) = OtpValidator::from_config(&otp_cfg, dir.path(), &store).unwrap();

        // 基于当前时间戳生成有效的 OTP 验证码
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let code = validator.code_for_timestamp(now);

        let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();
        manager.engage(EstopLevel::KillAll).unwrap();

        // 使用有效的 OTP 验证码恢复系统
        manager.resume(ResumeSelector::KillAll, Some(&code), Some(&validator)).unwrap();

        // 验证系统已成功恢复
        assert!(!manager.status().kill_all);
    }

    #[test]
    fn load_unreadable_state_fails_closed() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("estop-state.json");
        fs::create_dir(&state_path).unwrap();
        let cfg = estop_config(&state_path);

        let manager = EstopManager::load(&cfg, dir.path()).unwrap();

        assert!(manager.status().kill_all);
        assert!(manager.status().is_engaged());
    }

    #[test]
    fn engage_reports_state_directory_creation_failure() {
        let dir = tempdir().unwrap();
        let parent_file = dir.path().join("not-a-directory");
        fs::write(&parent_file, "occupied").unwrap();
        let state_path = parent_file.join("estop-state.json");
        let cfg = estop_config(&state_path);
        let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();

        let error = manager
            .engage(EstopLevel::KillAll)
            .expect_err("a file cannot be used as the state directory");

        assert!(error.to_string().contains("Failed to create estop state dir"));
    }

    #[test]
    fn engage_rejects_blank_tool_name() {
        let dir = tempdir().unwrap();
        let cfg = estop_config(&dir.path().join("estop-state.json"));
        let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();

        let error = manager
            .engage(EstopLevel::ToolFreeze(vec!["   ".into()]))
            .expect_err("blank tool names must be rejected");

        assert!(error.to_string().contains("Tool name must not be empty"));
        assert!(manager.status().frozen_tools.is_empty());
    }

    #[test]
    fn resume_rejects_blank_and_invalid_otp_codes() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("estop-state.json");
        let mut cfg = estop_config(&state_path);
        cfg.require_otp_to_resume = true;
        let otp_cfg = OtpConfig { enabled: true, ..OtpConfig::default() };
        let store = SecretStore::new(dir.path(), true);
        let (validator, _) = OtpValidator::from_config(&otp_cfg, dir.path(), &store).unwrap();
        let mut manager = EstopManager::load(&cfg, dir.path()).unwrap();
        manager.engage(EstopLevel::KillAll).unwrap();

        let blank_error = manager
            .resume(ResumeSelector::KillAll, Some("   "), Some(&validator))
            .expect_err("blank OTP must be treated as missing");
        assert!(blank_error.to_string().contains("OTP code is required"));

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let valid_code = validator.code_for_timestamp(now);
        let invalid_code = if valid_code == "000000" { "000001" } else { "000000" };
        let invalid_error = manager
            .resume(ResumeSelector::KillAll, Some(invalid_code), Some(&validator))
            .expect_err("invalid OTP must deny resume");

        assert!(invalid_error.to_string().contains("Invalid OTP code"));
        assert!(manager.status().kill_all);
    }

    #[test]
    fn resolve_state_file_path_preserves_absolute_path() {
        let dir = tempdir().unwrap();
        let absolute = dir.path().join("absolute-estop.json");

        assert_eq!(
            resolve_state_file_path(
                Path::new("ignored-config-dir"),
                &absolute.display().to_string()
            ),
            absolute
        );
    }
}
