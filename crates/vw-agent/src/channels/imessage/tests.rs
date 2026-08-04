//! iMessage 频道模块的单元测试集
//!
//! 本模块包含 iMessage 频道的全面测试用例，覆盖以下关键领域：
//!
//! # 测试分类
//!
//! 1. **联系人权限测试**：验证允许列表、通配符和访问控制逻辑
//! 2. **AppleScript 转义测试**：防止命令注入攻击（CWE-78）
//! 3. **目标验证测试**：验证电话号码和邮箱格式的合法性
//! 4. **数据库操作测试**：防止 SQL 注入攻击（CWE-89）
//!
//! # 安全考虑
//!
//! 本测试集重点关注两个关键安全漏洞：
//! - **CWE-78**：OS 命令注入（通过 AppleScript 转义）
//! - **CWE-89**：SQL 注入（通过参数化查询）
//!
//! # 使用方式
//!
//! 运行所有测试：
//! ```bash
//! cargo test --package vibe-agent --lib agent::channels::imessage::tests
//! ```

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试带联系人的频道创建
    ///
    /// 验证使用联系人列表创建 iMessage 频道时：
    /// - 允许的联系人列表正确初始化
    /// - 轮询间隔默认为 3 秒
    #[test]
    fn creates_with_contacts() {
        let ch = IMessageChannel::new(vec!["+1234567890".into()]);
        assert_eq!(ch.allowed_contacts.len(), 1);
        assert_eq!(ch.poll_interval_secs, 3);
    }

    /// 测试空联系人列表的频道创建
    ///
    /// 验证使用空列表创建频道时，允许联系人列表为空
    #[test]
    fn creates_with_empty_contacts() {
        let ch = IMessageChannel::new(vec![]);
        assert!(ch.allowed_contacts.is_empty());
    }

    /// 测试通配符允许任何人发送消息
    ///
    /// 当允许列表包含 "*" 时，所有联系人都应该被允许：
    /// - 电话号码（如 "+1234567890"）
    /// - 邮箱地址（如 "random@icloud.com"）
    /// - 空字符串
    #[test]
    fn wildcard_allows_anyone() {
        let ch = IMessageChannel::new(vec!["*".into()]);
        assert!(ch.is_contact_allowed("+1234567890"));
        assert!(ch.is_contact_allowed("random@icloud.com"));
        assert!(ch.is_contact_allowed(""));
    }

    /// 测试特定联系人的权限验证
    ///
    /// 验证只有在允许列表中的联系人才能通过验证
    #[test]
    fn specific_contact_allowed() {
        let ch = IMessageChannel::new(vec!["+1234567890".into(), "user@icloud.com".into()]);
        assert!(ch.is_contact_allowed("+1234567890"));
        assert!(ch.is_contact_allowed("user@icloud.com"));
    }

    /// 测试未知联系人被拒绝
    ///
    /// 验证不在允许列表中的联系人无法通过权限检查
    #[test]
    fn unknown_contact_denied() {
        let ch = IMessageChannel::new(vec!["+1234567890".into()]);
        assert!(!ch.is_contact_allowed("+9999999999"));
        assert!(!ch.is_contact_allowed("hacker@evil.com"));
    }

    /// 测试联系人验证的大小写不敏感性
    ///
    /// 邮箱地址的验证应该忽略大小写差异
    #[test]
    fn contact_case_insensitive() {
        let ch = IMessageChannel::new(vec!["User@iCloud.com".into()]);
        assert!(ch.is_contact_allowed("user@icloud.com"));
        assert!(ch.is_contact_allowed("USER@ICLOUD.COM"));
    }

    /// 测试空允许列表拒绝所有联系人
    ///
    /// 当允许列表为空时，所有联系人都应该被拒绝
    #[test]
    fn empty_allowlist_denies_all() {
        let ch = IMessageChannel::new(vec![]);
        assert!(!ch.is_contact_allowed("+1234567890"));
        assert!(!ch.is_contact_allowed("anyone"));
    }

    /// 测试频道名称返回 "imessage"
    ///
    /// 验证 name() 方法返回正确的标识符
    #[test]
    fn name_returns_imessage() {
        let ch = IMessageChannel::new(vec![]);
        assert_eq!(ch.name(), "imessage");
    }

    /// 测试通配符与其他联系人共存时的行为
    ///
    /// 当允许列表同时包含通配符和具体联系人时，
    /// 仍然应该允许所有联系人（通配符优先）
    #[test]
    fn wildcard_among_others_still_allows_all() {
        let ch = IMessageChannel::new(vec!["+111".into(), "*".into(), "+222".into()]);
        assert!(ch.is_contact_allowed("totally-unknown"));
    }

    /// 测试包含空格的联系人需要精确匹配
    ///
    /// 验证联系人验证是精确匹配，包括空格
    #[test]
    fn contact_with_spaces_exact_match() {
        let ch = IMessageChannel::new(vec!["  spaced  ".into()]);
        assert!(ch.is_contact_allowed("  spaced  "));
        assert!(!ch.is_contact_allowed("spaced"));
    }

    // ══════════════════════════════════════════════════════════
    // AppleScript 转义测试（CWE-78 命令注入防护）
    // ══════════════════════════════════════════════════════════

    /// 测试双引号的 AppleScript 转义
    ///
    /// 验证双引号字符被正确转义为 \"
    #[test]
    fn escape_applescript_double_quotes() {
        assert_eq!(escape_applescript(r#"hello "world""#), r#"hello \"world\""#);
    }

    /// 测试反斜杠的 AppleScript 转义
    ///
    /// 验证反斜杠字符被正确转义为 \\
    #[test]
    fn escape_applescript_backslashes() {
        assert_eq!(escape_applescript(r"path\to\file"), r"path\\to\\file");
    }

    /// 测试混合特殊字符的 AppleScript 转义
    ///
    /// 验证同时包含双引号和反斜杠的字符串能正确处理
    #[test]
    fn escape_applescript_mixed() {
        assert_eq!(escape_applescript(r#"say "hello\" world"#), r#"say \"hello\\\" world"#);
    }

    /// 测试 AppleScript 注入攻击防护
    ///
    /// 这是安全报告中提到的确切攻击向量。验证：
    /// - 恶意引号被转义，无法逃逸 AppleScript 字符串上下文
    /// - 所有引号字符都被反斜杠转义
    /// - 不存在未转义的引号
    #[test]
    fn escape_applescript_injection_attempt() {
        // 这是安全报告中提到的确切攻击向量
        let malicious = r#"" & do shell script "id" & ""#;
        let escaped = escape_applescript(malicious);
        // 转义后，引号应该被转义，不会逃逸出字符串上下文
        assert_eq!(escaped, r#"\" & do shell script \"id\" & \""#);
        // 验证所有引号都被转义（前面有反斜杠）
        // 转义后的字符串不应该有任何未转义的引号（引号前面没有反斜杠）
        let chars: Vec<char> = escaped.chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            if c == '"' {
                // 每个引号前面必须有反斜杠
                assert!(i > 0 && chars[i - 1] == '\\', "Found unescaped quote at position {i}");
            }
        }
    }

    /// 测试空字符串的 AppleScript 转义
    ///
    /// 空字符串应该保持不变
    #[test]
    fn escape_applescript_empty_string() {
        assert_eq!(escape_applescript(""), "");
    }

    /// 测试无特殊字符字符串的 AppleScript 转义
    ///
    /// 不包含特殊字符的字符串应该保持不变
    #[test]
    fn escape_applescript_no_special_chars() {
        assert_eq!(escape_applescript("hello world"), "hello world");
    }

    /// 测试 Unicode 字符的 AppleScript 转义
    ///
    /// Unicode 字符（如 emoji）应该保持不变
    #[test]
    fn escape_applescript_unicode() {
        assert_eq!(escape_applescript("hello world"), "hello world");
    }

    /// 测试换行符的 AppleScript 转义
    ///
    /// 各种换行符应该被转义：
    /// - \n (LF) -> \\n
    /// - \r (CR) -> \\r
    /// - \r\n (CRLF) -> \\r\\n
    #[test]
    fn escape_applescript_newlines_escaped() {
        assert_eq!(escape_applescript("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_applescript("line1\rline2"), "line1\\rline2");
        assert_eq!(escape_applescript("line1\r\nline2"), "line1\\r\\nline2");
    }

    // ══════════════════════════════════════════════════════════
    // 目标验证测试
    // ══════════════════════════════════════════════════════════

    /// 测试简单电话号码的验证
    ///
    /// 验证以 + 开头的数字字符串被识别为有效电话号码
    #[test]
    fn valid_phone_number_simple() {
        assert!(is_valid_imessage_target("+1234567890"));
    }

    /// 测试带国家代码的电话号码验证
    ///
    /// 验证完整格式的国际电话号码
    #[test]
    fn valid_phone_number_with_country_code() {
        assert!(is_valid_imessage_target("+14155551234"));
    }

    /// 测试带空格的电话号码验证
    ///
    /// 验证包含空格的电话号码格式
    #[test]
    fn valid_phone_number_with_spaces() {
        assert!(is_valid_imessage_target("+1 415 555 1234"));
    }

    /// 测试带连字符的电话号码验证
    ///
    /// 验证包含连字符的电话号码格式
    #[test]
    fn valid_phone_number_with_dashes() {
        assert!(is_valid_imessage_target("+1-415-555-1234"));
    }

    /// 测试国际电话号码验证
    ///
    /// 验证不同国家的电话号码格式：
    /// - 英国：+44
    /// - 日本：+81
    #[test]
    fn valid_phone_number_international() {
        assert!(is_valid_imessage_target("+447911123456")); // 英国
        assert!(is_valid_imessage_target("+81312345678")); // 日本
    }

    /// 测试简单邮箱地址的验证
    ///
    /// 验证标准格式的邮箱地址
    #[test]
    fn valid_email_simple() {
        assert!(is_valid_imessage_target("user@example.com"));
    }

    /// 测试带子域名的邮箱地址验证
    ///
    /// 验证包含子域名的邮箱地址
    #[test]
    fn valid_email_with_subdomain() {
        assert!(is_valid_imessage_target("user@mail.example.com"));
    }

    /// 测试带加号的邮箱地址验证
    ///
    /// 验证本地部分包含 + 标签的邮箱地址
    #[test]
    fn valid_email_with_plus() {
        assert!(is_valid_imessage_target("user+tag@example.com"));
    }

    /// 测试带点的邮箱地址验证
    ///
    /// 验证本地部分包含多个点的邮箱地址
    #[test]
    fn valid_email_with_dots() {
        assert!(is_valid_imessage_target("first.last@example.com"));
    }

    /// 测试 iCloud 相关邮箱地址验证
    ///
    /// 验证 Apple iCloud 相关的邮箱域名：
    /// - @icloud.com
    /// - @me.com
    #[test]
    fn valid_email_icloud() {
        assert!(is_valid_imessage_target("user@icloud.com"));
        assert!(is_valid_imessage_target("user@me.com"));
    }

    /// 测试空目标的验证
    ///
    /// 空字符串和仅包含空白的字符串应该被拒绝
    #[test]
    fn invalid_target_empty() {
        assert!(!is_valid_imessage_target(""));
        assert!(!is_valid_imessage_target("   "));
    }

    /// 测试无加号前缀的电话号码验证
    ///
    /// 电话号码必须以 + 开头，否则应该被拒绝
    #[test]
    fn invalid_target_no_plus_prefix() {
        // 电话号码必须以 + 开头
        assert!(!is_valid_imessage_target("1234567890"));
    }

    /// 测试过短电话号码的验证
    ///
    /// 少于 7 位数字的电话号码应该被拒绝
    #[test]
    fn invalid_target_too_short_phone() {
        // 少于 7 位数字
        assert!(!is_valid_imessage_target("+123456"));
    }

    /// 测试过长电话号码的验证
    ///
    /// 超过 15 位数字的电话号码应该被拒绝
    #[test]
    fn invalid_target_too_long_phone() {
        // 超过 15 位数字
        assert!(!is_valid_imessage_target("+1234567890123456"));
    }

    /// 测试无 @ 符号的邮箱地址验证
    ///
    /// 缺少 @ 符号的邮箱地址应该被拒绝
    #[test]
    fn invalid_target_email_no_at() {
        assert!(!is_valid_imessage_target("userexample.com"));
    }

    /// 测试无域名的邮箱地址验证
    ///
    /// @ 后缺少域名的邮箱地址应该被拒绝
    #[test]
    fn invalid_target_email_no_domain() {
        assert!(!is_valid_imessage_target("user@"));
    }

    /// 测试无本地部分的邮箱地址验证
    ///
    /// @ 前缺少本地部分的邮箱地址应该被拒绝
    #[test]
    fn invalid_target_email_no_local() {
        assert!(!is_valid_imessage_target("@example.com"));
    }

    /// 测试域名中无点的邮箱地址验证
    ///
    /// 域名部分必须包含至少一个点
    #[test]
    fn invalid_target_email_no_dot_in_domain() {
        assert!(!is_valid_imessage_target("user@localhost"));
    }

    /// 测试注入攻击目标的验证
    ///
    /// 这是安全报告中提到的确切攻击向量，
    /// 应该被目标验证器拒绝
    #[test]
    fn invalid_target_injection_attempt() {
        // 安全报告中提到的确切攻击向量
        assert!(!is_valid_imessage_target(r#"" & do shell script "id" & ""#));
    }

    /// 测试 AppleScript 注入尝试的验证
    ///
    /// 验证各种注入攻击模式都被拒绝：
    /// - 引号逃逸尝试
    /// - 换行符注入
    /// - 分号注入
    #[test]
    fn invalid_target_applescript_injection() {
        // 各种注入尝试
        assert!(!is_valid_imessage_target(r#"test" & quit"#));
        assert!(!is_valid_imessage_target(r"test\ndo shell script"));
        assert!(!is_valid_imessage_target("test\"; malicious code; \""));
    }

    /// 测试包含特殊字符的目标验证
    ///
    /// 包含危险特殊字符的目标应该被拒绝：
    /// - HTML 标签
    /// - Shell 命令
    #[test]
    fn invalid_target_special_chars() {
        assert!(!is_valid_imessage_target("user<script>@example.com"));
        assert!(!is_valid_imessage_target("user@example.com; rm -rf /"));
    }

    /// 测试包含空字节的目标验证
    ///
    /// 包含空字符的目标应该被拒绝
    #[test]
    fn invalid_target_null_byte() {
        assert!(!is_valid_imessage_target("user\0@example.com"));
    }

    /// 测试包含换行符的目标验证
    ///
    /// 包含换行符的目标应该被拒绝
    #[test]
    fn invalid_target_newline() {
        assert!(!is_valid_imessage_target("user\n@example.com"));
    }

    /// 测试前后空白会被修剪
    ///
    /// 验证目标验证前会自动修剪前后空白
    #[test]
    fn target_with_leading_trailing_whitespace_trimmed() {
        // 应该修剪并验证
        assert!(is_valid_imessage_target("  +1234567890  "));
        assert!(is_valid_imessage_target("  user@example.com  "));
    }

    // ══════════════════════════════════════════════════════════
    // SQLite/rusqlite 数据库测试（CWE-89 SQL 注入防护）
    // ══════════════════════════════════════════════════════════

    /// 创建临时测试数据库的辅助函数
    ///
    /// 创建一个临时的 SQLite 数据库，包含最小化的 Messages 架构：
    /// - `handle` 表：存储联系人信息（ROWID 和 id）
    /// - `message` 表：存储消息（ROWID、handle_id、text、is_from_me）
    ///
    /// # 返回值
    ///
    /// 返回一个元组：
    /// - `TempDir`：临时目录句柄（离开作用域时自动清理）
    /// - `PathBuf`：数据库文件路径
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let (_dir, db_path) = create_test_db();
    /// // _dir 保持存活以防止临时目录被删除
    /// let result = get_max_rowid(&db_path).await;
    /// ```
    fn create_test_db() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("chat.db");

        let conn = Connection::open(&db_path).unwrap();

        // 创建与 macOS Messages.app 匹配的最小架构
        conn.execute_batch(
            "CREATE TABLE handle (
                    ROWID INTEGER PRIMARY KEY,
                    id TEXT NOT NULL
                );
                CREATE TABLE message (
                    ROWID INTEGER PRIMARY KEY,
                    handle_id INTEGER,
                    text TEXT,
                    is_from_me INTEGER DEFAULT 0,
                    FOREIGN KEY (handle_id) REFERENCES handle(ROWID)
                );",
        )
        .unwrap();

        (dir, db_path)
    }

    /// 测试空数据库的 get_max_rowid
    ///
    /// 验证空消息表返回 0（NULL 被合并为 0）
    #[tokio::test]
    async fn get_max_rowid_empty_database() {
        let (_dir, db_path) = create_test_db();
        let result = get_max_rowid(&db_path).await;
        assert!(result.is_ok());
        // 空表返回 0（NULL 被合并）
        assert_eq!(result.unwrap(), 0);
    }

    /// 测试包含消息的数据库的 get_max_rowid
    ///
    /// 验证：
    /// - 返回接收消息（is_from_me=0）的最大 ROWID
    /// - 忽略发送消息（is_from_me=1）
    #[tokio::test]
    async fn get_max_rowid_with_messages() {
        let (_dir, db_path) = create_test_db();

        // 插入测试数据
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("INSERT INTO handle (ROWID, id) VALUES (1, '+1234567890')", []).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (100, 1, 'Hello', 0)",
                    []
                ).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (200, 1, 'World', 0)",
                    []
                ).unwrap();
            // 这条消息 is_from_me=1，应该被忽略
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (300, 1, 'Sent', 1)",
                    []
                ).unwrap();
        }

        let result = get_max_rowid(&db_path).await.unwrap();
        // 应该返回 200，而不是 300（忽略 is_from_me=1）
        assert_eq!(result, 200);
    }

    /// 测试不存在的数据库的 get_max_rowid
    ///
    /// 验证访问不存在的数据库时返回错误
    #[tokio::test]
    async fn get_max_rowid_nonexistent_database() {
        let path = std::path::Path::new("/nonexistent/path/chat.db");
        let result = get_max_rowid(path).await;
        assert!(result.is_err());
    }

    /// 测试空数据库的 fetch_new_messages
    ///
    /// 验证空数据库返回空的消息列表
    #[tokio::test]
    async fn fetch_new_messages_empty_database() {
        let (_dir, db_path) = create_test_db();
        let result = fetch_new_messages(&db_path, 0).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    /// 测试 fetch_new_messages 返回正确的数据
    ///
    /// 验证：
    /// - 正确获取多条消息
    /// - 消息数据包含正确的 ROWID、联系人和文本
    #[tokio::test]
    async fn fetch_new_messages_returns_correct_data() {
        let (_dir, db_path) = create_test_db();

        // 插入测试数据
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("INSERT INTO handle (ROWID, id) VALUES (1, '+1234567890')", []).unwrap();
            conn.execute("INSERT INTO handle (ROWID, id) VALUES (2, 'user@example.com')", [])
                .unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (10, 1, 'First message', 0)",
                    []
                ).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (20, 2, 'Second message', 0)",
                    []
                ).unwrap();
        }

        let result = fetch_new_messages(&db_path, 0).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], (10, "+1234567890".to_string(), "First message".to_string()));
        assert_eq!(result[1], (20, "user@example.com".to_string(), "Second message".to_string()));
    }

    /// 测试 fetch_new_messages 按 ROWID 过滤
    ///
    /// 验证只返回 ROWID 大于指定值的消息
    #[tokio::test]
    async fn fetch_new_messages_filters_by_rowid() {
        let (_dir, db_path) = create_test_db();

        // 插入测试数据
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("INSERT INTO handle (ROWID, id) VALUES (1, '+1234567890')", []).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (10, 1, 'Old message', 0)",
                    []
                ).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (20, 1, 'New message', 0)",
                    []
                ).unwrap();
        }

        // 只获取 ROWID 大于 15 的消息
        let result = fetch_new_messages(&db_path, 15).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, 20);
        assert_eq!(result[0].2, "New message");
    }

    /// 测试 fetch_new_messages 排除已发送消息
    ///
    /// 验证 is_from_me=1 的消息（自己发送的）被排除
    #[tokio::test]
    async fn fetch_new_messages_excludes_sent_messages() {
        let (_dir, db_path) = create_test_db();

        // 插入测试数据
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("INSERT INTO handle (ROWID, id) VALUES (1, '+1234567890')", []).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (10, 1, 'Received', 0)",
                    []
                ).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (20, 1, 'Sent by me', 1)",
                    []
                ).unwrap();
        }

        let result = fetch_new_messages(&db_path, 0).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].2, "Received");
    }

    /// 测试 fetch_new_messages 排除 NULL 文本
    ///
    /// 验证文本为 NULL 的消息被排除
    #[tokio::test]
    async fn fetch_new_messages_excludes_null_text() {
        let (_dir, db_path) = create_test_db();

        // 插入测试数据
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("INSERT INTO handle (ROWID, id) VALUES (1, '+1234567890')", []).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (10, 1, 'Has text', 0)",
                    []
                ).unwrap();
            conn.execute(
                "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (20, 1, NULL, 0)",
                [],
            )
            .unwrap();
        }

        let result = fetch_new_messages(&db_path, 0).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].2, "Has text");
    }

    /// 测试 fetch_new_messages 遵守限制
    ///
    /// 验证返回的消息数量不超过限制（默认 20 条）
    #[tokio::test]
    async fn fetch_new_messages_respects_limit() {
        let (_dir, db_path) = create_test_db();

        // 插入 25 条消息（限制为 20）
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("INSERT INTO handle (ROWID, id) VALUES (1, '+1234567890')", []).unwrap();
            for i in 1..=25 {
                conn.execute(
                        &format!("INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES ({i}, 1, 'Message {i}', 0)"),
                        []
                    ).unwrap();
            }
        }

        let result = fetch_new_messages(&db_path, 0).await.unwrap();
        assert_eq!(result.len(), 20); // 限制为 20 条
        assert_eq!(result[0].0, 1); // 第一条消息
        assert_eq!(result[19].0, 20); // 第 20 条消息
    }

    /// 测试 fetch_new_messages 按 ROWID 升序排序
    ///
    /// 验证消息按 ROWID 升序返回，即使插入顺序不同
    #[tokio::test]
    async fn fetch_new_messages_ordered_by_rowid_asc() {
        let (_dir, db_path) = create_test_db();

        // 乱序插入消息
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("INSERT INTO handle (ROWID, id) VALUES (1, '+1234567890')", []).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (30, 1, 'Third', 0)",
                    []
                ).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (10, 1, 'First', 0)",
                    []
                ).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (20, 1, 'Second', 0)",
                    []
                ).unwrap();
        }

        let result = fetch_new_messages(&db_path, 0).await.unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, 10);
        assert_eq!(result[1].0, 20);
        assert_eq!(result[2].0, 30);
    }

    /// 测试 fetch_new_messages 处理不存在的数据库
    ///
    /// 验证访问不存在的数据库时返回错误
    #[tokio::test]
    async fn fetch_new_messages_nonexistent_database() {
        let path = std::path::Path::new("/nonexistent/path/chat.db");
        let result = fetch_new_messages(path, 0).await;
        assert!(result.is_err());
    }

    /// 测试 fetch_new_messages 处理特殊字符
    ///
    /// 验证：
    /// - 消息中的特殊字符被正确保留
    /// - SQL 注入模式被当作普通文本处理
    /// - 参数化查询防止 SQL 注入
    #[tokio::test]
    async fn fetch_new_messages_handles_special_characters() {
        let (_dir, db_path) = create_test_db();

        // 插入包含特殊字符的消息（潜在的 SQL 注入模式）
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("INSERT INTO handle (ROWID, id) VALUES (1, '+1234567890')", []).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (10, 1, 'Hello \"world'' OR 1=1; DROP TABLE message;--', 0)",
                    []
                ).unwrap();
        }

        let result = fetch_new_messages(&db_path, 0).await.unwrap();
        assert_eq!(result.len(), 1);
        // 特殊字符应该被保留，而不是被解释为 SQL
        assert!(result[0].2.contains("DROP TABLE"));
    }

    /// 测试 fetch_new_messages 处理 Unicode 字符
    ///
    /// 验证各种 Unicode 字符（emoji、中文、阿拉伯文）被正确处理
    #[tokio::test]
    async fn fetch_new_messages_handles_unicode() {
        let (_dir, db_path) = create_test_db();

        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("INSERT INTO handle (ROWID, id) VALUES (1, '+1234567890')", []).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (10, 1, 'Hello 世界 مرحبا', 0)",
                    []
                ).unwrap();
        }

        let result = fetch_new_messages(&db_path, 0).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].2, "Hello 世界 مرحبا");
    }

    /// 测试 fetch_new_messages 处理空文本
    ///
    /// 验证空字符串（不是 NULL）的消息被包含在结果中
    #[tokio::test]
    async fn fetch_new_messages_handles_empty_text() {
        let (_dir, db_path) = create_test_db();

        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("INSERT INTO handle (ROWID, id) VALUES (1, '+1234567890')", []).unwrap();
            conn.execute(
                "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (10, 1, '', 0)",
                [],
            )
            .unwrap();
        }

        let result = fetch_new_messages(&db_path, 0).await.unwrap();
        // 空字符串不是 NULL，所以会被包含
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].2, "");
    }

    /// 测试 fetch_new_messages 处理负数 ROWID 边界情况
    ///
    /// 验证负数 ROWID 仍然可以正常工作（获取 ROWID > -1 的所有消息）
    #[tokio::test]
    async fn fetch_new_messages_negative_rowid_edge_case() {
        let (_dir, db_path) = create_test_db();

        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("INSERT INTO handle (ROWID, id) VALUES (1, '+1234567890')", []).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (10, 1, 'Test', 0)",
                    []
                ).unwrap();
        }

        // 负数 rowid 仍然应该工作（获取 ROWID > -1 的所有消息）
        let result = fetch_new_messages(&db_path, -1).await.unwrap();
        assert_eq!(result.len(), 1);
    }

    /// 测试 fetch_new_messages 处理极大 ROWID 边界情况
    ///
    /// 验证使用极大 ROWID 时返回空结果（没有消息大于此 ROWID）
    #[tokio::test]
    async fn fetch_new_messages_large_rowid_edge_case() {
        let (_dir, db_path) = create_test_db();

        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("INSERT INTO handle (ROWID, id) VALUES (1, '+1234567890')", []).unwrap();
            conn.execute(
                    "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (10, 1, 'Test', 0)",
                    []
                ).unwrap();
        }

        // 极大的 rowid 应该返回空结果（没有消息大于此 ROWID）
        let result = fetch_new_messages(&db_path, i64::MAX - 1).await.unwrap();
        assert!(result.is_empty());
    }
}
