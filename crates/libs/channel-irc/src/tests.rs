//! IRC 通道单元测试模块
//!
//! 本模块包含 IRC 通道实现的所有单元测试用例，覆盖以下功能：
//!
//! - IRC 消息解析：测试 PRIVMSG、PING、数字回复等各种 IRC 协议消息的解析
//! - SASL 认证：测试 SASL PLAIN 编码功能
//! - 消息分割：测试长消息按字节边界分割的逻辑
//! - 用户白名单：测试用户访问控制逻辑
//! - 构造函数：测试 IrcChannel 实例化与默认值处理
//! - 配置序列化：测试配置结构的 TOML/JSON 序列化与反序列化

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    // ── IRC 消息解析测试 ──────────────────────────────────

    /// 测试解析带有完整前缀的 PRIVMSG 消息
    ///
    /// 验证解析器能够正确提取：
    /// - 前缀（nick!user@host 格式）
    /// - 命令（PRIVMSG）
    /// - 参数列表（频道名和消息内容）
    #[test]
    fn parse_privmsg_with_prefix() {
        let msg = IrcMessage::parse(":nick!user@host PRIVMSG #channel :Hello world").unwrap();
        assert_eq!(msg.prefix.as_deref(), Some("nick!user@host"));
        assert_eq!(msg.command, "PRIVMSG");
        assert_eq!(msg.params, vec!["#channel", "Hello world"]);
    }

    /// 测试解析私聊（Direct Message）PRIVMSG 消息
    ///
    /// 验证解析器能够正确处理直接发送给机器人的私聊消息，
    /// 包括提取发送者昵称和消息内容。
    #[test]
    fn parse_privmsg_dm() {
        let msg = IrcMessage::parse(":alice!a@host PRIVMSG botname :hi there").unwrap();
        assert_eq!(msg.command, "PRIVMSG");
        assert_eq!(msg.params, vec!["botname", "hi there"]);
        assert_eq!(msg.nick(), Some("alice"));
    }

    /// 测试解析 PING 消息
    ///
    /// 验证解析器能够正确解析服务器发送的 PING 命令，
    /// 该命令用于保持连接活跃，需要客户端回复 PONG。
    #[test]
    fn parse_ping() {
        let msg = IrcMessage::parse("PING :server.example.com").unwrap();
        assert!(msg.prefix.is_none());
        assert_eq!(msg.command, "PING");
        assert_eq!(msg.params, vec!["server.example.com"]);
    }

    /// 测试解析数字回复消息
    ///
    /// 验证解析器能够正确处理 IRC 协议的数字回复码（如 001 欢迎消息），
    /// 这些数字码用于表示各种服务器响应状态。
    #[test]
    fn parse_numeric_reply() {
        let msg = IrcMessage::parse(":server 001 botname :Welcome to the IRC network").unwrap();
        assert_eq!(msg.prefix.as_deref(), Some("server"));
        assert_eq!(msg.command, "001");
        assert_eq!(msg.params, vec!["botname", "Welcome to the IRC network"]);
    }

    /// 测试解析不带尾部参数的消息
    ///
    /// 验证解析器能够正确处理只有中间参数、没有以冒号开头的尾部参数的消息。
    #[test]
    fn parse_no_trailing() {
        let msg = IrcMessage::parse(":server 433 * botname").unwrap();
        assert_eq!(msg.command, "433");
        assert_eq!(msg.params, vec!["*", "botname"]);
    }

    /// 测试解析 CAP ACK 消息
    ///
    /// 验证解析器能够正确处理 IRC 能力协商（CAP）的确认消息，
    /// 该机制用于客户端与服务器协商支持的功能扩展。
    #[test]
    fn parse_cap_ack() {
        let msg = IrcMessage::parse(":server CAP * ACK :sasl").unwrap();
        assert_eq!(msg.command, "CAP");
        assert_eq!(msg.params, vec!["*", "ACK", "sasl"]);
    }

    /// 测试解析空行返回 None
    ///
    /// 验证解析器能够正确处理空字符串和仅包含换行符的输入，
    /// 这些情况应该返回 None 而不是产生解析错误。
    #[test]
    fn parse_empty_line_returns_none() {
        assert!(IrcMessage::parse("").is_none());
        assert!(IrcMessage::parse("\r\n").is_none());
    }

    /// 测试解析器移除 CRLF 行尾符
    ///
    /// 验证解析器能够正确移除消息末尾的回车换行符（\r\n），
    /// IRC 协议标准使用 CRLF 作为消息终止符。
    #[test]
    fn parse_strips_crlf() {
        let msg = IrcMessage::parse("PING :test\r\n").unwrap();
        assert_eq!(msg.params, vec!["test"]);
    }

    /// 测试命令自动转换为大写
    ///
    /// 验证解析器能够将小写命令自动转换为大写，
    /// 符合 IRC 协议命令不区分大小写的规范。
    #[test]
    fn parse_command_uppercase() {
        let msg = IrcMessage::parse("ping :test").unwrap();
        assert_eq!(msg.command, "PING");
    }

    /// 测试从完整前缀提取昵称
    ///
    /// 验证 nick() 方法能够从标准的 nick!user@host 格式前缀中正确提取昵称部分。
    #[test]
    fn nick_extraction_full_prefix() {
        let msg = IrcMessage::parse(":nick!user@host PRIVMSG #ch :msg").unwrap();
        assert_eq!(msg.nick(), Some("nick"));
    }

    /// 测试从仅包含昵称的前缀提取
    ///
    /// 验证 nick() 方法能够处理不包含用户名和主机名、仅包含昵称的简化前缀格式。
    #[test]
    fn nick_extraction_nick_only() {
        let msg = IrcMessage::parse(":server 001 bot :Welcome").unwrap();
        assert_eq!(msg.nick(), Some("server"));
    }

    /// 测试无前缀消息的昵称提取
    ///
    /// 验证当消息不包含前缀时，nick() 方法正确返回 None。
    #[test]
    fn nick_extraction_no_prefix() {
        let msg = IrcMessage::parse("PING :token").unwrap();
        assert_eq!(msg.nick(), None);
    }

    /// 测试解析 AUTHENTICATE + 消息
    ///
    /// 验证解析器能够正确处理 SASL 认证流程中的 AUTHENTICATE 命令，
    /// "+" 表示客户端应发送空的初始响应。
    #[test]
    fn parse_authenticate_plus() {
        let msg = IrcMessage::parse("AUTHENTICATE +").unwrap();
        assert_eq!(msg.command, "AUTHENTICATE");
        assert_eq!(msg.params, vec!["+"]);
    }

    // ── SASL PLAIN 编码测试 ─────────────────────────────────

    /// 测试 SASL PLAIN 机制编码
    ///
    /// 验证 encode_sasl_plain 函数能够正确地将用户名和密码编码为
    /// SASL PLAIN 格式（\0username\0password 的 Base64 编码）。
    #[test]
    fn sasl_plain_encode() {
        let encoded = encode_sasl_plain("jilles", "sesame");
        // \0jilles\0sesame → base64
        assert_eq!(encoded, "AGppbGxlcwBzZXNhbWU=");
    }

    /// 测试 SASL PLAIN 编码空密码情况
    ///
    /// 验证当密码为空字符串时，编码函数仍能正确工作，
    /// 生成 \0username\0 的 Base64 编码。
    #[test]
    fn sasl_plain_empty_password() {
        let encoded = encode_sasl_plain("nick", "");
        // \0nick\0 → base64
        assert_eq!(encoded, "AG5pY2sA");
    }

    // ── 消息分割测试 ───────────────────────────────────

    /// 测试短消息不分割
    ///
    /// 验证当消息长度小于限制时，split_message 返回包含原始消息的单元素列表。
    #[test]
    fn split_short_message() {
        let chunks = split_message("hello", 400);
        assert_eq!(chunks, vec!["hello"]);
    }

    /// 测试长消息分割
    ///
    /// 验证当消息长度超过限制时，split_message 能够正确将其分割为多个块，
    /// 每个块不超过指定的字节限制。
    #[test]
    fn split_long_message() {
        let msg = "a".repeat(800);
        let chunks = split_message(&msg, 400);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 400);
        assert_eq!(chunks[1].len(), 400);
    }

    /// 测试恰好达到边界长度的消息
    ///
    /// 验证当消息长度恰好等于限制时，不需要分割，返回单元素列表。
    #[test]
    fn split_exact_boundary() {
        let msg = "a".repeat(400);
        let chunks = split_message(&msg, 400);
        assert_eq!(chunks.len(), 1);
    }

    /// 测试 Unicode 字符边界安全分割
    ///
    /// 验证分割函数能够正确处理 UTF-8 多字节字符，不会在字符中间截断。
    /// 'é' 在 UTF-8 中占 2 字节，分割时必须在字符边界处进行。
    #[test]
    fn split_unicode_safe() {
        // 'é' 在 UTF-8 中占 2 字节；在第 3 字节处分割会截断字符
        let msg = "ééé"; // 6 字节
        let chunks = split_message(msg, 3);
        // 应在字符边界（2 字节）处分割，而非字符中间
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], "é");
        assert_eq!(chunks[1], "é");
        assert_eq!(chunks[2], "é");
    }

    /// 测试空消息分割
    ///
    /// 验证空字符串输入返回包含一个空字符串的列表。
    #[test]
    fn split_empty_message() {
        let chunks = split_message("", 400);
        assert_eq!(chunks, vec![""]);
    }

    /// 测试将换行符分割为独立行
    ///
    /// 验证 split_message 能够识别换行符并将多行文本分割为独立的行。
    #[test]
    fn split_newlines_into_separate_lines() {
        let chunks = split_message("line one\nline two\nline three", 400);
        assert_eq!(chunks, vec!["line one", "line two", "line three"]);
    }

    /// 测试处理 CRLF 换行符
    ///
    /// 验证函数能够正确处理 Windows 风格的回车换行符（\r\n）。
    #[test]
    fn split_crlf_newlines() {
        let chunks = split_message("hello\r\nworld", 400);
        assert_eq!(chunks, vec!["hello", "world"]);
    }

    /// 测试跳过空行
    ///
    /// 验证连续的空行会被跳过，不会在结果中产生空字符串元素。
    #[test]
    fn split_skips_empty_lines() {
        let chunks = split_message("hello\n\n\nworld", 400);
        assert_eq!(chunks, vec!["hello", "world"]);
    }

    /// 测试处理尾部换行符
    ///
    /// 验证消息末尾的换行符不会产生额外的空字符串元素。
    #[test]
    fn split_trailing_newline() {
        let chunks = split_message("hello\n", 400);
        assert_eq!(chunks, vec!["hello"]);
    }

    /// 测试多行消息中包含长行的情况
    ///
    /// 验证函数能够同时处理换行符分割和长行分割的混合场景。
    #[test]
    fn split_multiline_with_long_line() {
        let long = "a".repeat(800);
        let msg = format!("short\n{long}\nend");
        let chunks = split_message(&msg, 400);
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0], "short");
        assert_eq!(chunks[1].len(), 400);
        assert_eq!(chunks[2].len(), 400);
        assert_eq!(chunks[3], "end");
    }

    /// 测试仅包含换行符的消息
    ///
    /// 验证仅由换行符组成的输入返回包含一个空字符串的列表。
    #[test]
    fn split_only_newlines() {
        let chunks = split_message("\n\n\n", 400);
        assert_eq!(chunks, vec![""]);
    }

    // ── 用户白名单测试 ───────────────────────────────────────────

    /// 测试通配符允许所有用户
    ///
    /// 验证当 allowed_users 包含 "*" 通配符时，
    /// 任何用户都被允许与机器人交互。
    #[test]
    fn wildcard_allows_anyone() {
        let ch = make_channel();
        // 默认 make_channel 包含通配符
        assert!(ch.is_user_allowed("anyone"));
        assert!(ch.is_user_allowed("stranger"));
    }

    /// 测试指定用户白名单访问控制
    ///
    /// 验证只有白名单中明确列出的用户才被允许，
    /// 未在列表中的用户应被拒绝访问。
    #[test]
    fn specific_user_allowed() {
        let ch = IrcChannel::new(IrcChannelConfig {
            server: "irc.test".into(),
            port: 6697,
            nickname: "bot".into(),
            username: None,
            channels: vec![],
            allowed_users: vec!["alice".into(), "bob".into()],
            server_password: None,
            nickserv_password: None,
            sasl_password: None,
            verify_tls: true,
        });
        assert!(ch.is_user_allowed("alice"));
        assert!(ch.is_user_allowed("bob"));
        assert!(!ch.is_user_allowed("eve"));
    }

    /// 测试白名单大小写不敏感
    ///
    /// 验证用户名白名单匹配是大小写不敏感的，
    /// 符合 IRC 协议中昵称大小写不敏感的惯例。
    #[test]
    fn allowlist_case_insensitive() {
        let ch = IrcChannel::new(IrcChannelConfig {
            server: "irc.test".into(),
            port: 6697,
            nickname: "bot".into(),
            username: None,
            channels: vec![],
            allowed_users: vec!["Alice".into()],
            server_password: None,
            nickserv_password: None,
            sasl_password: None,
            verify_tls: true,
        });
        assert!(ch.is_user_allowed("alice"));
        assert!(ch.is_user_allowed("ALICE"));
        assert!(ch.is_user_allowed("Alice"));
    }

    /// 测试空白名单拒绝所有用户
    ///
    /// 验证当 allowed_users 为空列表时，所有用户都被拒绝访问。
    /// 这是默认拒绝的安全策略。
    #[test]
    fn empty_allowlist_denies_all() {
        let ch = IrcChannel::new(IrcChannelConfig {
            server: "irc.test".into(),
            port: 6697,
            nickname: "bot".into(),
            username: None,
            channels: vec![],
            allowed_users: vec![],
            server_password: None,
            nickserv_password: None,
            sasl_password: None,
            verify_tls: true,
        });
        assert!(!ch.is_user_allowed("anyone"));
    }

    // ── 构造函数测试 ─────────────────────────────────────────

    /// 测试构造函数默认 username 为 nickname
    ///
    /// 验证当配置中未显式指定 username 时，
    /// IrcChannel 构造函数会自动使用 nickname 作为 username。
    #[test]
    fn new_defaults_username_to_nickname() {
        let ch = IrcChannel::new(IrcChannelConfig {
            server: "irc.test".into(),
            port: 6697,
            nickname: "mybot".into(),
            username: None,
            channels: vec![],
            allowed_users: vec![],
            server_password: None,
            nickserv_password: None,
            sasl_password: None,
            verify_tls: true,
        });
        assert_eq!(ch.username, "mybot");
    }

    /// 测试构造函数使用显式指定的 username
    ///
    /// 验证当配置中显式指定 username 时，
    /// 构造函数会使用该值而非 nickname。
    #[test]
    fn new_uses_explicit_username() {
        let ch = IrcChannel::new(IrcChannelConfig {
            server: "irc.test".into(),
            port: 6697,
            nickname: "mybot".into(),
            username: Some("customuser".into()),
            channels: vec![],
            allowed_users: vec![],
            server_password: None,
            nickserv_password: None,
            sasl_password: None,
            verify_tls: true,
        });
        assert_eq!(ch.username, "customuser");
        assert_eq!(ch.nickname, "mybot");
    }

    /// 测试 name() 方法返回 "irc"
    ///
    /// 验证 IrcChannel 实现的 Channel trait 的 name() 方法正确返回通道类型标识符。
    #[test]
    fn name_returns_irc() {
        let ch = make_channel();
        assert_eq!(ch.name(), "irc");
    }

    /// 测试构造函数存储所有字段
    ///
    /// 验证 IrcChannel::new 能够正确存储配置中的所有字段，
    /// 包括服务器地址、端口、认证信息、通道列表等。
    #[test]
    fn new_stores_all_fields() {
        let ch = IrcChannel::new(IrcChannelConfig {
            server: "irc.example.com".into(),
            port: 6697,
            nickname: "zcbot".into(),
            username: Some("vibewindow".into()),
            channels: vec!["#test".into()],
            allowed_users: vec!["alice".into()],
            server_password: Some("serverpass".into()),
            nickserv_password: Some("nspass".into()),
            sasl_password: Some("saslpass".into()),
            verify_tls: false,
        });
        assert_eq!(ch.server, "irc.example.com");
        assert_eq!(ch.port, 6697);
        assert_eq!(ch.nickname, "zcbot");
        assert_eq!(ch.username, "vibewindow");
        assert_eq!(ch.channels, vec!["#test"]);
        assert_eq!(ch.allowed_users, vec!["alice"]);
        assert_eq!(ch.server_password.as_deref(), Some("serverpass"));
        assert_eq!(ch.nickserv_password.as_deref(), Some("nspass"));
        assert_eq!(ch.sasl_password.as_deref(), Some("saslpass"));
        assert!(!ch.verify_tls);
    }

    // ── 配置序列化测试 ────────────────────────────────────────

    /// 测试 IRC 配置 TOML 序列化往返
    ///
    /// 验证 IrcConfig 结构体能够正确序列化为 TOML 格式，
    /// 并能从 TOML 字符串正确反序列化，保持数据完整性。
    #[test]
    fn irc_config_serde_roundtrip() {
        use config_channel_services::IrcConfig;

        let config = IrcConfig {
            server: "irc.example.com".into(),
            port: 6697,
            nickname: "zcbot".into(),
            username: Some("vibewindow".into()),
            channels: vec!["#test".into(), "#dev".into()],
            allowed_users: vec!["alice".into()],
            server_password: None,
            nickserv_password: Some("secret".into()),
            sasl_password: None,
            verify_tls: Some(true),
        };

        let toml_str = toml::to_string(&config).unwrap();
        let parsed: IrcConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.server, "irc.example.com");
        assert_eq!(parsed.port, 6697);
        assert_eq!(parsed.nickname, "zcbot");
        assert_eq!(parsed.username.as_deref(), Some("vibewindow"));
        assert_eq!(parsed.channels, vec!["#test", "#dev"]);
        assert_eq!(parsed.allowed_users, vec!["alice"]);
        assert!(parsed.server_password.is_none());
        assert_eq!(parsed.nickserv_password.as_deref(), Some("secret"));
        assert!(parsed.sasl_password.is_none());
        assert_eq!(parsed.verify_tls, Some(true));
    }

    /// 测试最小化 TOML 配置解析
    ///
    /// 验证仅包含必需字段（server 和 nickname）的 TOML 配置能够被正确解析，
    /// 缺失的可选字段应使用默认值。
    #[test]
    fn irc_config_minimal_toml() {
        use config_channel_services::IrcConfig;

        let toml_str = r#"
    server = "irc.example.com"
    nickname = "bot"
    "#;
        let parsed: IrcConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(parsed.server, "irc.example.com");
        assert_eq!(parsed.port, 6697); // default
        assert_eq!(parsed.nickname, "bot");
        assert!(parsed.username.is_none());
        assert!(parsed.channels.is_empty());
        assert!(parsed.allowed_users.is_empty());
        assert!(parsed.server_password.is_none());
        assert!(parsed.nickserv_password.is_none());
        assert!(parsed.sasl_password.is_none());
        assert!(parsed.verify_tls.is_none());
    }

    /// 测试 JSON 配置默认端口
    ///
    /// 验证从 JSON 解析配置时，未指定的 port 字段使用默认值 6697（IRC over TLS）。
    #[test]
    fn irc_config_default_port() {
        use config_channel_services::IrcConfig;

        let json = r#"{"server":"irc.test","nickname":"bot"}"#;
        let parsed: IrcConfig = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.port, 6697);
    }

    // ── 辅助函数 ─────────────────────────────────────────────

    /// 创建用于测试的 IrcChannel 实例
    ///
    /// 构造一个包含标准配置的 IrcChannel 实例，用于简化测试用例编写。
    /// 默认配置：
    /// - 服务器：irc.example.com:6697
    /// - 昵称：zcbot
    /// - 通道：#vibewindow
    /// - 允许用户：*（通配符，允许所有）
    /// - TLS 验证：启用
    ///
    /// # 返回值
    ///
    /// 返回配置好的 IrcChannel 实例
    fn make_channel() -> IrcChannel {
        IrcChannel::new(IrcChannelConfig {
            server: "irc.example.com".into(),
            port: 6697,
            nickname: "zcbot".into(),
            username: None,
            channels: vec!["#vibewindow".into()],
            allowed_users: vec!["*".into()],
            server_password: None,
            nickserv_password: None,
            sasl_password: None,
            verify_tls: true,
        })
    }
}
