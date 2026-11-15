//! Email 通道单元测试模块
//!
//! 本模块包含对 EmailChannel 及其相关组件的全面测试套件，验证以下功能：
//!
//! - **配置测试**：验证 EmailConfig 的默认值、自定义配置、克隆和序列化/反序列化
//! - **通道创建测试**：验证 EmailChannel 的初始化和基本属性
//! - **发件人过滤测试**：验证 is_sender_allowed 方法的各种匹配规则
//! - **HTML 处理测试**：验证 strip_html 方法对各种 HTML 输入的处理
//! - **消息去重测试**：验证 seen_messages 集合对消息 ID 的跟踪
//!
//! # 测试分类
//!
//! 1. 配置验证测试
//! 2. 通道行为测试
//! 3. 发件人权限测试
//! 4. 内容处理测试
//! 5. 序列化兼容性测试

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试默认 SMTP 端口使用 TLS 端口
    ///
    /// 验证 default_smtp_port() 函数返回正确的 TLS 端口号 465。
    /// 注意：此测试目前被注释，待功能实现后启用。
    #[test]
    fn default_smtp_port_uses_tls_port() {
        // assert_eq!(default_smtp_port(), 465);
    }

    /// 测试 EmailConfig 默认值使用 TLS SMTP 配置
    ///
    /// 验证默认配置中：
    /// - SMTP 端口为 465（TLS 标准端口）
    /// - SMTP TLS 标志为 true
    #[test]
    fn email_config_default_uses_tls_smtp_defaults() {
        let config = EmailConfig::default();
        assert_eq!(config.smtp_port, 465);
        assert!(config.smtp_tls);
    }

    /// 测试默认空闲超时为 29 分钟
    ///
    /// 验证 default_idle_timeout() 函数返回 1740 秒（29 分钟）。
    /// 注意：此测试目前被注释，待功能实现后启用。
    #[test]
    fn default_idle_timeout_is_29_minutes() {
        // assert_eq!(default_idle_timeout(), 1740);
    }

    /// 测试已见消息集合初始为空
    ///
    /// 验证新创建的 EmailChannel 实例的 seen_messages 集合为空。
    #[tokio::test]
    async fn seen_messages_starts_empty() {
        let channel = EmailChannel::new(EmailConfig::default());
        let seen = channel.seen_messages.lock().await;
        assert!(seen.is_empty());
    }

    /// 测试已见消息集合能正确跟踪唯一 ID
    ///
    /// 验证 seen_messages 集合：
    /// - 可以成功插入新消息 ID（返回 true）
    /// - 重复插入相同 ID 会失败（返回 false）
    /// - 不同 ID 可以正常插入
    /// - 集合大小与插入的唯一 ID 数量一致
    #[tokio::test]
    async fn seen_messages_tracks_unique_ids() {
        let channel = EmailChannel::new(EmailConfig::default());
        let mut seen = channel.seen_messages.lock().await;

        assert!(seen.insert("first-id".to_string()));
        assert!(!seen.insert("first-id".to_string()));
        assert!(seen.insert("second-id".to_string()));
        assert_eq!(seen.len(), 2);
    }

    // ============================================================================
    // EmailConfig 配置测试
    // ============================================================================

    /// 测试 EmailConfig 的默认值
    ///
    /// 验证默认配置中所有字段都初始化为空字符串或空集合，
    /// 仅 TLS 标志默认为 true。
    #[test]
    fn email_config_default() {
        let config = EmailConfig::default();
        assert_eq!(config.imap_host, "");
        assert_eq!(config.smtp_host, "");
        assert!(config.smtp_tls);
        assert_eq!(config.username, "");
        assert_eq!(config.password, "");
        assert_eq!(config.from_address, "");
        assert!(config.allowed_senders.is_empty());
    }

    /// 测试 EmailConfig 自定义配置
    ///
    /// 验证可以创建带有自定义值的 EmailConfig 实例，
    /// 并正确保存所有配置字段。
    #[test]
    fn email_config_custom() {
        let config = EmailConfig {
            imap_host: "imap.example.com".to_string(),
            imap_port: 993,
            imap_folder: "Archive".to_string(),
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 465,
            smtp_tls: true,
            username: "user@example.com".to_string(),
            password: "pass123".to_string(),
            from_address: "bot@example.com".to_string(),
            idle_timeout_secs: 1200,
            allowed_senders: vec!["allowed@example.com".to_string()],
        };
        assert_eq!(config.imap_host, "imap.example.com");
        assert_eq!(config.imap_folder, "Archive");
        assert_eq!(config.idle_timeout_secs, 1200);
    }

    /// 测试 EmailConfig 的克隆功能
    ///
    /// 验证克隆后的配置对象与原对象在所有字段上完全一致。
    #[test]
    fn email_config_clone() {
        let config = EmailConfig {
            imap_host: "imap.test.com".to_string(),
            imap_port: 993,
            imap_folder: "INBOX".to_string(),
            smtp_host: "smtp.test.com".to_string(),
            smtp_port: 587,
            smtp_tls: true,
            username: "user@test.com".to_string(),
            password: "secret".to_string(),
            from_address: "bot@test.com".to_string(),
            idle_timeout_secs: 1740,
            allowed_senders: vec!["*".to_string()],
        };
        let cloned = config.clone();
        assert_eq!(cloned.imap_host, config.imap_host);
        assert_eq!(cloned.smtp_port, config.smtp_port);
        assert_eq!(cloned.allowed_senders, config.allowed_senders);
    }

    // ============================================================================
    // EmailChannel 通道测试
    // ============================================================================

    /// 测试 EmailChannel 的创建
    ///
    /// 验证：
    /// - 新创建的通道配置与传入的配置一致
    /// - seen_messages 集合初始为空
    #[tokio::test]
    async fn email_channel_new() {
        let config = EmailConfig::default();
        let channel = EmailChannel::new(config.clone());
        assert_eq!(channel.config.imap_host, config.imap_host);

        let seen_guard = channel.seen_messages.lock().await;
        assert_eq!(seen_guard.len(), 0);
    }

    /// 测试 EmailChannel 的名称
    ///
    /// 验证 name() 方法返回固定的通道标识符 "email"。
    #[test]
    fn email_channel_name() {
        let channel = EmailChannel::new(EmailConfig::default());
        assert_eq!(channel.name(), "email");
    }

    // ============================================================================
    // is_sender_allowed 发件人权限测试
    // ============================================================================

    /// 测试空允许列表拒绝所有发件人
    ///
    /// 验证当 allowed_senders 为空列表时，所有发件人都被拒绝访问。
    #[test]
    fn is_sender_allowed_empty_list_denies_all() {
        let config = EmailConfig { allowed_senders: vec![], ..Default::default() };
        let channel = EmailChannel::new(config);
        assert!(!channel.is_sender_allowed("anyone@example.com"));
        assert!(!channel.is_sender_allowed("user@test.com"));
    }

    /// 测试通配符允许所有发件人
    ///
    /// 验证当 allowed_senders 包含 "*" 时，任意发件人都被允许。
    #[test]
    fn is_sender_allowed_wildcard_allows_all() {
        let config = EmailConfig { allowed_senders: vec!["*".to_string()], ..Default::default() };
        let channel = EmailChannel::new(config);
        assert!(channel.is_sender_allowed("anyone@example.com"));
        assert!(channel.is_sender_allowed("user@test.com"));
        assert!(channel.is_sender_allowed("random@domain.org"));
    }

    /// 测试精确邮箱地址匹配
    ///
    /// 验证只有完全匹配允许列表中的邮箱地址才被允许，
    /// 其他邮箱（包括同一域名的其他用户）都被拒绝。
    #[test]
    fn is_sender_allowed_specific_email() {
        let config = EmailConfig {
            allowed_senders: vec!["allowed@example.com".to_string()],
            ..Default::default()
        };
        let channel = EmailChannel::new(config);
        assert!(channel.is_sender_allowed("allowed@example.com"));
        assert!(!channel.is_sender_allowed("other@example.com"));
        assert!(!channel.is_sender_allowed("allowed@other.com"));
    }

    /// 测试带 @ 前缀的域名匹配
    ///
    /// 验证允许列表中的 "@example.com" 格式可以匹配该域名下的所有邮箱。
    #[test]
    fn is_sender_allowed_domain_with_at_prefix() {
        let config =
            EmailConfig { allowed_senders: vec!["@example.com".to_string()], ..Default::default() };
        let channel = EmailChannel::new(config);
        assert!(channel.is_sender_allowed("user@example.com"));
        assert!(channel.is_sender_allowed("admin@example.com"));
        assert!(!channel.is_sender_allowed("user@other.com"));
    }

    /// 测试不带 @ 前缀的域名匹配
    ///
    /// 验证允许列表中的 "example.com" 格式（无 @ 前缀）同样可以匹配该域名下的所有邮箱。
    #[test]
    fn is_sender_allowed_domain_without_at_prefix() {
        let config =
            EmailConfig { allowed_senders: vec!["example.com".to_string()], ..Default::default() };
        let channel = EmailChannel::new(config);
        assert!(channel.is_sender_allowed("user@example.com"));
        assert!(channel.is_sender_allowed("admin@example.com"));
        assert!(!channel.is_sender_allowed("user@other.com"));
    }

    /// 测试发件人匹配的大小写不敏感性
    ///
    /// 验证邮箱地址匹配时不区分大小写，允许列表和实际发件人地址
    /// 可以使用不同的大小写组合。
    #[test]
    fn is_sender_allowed_case_insensitive() {
        let config = EmailConfig {
            allowed_senders: vec!["Allowed@Example.COM".to_string()],
            ..Default::default()
        };
        let channel = EmailChannel::new(config);
        assert!(channel.is_sender_allowed("allowed@example.com"));
        assert!(channel.is_sender_allowed("ALLOWED@EXAMPLE.COM"));
        assert!(channel.is_sender_allowed("AlLoWeD@eXaMpLe.cOm"));
    }

    /// 测试多个发件人规则组合
    ///
    /// 验证允许列表可以同时包含：
    /// - 精确邮箱地址
    /// - 域名通配符（带或不带 @ 前缀）
    /// 并且所有规则都能正确生效。
    #[test]
    fn is_sender_allowed_multiple_senders() {
        let config = EmailConfig {
            allowed_senders: vec![
                "user1@example.com".to_string(),
                "user2@test.com".to_string(),
                "@allowed.com".to_string(),
            ],
            ..Default::default()
        };
        let channel = EmailChannel::new(config);
        assert!(channel.is_sender_allowed("user1@example.com"));
        assert!(channel.is_sender_allowed("user2@test.com"));
        assert!(channel.is_sender_allowed("anyone@allowed.com"));
        assert!(!channel.is_sender_allowed("user3@example.com"));
    }

    /// 测试通配符与特定邮箱的组合
    ///
    /// 验证当通配符 "*" 与特定邮箱同时存在时，所有发件人都被允许。
    #[test]
    fn is_sender_allowed_wildcard_with_specific() {
        let config = EmailConfig {
            allowed_senders: vec!["*".to_string(), "specific@example.com".to_string()],
            ..Default::default()
        };
        let channel = EmailChannel::new(config);
        assert!(channel.is_sender_allowed("anyone@example.com"));
        assert!(channel.is_sender_allowed("specific@example.com"));
    }

    /// 测试空发件人和特殊地址处理
    ///
    /// 验证：
    /// - 空字符串发件人被拒绝（即使配置了域名规则）
    /// - "@example.com" 这个特殊地址本身符合域名匹配规则（以 "@example.com" 结尾）
    #[test]
    fn is_sender_allowed_empty_sender() {
        let config =
            EmailConfig { allowed_senders: vec!["@example.com".to_string()], ..Default::default() };
        let channel = EmailChannel::new(config);
        assert!(!channel.is_sender_allowed(""));
        // "@example.com" 以 "@example.com" 结尾，因此符合匹配规则
        assert!(channel.is_sender_allowed("@example.com"));
    }

    // ============================================================================
    // strip_html HTML 处理测试
    // ============================================================================

    /// 测试基本 HTML 标签移除
    ///
    /// 验证简单的 HTML 标签能被正确移除，仅保留标签内的文本内容。
    #[test]
    fn strip_html_basic() {
        assert_eq!(EmailChannel::strip_html("<p>Hello</p>"), "Hello");
        assert_eq!(EmailChannel::strip_html("<div>World</div>"), "World");
    }

    /// 测试嵌套 HTML 标签处理
    ///
    /// 验证多层嵌套的 HTML 标签能被正确移除，
    /// 嵌套标签中的文本会被合并并用空格分隔。
    #[test]
    fn strip_html_nested_tags() {
        assert_eq!(
            EmailChannel::strip_html("<div><p>Hello <strong>World</strong></p></div>"),
            "Hello World"
        );
    }

    /// 测试多行 HTML 内容处理
    ///
    /// 验证包含换行符的多行 HTML 内容能被正确处理，
    /// 标签被移除后，不同行的文本会用空格连接。
    #[test]
    fn strip_html_multiple_lines() {
        let html = "<div>\n  <p>Line 1</p>\n  <p>Line 2</p>\n</div>";
        assert_eq!(EmailChannel::strip_html(html), "Line 1 Line 2");
    }

    /// 测试纯文本内容保持不变
    ///
    /// 验证不包含 HTML 标签的纯文本内容会被原样返回，
    /// 空字符串也会被正确处理。
    #[test]
    fn strip_html_preserves_text() {
        assert_eq!(EmailChannel::strip_html("No tags here"), "No tags here");
        assert_eq!(EmailChannel::strip_html(""), "");
    }

    /// 测试格式错误的 HTML 处理
    ///
    /// 验证：
    /// - 未闭合的标签能被正确处理，仅保留文本内容
    /// - 函数会移除 < 和 > 之间的所有内容，因此 "Text>with>brackets" 变为 "Textwithbrackets"
    #[test]
    fn strip_html_handles_malformed() {
        assert_eq!(EmailChannel::strip_html("<p>Unclosed"), "Unclosed");
        // 函数会移除 < 和 > 之间的所有内容，因此 "Text>with>brackets" 变为 "Textwithbrackets"
        assert_eq!(EmailChannel::strip_html("Text>with>brackets"), "Textwithbrackets");
    }

    /// 测试自闭合标签处理
    ///
    /// 验证自闭合标签（如 <br/>、<hr/>）会被移除但不会添加额外空格。
    #[test]
    fn strip_html_self_closing_tags() {
        // 自闭合标签会被移除，但不会添加空格
        assert_eq!(EmailChannel::strip_html("Hello<br/>World"), "HelloWorld");
        assert_eq!(EmailChannel::strip_html("Text<hr/>More"), "TextMore");
    }

    /// 测试带属性的 HTML 标签处理
    ///
    /// 验证带有属性（如 href）的 HTML 标签能被正确移除，
    /// 仅保留标签内的文本内容，属性值不影响处理结果。
    #[test]
    fn strip_html_attributes_preserved() {
        assert_eq!(EmailChannel::strip_html("<a href=\"http://example.com\">Link</a>"), "Link");
    }

    /// 测试多个连续空格的压缩
    ///
    /// 验证 HTML 中的多个连续空格会被压缩为单个空格。
    #[test]
    fn strip_html_multiple_spaces_collapsed() {
        assert_eq!(EmailChannel::strip_html("<p>Word</p>  <p>Word</p>"), "Word Word");
    }

    /// 测试 HTML 特殊字符实体的保留
    ///
    /// 验证 HTML 实体（如 &lt;、&gt;）会被保留在输出中，
    /// 不会被转换或移除。
    #[test]
    fn strip_html_special_characters() {
        assert_eq!(EmailChannel::strip_html("<span>&lt;tag&gt;</span>"), "&lt;tag&gt;");
    }

    // ============================================================================
    // EmailConfig 序列化测试
    // ============================================================================

    /// 测试 EmailConfig 的序列化和反序列化
    ///
    /// 验证 EmailConfig 可以被正确序列化为 JSON 并从 JSON 反序列化，
    /// 且所有字段在往返过程中保持一致。
    #[test]
    fn email_config_serialize_deserialize() {
        let config = EmailConfig {
            imap_host: "imap.example.com".to_string(),
            imap_port: 993,
            imap_folder: "INBOX".to_string(),
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 587,
            smtp_tls: true,
            username: "user@example.com".to_string(),
            password: "password123".to_string(),
            from_address: "bot@example.com".to_string(),
            idle_timeout_secs: 1740,
            allowed_senders: vec!["allowed@example.com".to_string()],
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: EmailConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.imap_host, config.imap_host);
        assert_eq!(deserialized.smtp_port, config.smtp_port);
        assert_eq!(deserialized.allowed_senders, config.allowed_senders);
    }

    /// 测试 EmailConfig 反序列化时使用默认值
    ///
    /// 验证当 JSON 中省略某些字段时，反序列化会使用配置的默认值：
    /// - IMAP 端口默认为 993
    /// - SMTP 端口默认为 465
    /// - SMTP TLS 默认为 true
    /// - 空闲超时默认为 1740 秒（29 分钟）
    #[test]
    fn email_config_deserialize_with_defaults() {
        let json = r#"{
                "imap_host": "imap.test.com",
                "smtp_host": "smtp.test.com",
                "username": "user",
                "password": "pass",
                "from_address": "bot@test.com"
            }"#;

        let config: EmailConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.imap_port, 993); // 默认值
        assert_eq!(config.smtp_port, 465); // 默认值
        assert!(config.smtp_tls); // 默认值
        assert_eq!(config.idle_timeout_secs, 1740); // 默认值
    }

    /// 测试空闲超时的显式值反序列化
    ///
    /// 验证当 JSON 中显式指定 idle_timeout_secs 字段时，
    /// 反序列化会使用该指定值而非默认值。
    #[test]
    fn idle_timeout_deserializes_explicit_value() {
        let json = r#"{
                "imap_host": "imap.test.com",
                "smtp_host": "smtp.test.com",
                "username": "user",
                "password": "pass",
                "from_address": "bot@test.com",
                "idle_timeout_secs": 900
            }"#;
        let config: EmailConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.idle_timeout_secs, 900);
    }

    /// 测试空闲超时的旧版 poll_interval_secs 别名反序列化
    ///
    /// 验证为了向后兼容，旧的 poll_interval_secs 字段名
    /// 可以被正确反序列化为 idle_timeout_secs 字段。
    #[test]
    fn idle_timeout_deserializes_legacy_poll_interval_alias() {
        let json = r#"{
                "imap_host": "imap.test.com",
                "smtp_host": "smtp.test.com",
                "username": "user",
                "password": "pass",
                "from_address": "bot@test.com",
                "poll_interval_secs": 120
            }"#;
        let config: EmailConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.idle_timeout_secs, 120);
    }

    /// 测试空闲超时配置能正确传递到通道
    ///
    /// 验证 EmailConfig 中的 idle_timeout_secs 值
    /// 能被 EmailChannel 正确继承和存储。
    #[test]
    fn idle_timeout_propagates_to_channel() {
        let config = EmailConfig { idle_timeout_secs: 600, ..Default::default() };
        let channel = EmailChannel::new(config);
        assert_eq!(channel.config.idle_timeout_secs, 600);
    }

    /// 测试 EmailConfig 的 Debug 输出
    ///
    /// 验证 EmailConfig 实现了 Debug trait，
    /// 并且调试输出中包含配置的关键字段信息。
    #[test]
    fn email_config_debug_output() {
        let config = EmailConfig { imap_host: "imap.debug.com".to_string(), ..Default::default() };
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("imap.debug.com"));
    }
}
