//! Daemon 模块单元测试
//!
//! 本模块包含守护进程（daemon）核心功能的测试用例，覆盖以下方面：
//! - 状态文件路径计算
//! - 关闭信号处理
//! - 优雅关闭机制
//! - 组件监督器
//! - 受监督通道检测
//! - 心跳任务调度
//! - 心跳投递目标验证

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::agent_scheduler::store::AgentSchedulerStore;
    use crate::app::agent::config::schema::{
        DiscordConfig, FeishuConfig, FeishuInstanceConfig, LarkConfig, LarkReceiveMode,
        MattermostConfig, SlackConfig, WebhookConfig,
    };
    use tempfile::TempDir;
    use vw_api_types::common::TimestampMs;

    /// 创建用于测试的配置对象
    ///
    /// # 参数
    ///
    /// - `tmp`: 临时目录引用，用于创建测试工作区
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `Config` 实例，包含：
    /// - 临时工作区目录
    /// - 临时配置文件路径
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let tmp = TempDir::new().unwrap();
    /// let config = test_config(&tmp);
    /// // config.workspace_dir 和 config.config_path 都指向 tmp 内的路径
    /// ```
    fn test_config(tmp: &TempDir) -> Config {
        let config = Config {
            workspace_dir: tmp.path().join("workspace"),
            config_path: tmp.path().join("vibewindow.json"),
            ..Config::default()
        };
        // 确保工作区目录存在
        std::fs::create_dir_all(&config.workspace_dir).unwrap();
        config
    }

    /// 测试状态文件路径应使用配置目录
    ///
    /// 验证 `state_file_path` 函数返回的路径是否正确指向配置目录下的
    /// `daemon_state.json` 文件
    #[test]
    fn state_file_path_uses_config_directory() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);

        let path = state_file_path(&config);
        // 状态文件应该在配置文件的同一目录下，命名为 daemon_state.json
        assert_eq!(path, tmp.path().join("daemon_state.json"));
    }

    /// 测试 Ctrl+C 关闭原因应提及 SIGINT
    ///
    /// 验证 `shutdown_reason` 函数对 `ShutdownSignal::CtrlC` 的处理
    /// 返回包含 "SIGINT" 的描述字符串
    #[test]
    fn shutdown_reason_for_ctrl_c_mentions_sigint() {
        assert_eq!(shutdown_reason(ShutdownSignal::CtrlC), "shutdown requested (SIGINT)");
    }

    /// 测试 SIGTERM 关闭原因应提及 SIGTERM
    ///
    /// 验证 `shutdown_reason` 函数对 `ShutdownSignal::SigTerm` 的处理
    /// 返回包含 "SIGTERM" 的描述字符串
    #[test]
    fn shutdown_reason_for_sigterm_mentions_sigterm() {
        assert_eq!(shutdown_reason(ShutdownSignal::SigTerm), "shutdown requested (SIGTERM)");
    }

    /// 测试关闭提示应匹配平台信号支持
    ///
    /// 验证 `shutdown_hint` 函数根据不同操作系统返回正确的关闭提示：
    /// - Unix 系统：支持 Ctrl+C 和 SIGTERM
    /// - 非 Unix 系统：仅支持 Ctrl+C
    #[test]
    fn shutdown_hint_matches_platform_signal_support() {
        #[cfg(unix)]
        assert_eq!(shutdown_hint(), "Ctrl+C or SIGTERM to stop");

        #[cfg(not(unix))]
        assert_eq!(shutdown_hint(), "Ctrl+C to stop");
    }

    /// 测试关闭时不应中止已完成的句柄
    ///
    /// 验证 `shutdown_handles` 函数在任务已完成时不会中止任何句柄。
    #[tokio::test]
    async fn shutdown_keeps_completed_handles_finished() {
        // 创建一个立即完成的任务
        let finished = tokio::spawn(async {});
        while !finished.is_finished() {
            tokio::task::yield_now().await;
        }
        let stopped = shutdown_handles(vec![finished]).await;
        // 不应中止任何句柄
        assert_eq!(stopped, 0);
    }

    #[tokio::test]
    async fn shutdown_accepts_empty_handle_list() {
        let stopped = shutdown_handles(Vec::new()).await;

        assert_eq!(stopped, 0);
    }

    #[tokio::test]
    async fn shutdown_stops_only_unfinished_handles() {
        let finished = tokio::spawn(async {});
        while !finished.is_finished() {
            tokio::task::yield_now().await;
        }
        let unfinished = tokio::spawn(async {
            std::future::pending::<()>().await;
        });

        let stopped = shutdown_handles(vec![finished, unfinished]).await;

        assert_eq!(stopped, 1);
    }

    /// 测试关闭时应中止卡住的句柄
    ///
    /// 验证 `shutdown_handles` 函数会停止未完成句柄，防止无限期阻塞。
    #[tokio::test]
    async fn shutdown_stops_stuck_handles_without_delay() {
        // 创建一个永远不会完成的任务（需 30 秒）
        let never_finishes = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(30)).await;
        });
        let started = tokio::time::Instant::now();
        let stopped = shutdown_handles(vec![never_finishes]).await;

        // 应停止 1 个句柄
        assert_eq!(stopped, 1);
        // 验证关闭不应无限期阻塞（应在 2 秒内完成）
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "shutdown should not block indefinitely"
        );
    }

    /// 测试监督器应在失败时标记错误并重启
    ///
    /// 验证 `spawn_component_supervisor` 函数在组件失败时：
    /// - 将组件状态标记为 "error"
    /// - 增加重启计数
    /// - 记录最后错误信息
    #[tokio::test]
    async fn supervisor_marks_error_and_restart_on_failure() {
        // 启动一个会立即失败（bail）的组件监督器
        let handle = spawn_component_supervisor("daemon-test-fail", 1, 1, || async {
            anyhow::bail!("boom")
        });

        // 等待组件执行并记录状态
        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();
        let _ = handle.await;

        // 检查健康快照中的组件状态
        let snapshot = crate::app::agent::health::snapshot_json();
        let component = &snapshot["components"]["daemon-test-fail"];
        assert_eq!(component["status"], "error");
        // 重启计数应至少为 1
        assert!(component["restart_count"].as_u64().unwrap_or(0) >= 1);
        // 错误信息应包含 "boom"
        assert!(component["last_error"].as_str().unwrap_or("").contains("boom"));
    }

    /// 测试监督器应将意外退出标记为错误
    ///
    /// 验证 `spawn_component_supervisor` 函数在组件意外成功退出（Ok(())）
    /// 时也会将其标记为错误状态，因为守护组件应该持续运行
    #[tokio::test]
    async fn supervisor_marks_unexpected_exit_as_error() {
        // 启动一个会立即成功退出的组件监督器
        let handle = spawn_component_supervisor("daemon-test-exit", 1, 1, || async { Ok(()) });

        // 等待组件执行并记录状态
        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();
        let _ = handle.await;

        // 检查健康快照中的组件状态
        let snapshot = crate::app::agent::health::snapshot_json();
        let component = &snapshot["components"]["daemon-test-exit"];
        assert_eq!(component["status"], "error");
        // 重启计数应至少为 1
        assert!(component["restart_count"].as_u64().unwrap_or(0) >= 1);
        // 错误信息应包含 "component exited unexpectedly"
        assert!(
            component["last_error"]
                .as_str()
                .unwrap_or("")
                .contains("component exited unexpectedly")
        );
    }

    /// 测试检测无受监督通道
    ///
    /// 验证默认配置下 `has_supervised_channels` 返回 false，
    /// 因为没有配置任何通道
    #[test]
    fn detects_no_supervised_channels() {
        let config = Config::default();
        assert!(!has_supervised_channels(&config));
    }

    #[test]
    fn configured_webhook_is_not_a_supervised_channel() {
        let mut config = Config::default();
        config.channels_config.webhook = Some(WebhookConfig { port: 8787, secret: None });

        assert!(!has_supervised_channels(&config));
    }

    #[test]
    fn blank_feishu_is_not_supervised() {
        let mut config = Config::default();
        config.channels_config.lark = Some(LarkConfig {
            app_id: "legacy-app".to_string(),
            app_secret: " ".to_string(),
            encrypt_key: None,
            verification_token: None,
            allowed_users: Vec::new(),
            mention_only: false,
            group_reply: None,
            use_feishu: true,
            receive_mode: LarkReceiveMode::Websocket,
            port: None,
            draft_update_interval_ms: 3000,
            max_draft_edits: 20,
        });
        config.channels_config.feishu = Some(FeishuConfig {
            app_id: String::new(),
            app_secret: "secret".to_string(),
            encrypt_key: None,
            verification_token: None,
            allowed_users: Vec::new(),
            group_reply: None,
            receive_mode: LarkReceiveMode::Websocket,
            port: None,
            draft_update_interval_ms: 3000,
            max_draft_edits: 20,
        });
        config.channels_config.feishu_instances.push(FeishuInstanceConfig {
            id: "partial".to_string(),
            config: FeishuConfig {
                app_id: "instance-app".to_string(),
                app_secret: "\t".to_string(),
                encrypt_key: None,
                verification_token: None,
                allowed_users: Vec::new(),
                group_reply: None,
                receive_mode: LarkReceiveMode::Websocket,
                port: None,
                draft_update_interval_ms: 3000,
                max_draft_edits: 20,
            },
        });

        assert!(!has_supervised_channels(&config));
    }

    /// 测试检测 Telegram 为受监督通道
    ///
    /// 验证配置 Telegram 通道后 `has_supervised_channels` 返回 true
    #[test]
    fn detects_telegram_as_supervised_channel() {
        let mut config = Config::default();
        // 配置 Telegram 通道（仅需必填字段）
        config.channels_config.telegram = Some(crate::app::agent::config::TelegramConfig {
            bot_token: "token".into(),
            allowed_users: vec![],
            stream_mode: crate::app::agent::config::StreamMode::default(),
            draft_update_interval_ms: 1000,
            interrupt_on_new_message: false,
            mention_only: false,
            group_reply: None,
            base_url: None,
        });
        assert!(has_supervised_channels(&config));
    }

    /// 测试检测钉钉为受监督通道
    ///
    /// 验证配置钉钉通道后 `has_supervised_channels` 返回 true
    #[test]
    fn detects_dingtalk_as_supervised_channel() {
        let mut config = Config::default();
        // 配置钉钉通道（仅需必填字段）
        config.channels_config.dingtalk = Some(crate::app::agent::config::schema::DingTalkConfig {
            client_id: "client_id".into(),
            client_secret: "client_secret".into(),
            allowed_users: vec!["*".into()],
        });
        assert!(has_supervised_channels(&config));
    }

    /// 测试检测 Mattermost 为受监督通道
    ///
    /// 验证配置 Mattermost 通道后 `has_supervised_channels` 返回 true
    #[test]
    fn detects_mattermost_as_supervised_channel() {
        let mut config = Config::default();
        // 配置 Mattermost 通道（仅需必填字段）
        config.channels_config.mattermost =
            Some(crate::app::agent::config::schema::MattermostConfig {
                url: "https://mattermost.example.com".into(),
                bot_token: "token".into(),
                channel_id: Some("channel-id".into()),
                allowed_users: vec!["*".into()],
                thread_replies: Some(true),
                mention_only: Some(false),
                group_reply: None,
            });
        assert!(has_supervised_channels(&config));
    }

    /// 测试检测 QQ 为受监督通道
    ///
    /// 验证配置 QQ 通道后 `has_supervised_channels` 返回 true
    #[test]
    fn detects_qq_as_supervised_channel() {
        let mut config = Config::default();
        // 配置 QQ 通道（仅需必填字段）
        config.channels_config.qq = Some(crate::app::agent::config::schema::QQConfig {
            app_id: "app-id".into(),
            app_secret: "app-secret".into(),
            allowed_users: vec!["*".into()],
            receive_mode: crate::app::agent::config::schema::QQReceiveMode::Websocket,
        });
        assert!(has_supervised_channels(&config));
    }

    /// 测试检测 Nextcloud Talk 为受监督通道
    ///
    /// 验证配置 Nextcloud Talk 通道后 `has_supervised_channels` 返回 true
    #[test]
    fn detects_nextcloud_talk_as_supervised_channel() {
        let mut config = Config::default();
        // 配置 Nextcloud Talk 通道（仅需必填字段）
        config.channels_config.nextcloud_talk =
            Some(crate::app::agent::config::schema::NextcloudTalkConfig {
                base_url: "https://cloud.example.com".into(),
                app_token: "app-token".into(),
                webhook_secret: None,
                allowed_users: vec!["*".into()],
            });
        assert!(has_supervised_channels(&config));
    }

    /// 测试心跳任务应优先使用文件任务
    ///
    /// 验证 `heartbeat_tasks_for_tick` 函数在文件任务可用时
    /// 使用文件任务，而不是配置中的回退消息
    #[test]
    fn heartbeat_tasks_use_file_tasks_when_available() {
        // 文件任务列表非空，应使用文件任务
        let tasks =
            heartbeat_tasks_for_tick(vec!["From file".to_string()], Some("Fallback from config"));
        assert_eq!(tasks, vec!["From file".to_string()]);
    }

    /// 测试心跳任务应回退到配置消息
    ///
    /// 验证 `heartbeat_tasks_for_tick` 函数在文件任务为空时
    /// 使用配置中的回退消息，并自动去除空白
    #[test]
    fn heartbeat_tasks_fall_back_to_config_message() {
        // 文件任务为空，应使用配置消息（去除前后空白）
        let tasks = heartbeat_tasks_for_tick(vec![], Some("  check london time  "));
        assert_eq!(tasks, vec!["check london time".to_string()]);
    }

    /// 测试心跳任务应忽略空白回退消息
    ///
    /// 验证 `heartbeat_tasks_for_tick` 函数在文件任务为空且
    /// 配置回退消息仅含空白时，返回空列表
    #[test]
    fn heartbeat_tasks_ignore_empty_fallback_message() {
        // 文件任务为空，配置消息仅含空白，应返回空列表
        let tasks = heartbeat_tasks_for_tick(vec![], Some("   "));
        assert!(tasks.is_empty());
    }

    #[test]
    fn heartbeat_tasks_are_empty_without_file_tasks_or_fallback() {
        let tasks = heartbeat_tasks_for_tick(Vec::new(), None);

        assert!(tasks.is_empty());
    }

    /// 测试心跳投递目标在未配置时应返回 None
    ///
    /// 验证默认配置下 `heartbeat_delivery_target` 返回 `Ok(None)`
    #[test]
    fn heartbeat_delivery_target_none_when_unset() {
        let config = Config::default();
        let target = heartbeat_delivery_target(&config).unwrap();
        assert!(target.is_none());
    }

    #[test]
    fn heartbeat_delivery_target_treats_whitespace_fields_as_unset() {
        let mut config = Config::default();
        config.heartbeat.target = Some("   ".into());
        config.heartbeat.to = Some("\t".into());

        assert!(heartbeat_delivery_target(&config).unwrap().is_none());

        config.heartbeat.to = Some("recipient".into());
        let err = heartbeat_delivery_target(&config).unwrap_err();
        assert!(err.to_string().contains("heartbeat.target is required"));

        config.heartbeat.target = Some("telegram".into());
        config.heartbeat.to = Some("\n".into());
        let err = heartbeat_delivery_target(&config).unwrap_err();
        assert!(err.to_string().contains("heartbeat.to is required"));
    }

    /// 测试心跳投递目标在缺少 to 字段时应报错
    ///
    /// 验证仅配置 target 而不配置 to 时，
    /// `heartbeat_delivery_target` 返回包含明确错误提示的错误
    #[test]
    fn heartbeat_delivery_target_requires_to_field() {
        let mut config = Config::default();
        // 仅设置 target，不设置 to
        config.heartbeat.target = Some("telegram".into());
        let err = heartbeat_delivery_target(&config).unwrap_err();
        assert!(err.to_string().contains("heartbeat.to is required when heartbeat.target is set"));
    }

    /// 测试心跳投递目标在缺少 target 字段时应报错
    ///
    /// 验证仅配置 to 而不配置 target 时，
    /// `heartbeat_delivery_target` 返回包含明确错误提示的错误
    #[test]
    fn heartbeat_delivery_target_requires_target_field() {
        let mut config = Config::default();
        // 仅设置 to，不设置 target
        config.heartbeat.to = Some("123456".into());
        let err = heartbeat_delivery_target(&config).unwrap_err();
        assert!(err.to_string().contains("heartbeat.target is required when heartbeat.to is set"));
    }

    /// 测试心跳投递目标应拒绝不支持的通道
    ///
    /// 验证配置不支持的通道类型（如 email）时，
    /// `heartbeat_delivery_target` 返回包含明确错误提示的错误
    #[test]
    fn heartbeat_delivery_target_rejects_unsupported_channel() {
        let mut config = Config::default();
        // 设置不支持的通道类型
        config.heartbeat.target = Some("email".into());
        config.heartbeat.to = Some("ops@example.com".into());
        let err = heartbeat_delivery_target(&config).unwrap_err();
        assert!(err.to_string().contains("unsupported heartbeat.target channel"));
    }

    /// 测试心跳投递目标应要求通道配置
    ///
    /// 验证设置 heartbeat.target 为 telegram 但未配置
    /// channels_config.telegram 时，返回包含明确错误提示的错误
    #[test]
    fn heartbeat_delivery_target_requires_channel_configuration() {
        let mut config = Config::default();
        // 设置 target 为 telegram，但未配置 telegram 通道
        config.heartbeat.target = Some("telegram".into());
        config.heartbeat.to = Some("123456".into());
        let err = heartbeat_delivery_target(&config).unwrap_err();
        assert!(err.to_string().contains("channels_config.telegram is not configured"));
    }

    #[test]
    fn heartbeat_delivery_target_requires_each_supported_channel_configuration() {
        for (channel, expected_field) in [
            ("discord", "channels_config.discord"),
            ("slack", "channels_config.slack"),
            ("mattermost", "channels_config.mattermost"),
        ] {
            let mut config = Config::default();
            config.heartbeat.target = Some(channel.into());
            config.heartbeat.to = Some("recipient".into());

            let err = heartbeat_delivery_target(&config).unwrap_err();
            assert!(
                err.to_string().contains(expected_field),
                "{channel} should require {expected_field}"
            );
        }
    }

    /// 测试心跳投递目标应接受有效的 Telegram 配置
    ///
    /// 验证正确配置 target、to 和对应通道配置时，
    /// `heartbeat_delivery_target` 返回正确的投递目标元组
    #[test]
    fn heartbeat_delivery_target_accepts_telegram_configuration() {
        let mut config = Config::default();
        // 完整配置：target、to 和 telegram 通道配置
        config.heartbeat.target = Some("telegram".into());
        config.heartbeat.to = Some("123456".into());
        config.channels_config.telegram = Some(crate::app::agent::config::TelegramConfig {
            bot_token: "bot-token".into(),
            allowed_users: vec![],
            stream_mode: crate::app::agent::config::StreamMode::default(),
            draft_update_interval_ms: 1000,
            interrupt_on_new_message: false,
            mention_only: false,
            group_reply: None,
            base_url: None,
        });

        let target = heartbeat_delivery_target(&config).unwrap();
        // 应返回 (通道名, 目标ID) 元组
        assert_eq!(target, Some(("telegram".to_string(), "123456".to_string())));
    }

    #[test]
    fn heartbeat_delivery_target_accepts_other_supported_channels() {
        let mut config = Config::default();
        config.heartbeat.to = Some("recipient".into());
        config.channels_config.discord = Some(DiscordConfig {
            bot_token: "bot-token".into(),
            guild_id: None,
            allowed_users: Vec::new(),
            listen_to_bots: false,
            mention_only: false,
            group_reply: None,
        });
        config.channels_config.slack = Some(SlackConfig {
            bot_token: "bot-token".into(),
            app_token: None,
            channel_id: None,
            allowed_users: Vec::new(),
            group_reply: None,
        });
        config.channels_config.mattermost = Some(MattermostConfig {
            url: "https://mattermost.example.test".into(),
            bot_token: "bot-token".into(),
            channel_id: None,
            allowed_users: Vec::new(),
            thread_replies: None,
            mention_only: None,
            group_reply: None,
        });

        for channel in ["discord", "slack", "mattermost"] {
            config.heartbeat.target = Some(channel.into());
            assert_eq!(
                heartbeat_delivery_target(&config).unwrap(),
                Some((channel.to_string(), "recipient".to_string()))
            );
        }
    }

    #[test]
    fn state_file_path_uses_current_directory_when_config_has_no_parent() {
        let mut config = Config::default();
        config.config_path = PathBuf::new();

        assert_eq!(state_file_path(&config), PathBuf::from(".").join("daemon_state.json"));
    }

    #[test]
    fn gateway_base_url_normalizes_probe_hosts_and_ipv6() {
        assert_eq!(gateway_base_url("", 8080), "http://127.0.0.1:8080");
        assert_eq!(gateway_base_url("0.0.0.0", 8080), "http://127.0.0.1:8080");
        assert_eq!(gateway_base_url("::", 8080), "http://[::1]:8080");
        assert_eq!(gateway_base_url("localhost", 8080), "http://localhost:8080");
        assert_eq!(gateway_base_url("2001:db8::1", 8080), "http://[2001:db8::1]:8080");
    }

    #[test]
    fn gateway_base_url_trims_hosts_and_preserves_bracketed_ipv6() {
        assert_eq!(gateway_base_url("  localhost  ", 8080), "http://localhost:8080");
        assert_eq!(gateway_base_url("[::1]", 8080), "http://[::1]:8080");
    }

    #[test]
    fn gateway_probe_urls_append_expected_paths() {
        assert_eq!(gateway_health_url("localhost", 7777), "http://localhost:7777/v1/health");
        assert_eq!(
            gateway_cron_history_probe_url("localhost", 7777),
            "http://localhost:7777/v1/cron/runs/__probe__"
        );
    }

    #[test]
    fn addr_in_use_gateway_errors_are_non_retryable() {
        let err =
            anyhow::Error::new(std::io::Error::new(ErrorKind::AddrInUse, "already listening"));

        assert!(is_non_retryable_component_error("gateway", &err));
        assert!(!is_non_retryable_component_error("channels", &err));
    }

    #[test]
    fn gateway_error_retryability_checks_error_kind_and_entire_chain() {
        let retryable = anyhow::Error::new(std::io::Error::new(
            ErrorKind::ConnectionRefused,
            "connection refused",
        ));
        assert!(!is_non_retryable_component_error("gateway", &retryable));

        let wrapped =
            anyhow::Error::new(std::io::Error::new(ErrorKind::AddrInUse, "already listening"))
                .context("gateway startup failed");
        assert!(is_non_retryable_component_error("gateway", &wrapped));
    }

    #[tokio::test]
    async fn gateway_supervisor_stops_on_non_retryable_addr_in_use() {
        let component = "gateway";
        let handle = spawn_component_supervisor(component, 1, 1, || async {
            Err(anyhow::Error::new(std::io::Error::new(ErrorKind::AddrInUse, "already listening")))
        });

        tokio::time::timeout(Duration::from_millis(200), handle)
            .await
            .expect("non-retryable gateway supervisor should finish")
            .unwrap();

        let snapshot = crate::app::agent::health::snapshot_json();
        let entry = &snapshot["components"][component];
        assert_eq!(entry["status"], "error");
        assert_eq!(entry["restart_count"].as_u64().unwrap_or(0), 0);
    }

    #[tokio::test]
    async fn state_writer_creates_parent_directory_and_writes_json() {
        let tmp = TempDir::new().unwrap();
        let mut config = test_config(&tmp);
        config.config_path = tmp.path().join("nested").join("vibewindow.json");
        crate::app::agent::health::mark_component_ok("daemon-test-state-writer");

        let handle = spawn_state_writer(config.clone());
        let path = state_file_path(&config);
        tokio::time::timeout(Duration::from_millis(500), async {
            loop {
                if path.exists() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("state writer should create the state file");
        handle.abort();
        let _ = handle.await;

        let raw = std::fs::read_to_string(path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert!(json.get("written_at").and_then(serde_json::Value::as_str).is_some());
        assert_eq!(json["components"]["daemon-test-state-writer"]["status"], "ok");
    }

    #[test]
    fn heartbeat_delivery_target_trims_channel_and_target() {
        let mut config = Config::default();
        config.heartbeat.target = Some(" telegram ".into());
        config.heartbeat.to = Some(" 123456 ".into());
        config.channels_config.telegram = Some(crate::app::agent::config::TelegramConfig {
            bot_token: "bot-token".into(),
            allowed_users: vec![],
            stream_mode: crate::app::agent::config::StreamMode::default(),
            draft_update_interval_ms: 1000,
            interrupt_on_new_message: false,
            mention_only: false,
            group_reply: None,
            base_url: None,
        });

        let target = heartbeat_delivery_target(&config).unwrap();
        assert_eq!(target, Some(("telegram".to_string(), "123456".to_string())));
    }
}
