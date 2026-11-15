//! 浏览器工具模块的单元测试集
//!
//! 本模块包含浏览器工具（BrowserTool）及其相关组件的全面测试用例，
//! 涵盖以下测试领域：
//!
//! - 域名规范化与主机提取测试
//! - 私有主机检测（防止SSRF攻击）
//! - URL验证与白名单匹配
//! - 浏览器后端类型解析与配置
//! - Computer Use功能的安全限制
//! - 会话错误恢复检测
//!
//! 测试遵循最小权限原则，重点验证安全策略执行情况。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;
    use crate::actions::is_computer_use_only_action;
    use crate::backend::{
        BrowserBackendKind, ResolvedBackend, unavailable_action_for_backend_error,
    };
    use crate::helpers::{
        extract_host, host_matches_allowlist, is_private_host, is_recoverable_rust_native_error,
        normalize_domains,
    };
    use tool_types::Tool;
    use std::path::Path;

    /// 创建目录符号链接（Unix平台实现）
    ///
    /// 在Unix系统上，使用标准symlink创建目录符号链接。
    ///
    /// 参数：
    /// - `src`: 源路径
    /// - `dst`: 目标链接路径
    ///
    /// 失败时会触发panic，因为这是测试辅助函数
    #[cfg(unix)]
    fn symlink_dir(src: &Path, dst: &Path) {
        std::os::unix::fs::symlink(src, dst).expect("symlink should be created");
    }

    /// 创建目录符号链接（Windows平台实现）
    ///
    /// 在Windows系统上，使用symlink_dir创建目录符号链接。
    ///
    /// 参数：
    /// - `src`: 源路径
    /// - `dst`: 目标链接路径
    ///
    /// 失败时会触发panic，因为这是测试辅助函数
    #[cfg(windows)]
    fn symlink_dir(src: &Path, dst: &Path) {
        std::os::windows::fs::symlink_dir(src, dst).expect("symlink should be created");
    }

    /// 测试域名规范化功能
    ///
    /// 验证normalize_domains函数能正确处理：
    /// - 移除首尾空白字符
    /// - 转换为小写
    /// - 过滤空字符串
    #[test]
    fn normalize_domains_works() {
        let domains = vec!["  Example.COM  ".into(), "docs.example.com".into(), String::new()];
        let normalized = normalize_domains(domains);
        assert_eq!(normalized, vec!["example.com", "docs.example.com"]);
    }

    /// 测试主机提取功能
    ///
    /// 验证extract_host函数能从URL中正确提取主机名并规范化为小写
    #[test]
    fn extract_host_works() {
        assert_eq!(extract_host("https://example.com/path").unwrap(), "example.com");
        assert_eq!(extract_host("https://Sub.Example.COM:8080/").unwrap(), "sub.example.com");
    }

    /// 测试IPv6地址的主机提取
    ///
    /// 验证extract_host函数能正确处理IPv6地址格式：
    /// - 带方括号的IPv6地址（URL标准格式）
    /// - 带端口和方括号的IPv6地址
    /// - 带尾部斜杠的IPv6地址
    #[test]
    fn extract_host_handles_ipv6() {
        // IPv6地址必须用方括号包裹（URL带端口时的标准格式）
        assert_eq!(extract_host("https://[::1]/path").unwrap(), "[::1]");
        // IPv6地址带方括号和端口号
        assert_eq!(extract_host("https://[2001:db8::1]:8080/path").unwrap(), "[2001:db8::1]");
        // IPv6地址带方括号和尾部斜杠
        assert_eq!(extract_host("https://[fe80::1]/").unwrap(), "[fe80::1]");
    }

    /// 测试私有主机检测功能 - 本地地址
    ///
    /// 验证is_private_host函数能正确识别各类私有/本地地址：
    /// - localhost及其子域名
    /// - .local本地域名
    /// - 私有IP地址段（127.x、192.168.x、10.x）
    /// - 公网域名应返回false
    #[test]
    fn is_private_host_detects_local() {
        assert!(is_private_host("localhost"));
        assert!(is_private_host("app.localhost"));
        assert!(is_private_host("printer.local"));
        assert!(is_private_host("127.0.0.1"));
        assert!(is_private_host("192.168.1.1"));
        assert!(is_private_host("10.0.0.1"));
        assert!(!is_private_host("example.com"));
        assert!(!is_private_host("google.com"));
    }

    /// 测试私有的主机检测功能 - 组播和保留地址
    ///
    /// 验证is_private_host函数能阻止特殊用途的IP地址：
    /// - 组播地址（224.x.x.x）
    /// - 广播地址（255.255.255.255）
    /// - 共享地址空间（100.64.x.x）
    /// - 保留地址（240.x.x.x）
    /// - 文档用途地址（192.0.2.x、198.51.100.x、203.0.113.x）
    /// - 基准测试地址（198.18.x.x）
    #[test]
    fn is_private_host_blocks_multicast_and_reserved() {
        assert!(is_private_host("224.0.0.1")); // 组播地址
        assert!(is_private_host("255.255.255.255")); // 广播地址
        assert!(is_private_host("100.64.0.1")); // 共享地址空间（CGNAT）
        assert!(is_private_host("240.0.0.1")); // 保留地址
        assert!(is_private_host("192.0.2.1")); // 文档用途地址（TEST-NET-1）
        assert!(is_private_host("198.51.100.1")); // 文档用途地址（TEST-NET-2）
        assert!(is_private_host("203.0.113.1")); // 文档用途地址（TEST-NET-3）
        assert!(is_private_host("198.18.0.1")); // 基准测试地址
    }

    /// 测试私有的主机检测功能 - IPv6回环地址
    ///
    /// 验证is_private_host函数能识别IPv6回环地址和特殊格式
    #[test]
    fn is_private_host_catches_ipv6() {
        assert!(is_private_host("::1"));
        assert!(is_private_host("[::1]"));
        assert!(is_private_host("0.0.0.0"));
    }

    /// 测试私有的主机检测功能 - IPv4映射的IPv6地址
    ///
    /// 验证is_private_host函数能识别IPv4映射的IPv6地址格式，
    /// 防止通过IPv6格式绕过IPv4私有地址检测
    #[test]
    fn is_private_host_catches_mapped_ipv4() {
        // IPv4映射的IPv6地址（::ffff:x.x.x.x格式）
        assert!(is_private_host("::ffff:127.0.0.1"));
        assert!(is_private_host("::ffff:10.0.0.1"));
        assert!(is_private_host("::ffff:192.168.1.1"));
    }

    /// 测试私有的主机检测功能 - IPv6私有地址段
    ///
    /// 验证is_private_host函数能识别IPv6私有地址范围：
    /// - 唯一本地地址（fc00::/7）
    /// - 链路本地地址（fe80::/10）
    /// - 公网IPv6地址应返回false
    #[test]
    fn is_private_host_catches_ipv6_private_ranges() {
        // 唯一本地地址（fc00::/7，相当于IPv4的私有地址）
        assert!(is_private_host("fd00::1"));
        assert!(is_private_host("fc00::1"));
        // 链路本地地址（fe80::/10）
        assert!(is_private_host("fe80::1"));
        // 公网IPv6地址应通过检测
        assert!(!is_private_host("2001:db8::1"));
    }

    /// 测试URL验证阻止IPv6 SSRF攻击
    ///
    /// 验证即使允许所有域名（*），URL验证也会阻止：
    /// - IPv6回环地址（[::1]）
    /// - IPv4映射的IPv6本地地址
    /// - IPv4映射的私有网络地址
    #[test]
    fn validate_url_blocks_ipv6_ssrf() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new(security, vec!["*".into()], None);
        assert!(tool.validate_url("https://[::1]/").is_err());
        assert!(tool.validate_url("https://[::ffff:127.0.0.1]/").is_err());
        assert!(tool.validate_url("https://[::ffff:10.0.0.1]:8080/").is_err());
    }

    /// 测试主机白名单匹配 - 精确匹配
    ///
    /// 验证host_matches_allowlist函数支持精确域名匹配，
    /// 同时也匹配子域名
    #[test]
    fn host_matches_allowlist_exact() {
        let allowed = vec!["example.com".into()];
        assert!(host_matches_allowlist("example.com", &allowed));
        assert!(host_matches_allowlist("sub.example.com", &allowed));
        assert!(!host_matches_allowlist("notexample.com", &allowed));
    }

    /// 测试主机白名单匹配 - 通配符域名
    ///
    /// 验证host_matches_allowlist函数支持通配符域名格式（*.example.com），
    /// 通配符可以匹配主域名本身及其所有子域名
    #[test]
    fn host_matches_allowlist_wildcard() {
        let allowed = vec!["*.example.com".into()];
        assert!(host_matches_allowlist("sub.example.com", &allowed));
        assert!(host_matches_allowlist("example.com", &allowed));
        assert!(!host_matches_allowlist("other.com", &allowed));
    }

    /// 测试主机白名单匹配 - 全局通配符
    ///
    /// 验证"*"通配符允许匹配任意域名（但仍然受私有的主机检测限制）
    #[test]
    fn host_matches_allowlist_star() {
        let allowed = vec!["*".into()];
        assert!(host_matches_allowlist("anything.com", &allowed));
        assert!(host_matches_allowlist("example.org", &allowed));
    }

    /// 测试浏览器后端类型解析器 - 有效值
    ///
    /// 验证BrowserBackendKind::parse能正确解析所有支持的后端类型：
    /// - agent_browser：Agent浏览器模式
    /// - rust-native：Rust原生模式
    /// - computer_use：Computer Use API模式
    /// - auto：自动选择模式
    #[test]
    fn browser_backend_parser_accepts_supported_values() {
        assert_eq!(
            BrowserBackendKind::parse("agent_browser").unwrap(),
            BrowserBackendKind::AgentBrowser
        );
        assert_eq!(
            BrowserBackendKind::parse("rust-native").unwrap(),
            BrowserBackendKind::RustNative
        );
        assert_eq!(
            BrowserBackendKind::parse("computer_use").unwrap(),
            BrowserBackendKind::ComputerUse
        );
        assert_eq!(BrowserBackendKind::parse("auto").unwrap(), BrowserBackendKind::Auto);
    }

    /// 测试浏览器后端类型解析器 - 无效值
    ///
    /// 验证BrowserBackendKind::parse会拒绝未知或不支持的后端类型
    #[test]
    fn browser_backend_parser_rejects_unknown_values() {
        assert!(BrowserBackendKind::parse("playwright").is_err());
    }

    /// 测试浏览器工具默认后端
    ///
    /// 验证使用new构造函数创建的BrowserTool默认使用AgentBrowser后端
    #[test]
    fn browser_tool_default_backend_is_agent_browser() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new(security, vec!["example.com".into()], None);
        assert_eq!(tool.configured_backend().unwrap(), BrowserBackendKind::AgentBrowser);
    }

    /// 测试浏览器工具接受auto后端配置
    ///
    /// 验证BrowserTool能正确配置和使用auto自动选择后端模式
    #[test]
    fn browser_tool_accepts_auto_backend_config() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new_with_backend(
            security,
            vec!["example.com".into()],
            None,
            "auto".into(),
            true,
            "http://127.0.0.1:9515".into(),
            None,
            ComputerUseConfig::default(),
        );
        assert_eq!(tool.configured_backend().unwrap(), BrowserBackendKind::Auto);
    }

    /// 测试浏览器工具接受Computer Use后端配置
    ///
    /// 验证BrowserTool能正确配置和使用Computer Use API后端
    #[test]
    fn browser_tool_accepts_computer_use_backend_config() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new_with_backend(
            security,
            vec!["example.com".into()],
            None,
            "computer_use".into(),
            true,
            "http://127.0.0.1:9515".into(),
            None,
            ComputerUseConfig::default(),
        );
        assert_eq!(tool.configured_backend().unwrap(), BrowserBackendKind::ComputerUse);
    }

    /// 测试Computer Use端点默认拒绝公网HTTP
    ///
    /// 验证安全策略：默认情况下，Computer Use端点不允许使用HTTP协议访问公网地址，
    /// 以防止凭证在传输过程中被截获
    #[test]
    fn computer_use_endpoint_rejects_public_http_by_default() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new_with_backend(
            security,
            vec!["example.com".into()],
            None,
            "computer_use".into(),
            true,
            "http://127.0.0.1:9515".into(),
            None,
            ComputerUseConfig {
                endpoint: "http://computer-use.example.com/v1/actions".into(),
                ..ComputerUseConfig::default()
            },
        );

        assert!(tool.computer_use_endpoint_url().is_err());
    }

    /// 测试Computer Use端点要求公网地址使用HTTPS
    ///
    /// 验证当明确允许远程端点时，公网地址必须使用HTTPS协议，
    /// 这确保了通信安全
    #[test]
    fn computer_use_endpoint_requires_https_for_public_remote() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new_with_backend(
            security,
            vec!["example.com".into()],
            None,
            "computer_use".into(),
            true,
            "http://127.0.0.1:9515".into(),
            None,
            ComputerUseConfig {
                endpoint: "https://computer-use.example.com/v1/actions".into(),
                allow_remote_endpoint: true,
                ..ComputerUseConfig::default()
            },
        );

        assert!(tool.computer_use_endpoint_url().is_ok());
    }

    /// 测试Computer Use坐标验证应用限制
    ///
    /// 验证坐标输入验证会应用配置的最大坐标限制：
    /// - 在范围内的坐标应通过验证
    /// - 超出最大X坐标的应被拒绝
    /// - 负数坐标应被拒绝
    #[test]
    fn computer_use_coordinate_validation_applies_limits() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new_with_backend(
            security,
            vec!["example.com".into()],
            None,
            "computer_use".into(),
            true,
            "http://127.0.0.1:9515".into(),
            None,
            ComputerUseConfig {
                max_coordinate_x: Some(100),
                max_coordinate_y: Some(100),
                ..ComputerUseConfig::default()
            },
        );
        let client = super::computer_use_client_for_tests(&tool);
        let max_x = tool.computer_use.max_coordinate_x;
        let max_y = tool.computer_use.max_coordinate_y;

        assert!(client.validate_coordinate("x", 50, max_x).is_ok());
        assert!(client.validate_coordinate("x", 101, max_x).is_err());
        assert!(client.validate_coordinate("y", -1, max_y).is_err());
    }

    /// 测试截图路径验证阻止逃逸路径
    ///
    /// 验证输出路径验证防止路径遍历攻击：
    /// - 绝对路径（如/etc/passwd）应被拒绝
    /// - 包含../的路径应被拒绝
    /// - 相对路径应在允许的目录范围内
    #[test]
    fn screenshot_path_validation_blocks_escaped_paths() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new(security, vec!["example.com".into()], None);
        let client = super::computer_use_client_for_tests(&tool);
        assert!(client.validate_output_path("path", "/etc/passwd").is_err());
        assert!(client.validate_output_path("path", "../outside.png").is_err());
        assert!(client.validate_output_path("path", "captures/page.png").is_ok());
    }

    /// 测试Computer Use键盘动作参数验证
    ///
    /// 验证键盘相关动作的参数验证：
    /// - key_type动作需要text参数
    /// - key_press动作需要有效的key参数（符合键盘键名规范）
    #[test]
    fn computer_use_key_actions_validate_params() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new_with_backend(
            security,
            vec!["example.com".into()],
            None,
            "computer_use".into(),
            true,
            "http://127.0.0.1:9515".into(),
            None,
            ComputerUseConfig::default(),
        );
        let client = super::computer_use_client_for_tests(&tool);

        // key_type动作测试：有效的text参数
        let key_type_args = serde_json::json!({"text": "hello"});
        assert!(client.validate_action("key_type", key_type_args.as_object().unwrap()).is_ok());
        // key_type动作测试：缺少text参数
        let missing_key_type = serde_json::json!({});
        assert!(client.validate_action("key_type", missing_key_type.as_object().unwrap()).is_err());

        // key_press动作测试：有效的key参数
        let key_press_args = serde_json::json!({"key": "Enter"});
        assert!(client.validate_action("key_press", key_press_args.as_object().unwrap()).is_ok());
        // key_press动作测试：无效的key参数（包含非法字符）
        let bad_key_press_args = serde_json::json!({"key": "Ctrl+Shift+Enter!!"});
        assert!(
            client.validate_action("key_press", bad_key_press_args.as_object().unwrap()).is_err()
        );
    }

    /// 测试浏览器工具名称
    ///
    /// 验证BrowserTool的name方法返回正确的工具标识符"browser"
    #[test]
    fn browser_tool_name() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new(security, vec!["example.com".into()], None);
        assert_eq!(tool.name(), "browser");
    }

    /// 测试浏览器工具URL验证
    ///
    /// 全面验证URL验证功能的各项安全检查：
    /// - 白名单内的HTTPS URL应通过
    /// - 白名单外的URL应被拒绝
    /// - 私有主机地址应被拒绝（SSRF防护）
    /// - 非HTTPS协议应被拒绝
    /// - file://协议应被阻止（防止本地文件泄露）
    #[test]
    fn browser_tool_validates_url() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new(security, vec!["example.com".into()], None);

        // 有效URL：在白名单内且使用HTTPS
        assert!(tool.validate_url("https://example.com").is_ok());
        assert!(tool.validate_url("https://sub.example.com/path").is_ok());

        // 无效URL：不在白名单内
        assert!(tool.validate_url("https://other.com").is_err());

        // 无效URL：私有主机地址
        assert!(tool.validate_url("https://localhost").is_err());
        assert!(tool.validate_url("https://127.0.0.1").is_err());

        // 无效URL：非HTTPS协议
        assert!(tool.validate_url("ftp://example.com").is_err());

        // file:// URL被阻止（防止本地文件泄露风险）
        assert!(tool.validate_url("file:///tmp/test.html").is_err());
    }

    /// 测试空白名单阻止所有URL
    ///
    /// 验证当白名单为空时，所有URL访问都会被拒绝，
    /// 这是一种默认拒绝的安全策略
    #[test]
    fn browser_tool_empty_allowlist_blocks() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new(security, vec![], None);
        assert!(tool.validate_url("https://example.com").is_err());
    }

    /// 测试Computer Use专属动作检测
    ///
    /// 验证is_computer_use_only_action函数能正确识别
    /// 哪些动作只能在Computer Use后端使用：
    /// - 鼠标操作（mouse_move、mouse_click、mouse_drag）
    /// - 键盘操作（key_type、key_press）
    /// - 屏幕捕获（screen_capture）
    /// - 通用动作（open、snapshot）不是Computer Use专属
    #[test]
    fn computer_use_only_action_detection_is_correct() {
        assert!(is_computer_use_only_action("mouse_move"));
        assert!(is_computer_use_only_action("mouse_click"));
        assert!(is_computer_use_only_action("mouse_drag"));
        assert!(is_computer_use_only_action("key_type"));
        assert!(is_computer_use_only_action("key_press"));
        assert!(is_computer_use_only_action("screen_capture"));
        assert!(!is_computer_use_only_action("open"));
        assert!(!is_computer_use_only_action("snapshot"));
    }

    /// 测试不可用动作错误信息包含后端上下文
    ///
    /// 验证unavailable_action_for_backend_error函数生成的错误信息
    /// 包含动作名称和后端类型，便于调试和问题定位
    #[test]
    fn unavailable_action_error_preserves_backend_context() {
        assert_eq!(
            unavailable_action_for_backend_error("mouse_move", ResolvedBackend::AgentBrowser),
            "Action 'mouse_move' is unavailable for backend 'agent_browser'"
        );
        assert_eq!(
            unavailable_action_for_backend_error("mouse_move", ResolvedBackend::RustNative),
            "Action 'mouse_move' is unavailable for backend 'rust_native'"
        );
    }

    /// 测试可恢复错误检测匹配会话模式
    ///
    /// 验证is_recoverable_rust_native_error函数能正确识别
    /// 可以通过重试或重建会话来恢复的临时性错误：
    /// - 无效会话ID
    /// - 窗口不存在
    /// - 会话创建失败
    /// - 连接重置
    /// - 管道断开
    /// - WebDriver超时
    ///
    /// 同时验证策略错误不应被标记为可恢复
    #[test]
    fn recoverable_error_detection_matches_session_patterns() {
        for message in [
            "invalid session id",
            "No Such Window",
            "session not created",
            "connection reset by peer",
            "broken pipe while writing webdriver command",
            "WebDriver request timed out",
        ] {
            let err = anyhow::anyhow!(message);
            assert!(is_recoverable_rust_native_error(&err), "{message}");
        }

        // 白名单错误不应被视为可恢复（这是策略违规）
        let allowlist_error =
            anyhow::anyhow!("URL host 'localhost' is not in browser allowlist [example.com]");
        assert!(!is_recoverable_rust_native_error(&allowlist_error));
    }

    /// 测试不可恢复错误检测拒绝策略错误
    ///
    /// 验证is_recoverable_rust_native_error函数不会将
    /// 安全策略相关的错误标记为可恢复：
    /// - 安全策略阻止
    /// - 私有主机拒绝
    /// - 动作不可用错误
    ///
    /// 这些错误需要修改配置或代码，不能通过简单重试解决
    #[test]
    fn non_recoverable_error_detection_rejects_policy_errors() {
        for message in [
            "Blocked by security policy",
            "URL host '127.0.0.1' is private and disallowed",
            "Action 'mouse_move' is unavailable for backend 'rust_native'",
        ] {
            let err = anyhow::anyhow!(message);
            assert!(!is_recoverable_rust_native_error(&err), "{message}");
        }
    }

    /// 测试会话重置在没有客户端时是幂等的
    ///
    /// 验证NativeBrowserState的reset_session方法在没有活跃客户端时
    /// 可以安全地多次调用而不会产生错误，
    /// 这对于错误恢复和清理场景很重要
    ///
    /// 此测试仅在browser-native特性启用时运行
    #[cfg(feature = "browser-native")]
    #[test]
    fn reset_session_is_idempotent_without_client() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("current-thread tokio runtime should build for browser test");
        runtime.block_on(async {
            let mut state = native_backend::NativeBrowserState::default();
            state.reset_session().await;
            state.reset_session().await;
        });
    }
}
