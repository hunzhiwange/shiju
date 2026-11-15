//! Mattermost 频道模块单元测试
//!
//! 本模块包含 MattermostChannel 的完整测试套件，覆盖以下核心功能：
//!
//! - URL 处理（尾部斜杠修剪）
//! - 用户白名单验证（包括通配符支持）
//! - 帖子解析逻辑（消息内容、发送者、回复目标）
//! - 线程回复模式（启用/禁用时的行为差异）
//! - 仅提及模式（mention_only）的过滤逻辑
//! - 机器人提及检测（文本和元数据两种方式）
//! - 消息内容规范化处理
//!
//! 测试遵循 Rust 单元测试最佳实践，每个测试聚焦单一行为验证。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use serde_json::json;

    /// 创建标准频道的辅助函数
    ///
    /// 创建一个 `mention_only=false` 的频道实例，模拟默认（遗留）行为。
    ///
    /// # 参数
    ///
    /// - `allowed`: 允许交互的用户 ID 列表，支持通配符 `"*"`
    /// - `thread_replies`: 是否启用线程回复模式
    ///
    /// # 返回
    ///
    /// 返回配置好的 MattermostChannel 实例
    fn make_channel(allowed: Vec<String>, thread_replies: bool) -> MattermostChannel {
        MattermostChannel::new("url".into(), "token".into(), None, allowed, thread_replies, false)
    }

    /// 创建仅提及模式频道的辅助函数
    ///
    /// 创建一个 `mention_only=true` 的频道实例，该模式下机器人仅响应直接提及它的消息。
    fn make_mention_only_channel() -> MattermostChannel {
        MattermostChannel::new("url".into(), "token".into(), None, vec!["*".into()], true, true)
    }

    /// 测试 URL 尾部斜杠修剪
    ///
    /// 验证频道初始化时会自动移除 URL 末尾的斜杠，
    /// 确保后续 API 调用时 URL 格式一致。
    #[test]
    fn mattermost_url_trimming() {
        let ch = MattermostChannel::new(
            "https://mm.example.com/".into(),
            "token".into(),
            None,
            vec![],
            false,
            false,
        );
        assert_eq!(ch.base_url, "https://mm.example.com");
    }

    /// 测试通配符白名单匹配
    ///
    /// 当白名单包含 `"*"` 时，所有用户 ID 都应被允许访问。
    #[test]
    fn mattermost_allowlist_wildcard() {
        let ch = make_channel(vec!["*".into()], false);
        assert!(ch.is_user_allowed("any-id"));
    }

    /// 测试基础帖子解析
    ///
    /// 验证能正确从 JSON 帖子对象中提取发送者、内容和回复目标。
    /// 当启用线程回复时，新帖子的回复目标应包含帖子 ID。
    #[test]
    fn mattermost_parse_post_basic() {
        let ch = make_channel(vec!["*".into()], true);
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "hello world",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(&post, "bot123", "botname", 1_500_000_000_000_i64, "chan789")
            .unwrap();
        assert_eq!(msg.sender, "user456");
        assert_eq!(msg.content, "hello world");
        assert_eq!(msg.reply_target, "chan789:post123");
    }

    /// 测试线程回复模式启用时的行为
    ///
    /// 当 `thread_replies=true` 时，即使是顶级帖子（无 root_id），
    /// 回复目标也应设置为线程格式（频道:帖子ID）。
    #[test]
    fn mattermost_parse_post_thread_replies_enabled() {
        let ch = make_channel(vec!["*".into()], true);
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "hello world",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(&post, "bot123", "botname", 1_500_000_000_000_i64, "chan789")
            .unwrap();
        assert_eq!(msg.reply_target, "chan789:post123");
    }

    /// 测试现有线程中的回复解析
    ///
    /// 当帖子已有 `root_id` 时，表示它是某个线程的一部分，
    /// 回复目标应保持在该线程内，使用 root_id 作为回复目标。
    #[test]
    fn mattermost_parse_post_thread() {
        let ch = make_channel(vec!["*".into()], false);
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "reply",
            "create_at": 1_600_000_000_000_i64,
            "root_id": "root789"
        });

        let msg = ch
            .parse_mattermost_post(&post, "bot123", "botname", 1_500_000_000_000_i64, "chan789")
            .unwrap();
        assert_eq!(msg.reply_target, "chan789:root789");
    }

    /// 测试忽略机器人自身消息
    ///
    /// 当帖子的 user_id 与机器人 ID 相同时，应返回 None，
    /// 防止机器人对自己产生无限回复循环。
    #[test]
    fn mattermost_parse_post_ignore_self() {
        let ch = make_channel(vec!["*".into()], false);
        let post = json!({
            "id": "post123",
            "user_id": "bot123",
            "message": "my own message",
            "create_at": 1_600_000_000_000_i64
        });

        let msg =
            ch.parse_mattermost_post(&post, "bot123", "botname", 1_500_000_000_000_i64, "chan789");
        assert!(msg.is_none());
    }

    /// 测试忽略历史旧消息
    ///
    /// 当帖子的创建时间早于已知的最后处理时间戳时，
    /// 应返回 None 以避免重复处理。
    #[test]
    fn mattermost_parse_post_ignore_old() {
        let ch = make_channel(vec!["*".into()], false);
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "old message",
            "create_at": 1_400_000_000_000_i64
        });

        let msg =
            ch.parse_mattermost_post(&post, "bot123", "botname", 1_500_000_000_000_i64, "chan789");
        assert!(msg.is_none());
    }

    /// 测试线程回复禁用时的行为
    ///
    /// 当 `thread_replies=false` 且帖子是顶级帖子（无 root_id）时，
    /// 回复目标应仅为频道 ID，不带帖子后缀。
    #[test]
    fn mattermost_parse_post_no_thread_when_disabled() {
        let ch = make_channel(vec!["*".into()], false);
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "hello world",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(&post, "bot123", "botname", 1_500_000_000_000_i64, "chan789")
            .unwrap();
        assert_eq!(msg.reply_target, "chan789");
    }

    /// 测试现有线程始终保持线程化
    ///
    /// 即使 `thread_replies=false`，对现有线程（有 root_id）的回复
    /// 仍应保持在原线程内，这是为了维护对话上下文的完整性。
    #[test]
    fn mattermost_existing_thread_always_threads() {
        let ch = make_channel(vec!["*".into()], false);
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "reply in thread",
            "create_at": 1_600_000_000_000_i64,
            "root_id": "root789"
        });

        let msg = ch
            .parse_mattermost_post(&post, "bot123", "botname", 1_500_000_000_000_i64, "chan789")
            .unwrap();
        assert_eq!(msg.reply_target, "chan789:root789");
    }

    // ── 仅提及模式（mention_only）测试 ─────────────────────────────────────

    /// 测试仅提及模式过滤无提及消息
    ///
    /// 当 `mention_only=true` 时，不包含 @提及 的消息应被忽略。
    #[test]
    fn mention_only_skips_message_without_mention() {
        let ch = make_mention_only_channel();
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "hello everyone",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg =
            ch.parse_mattermost_post(&post, "bot123", "mybot", 1_500_000_000_000_i64, "chan1");
        assert!(msg.is_none());
    }

    /// 测试仅提及模式接受带 @提及 的消息
    ///
    /// 当消息以 `@botname` 开头时，应被正确解析，
    /// 且内容中移除提及部分后的文本被保留。
    #[test]
    fn mention_only_accepts_message_with_at_mention() {
        let ch = make_mention_only_channel();
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "@mybot what is the weather?",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(&post, "bot123", "mybot", 1_500_000_000_000_i64, "chan1")
            .unwrap();
        assert_eq!(msg.content, "what is the weather?");
    }

    /// 测试提及移除和空白修剪
    ///
    /// 验证移除 @提及 后会正确修剪前后空白字符。
    #[test]
    fn mention_only_strips_mention_and_trims() {
        let ch = make_mention_only_channel();
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "  @mybot  run status  ",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(&post, "bot123", "mybot", 1_500_000_000_000_i64, "chan1")
            .unwrap();
        assert_eq!(msg.content, "run status");
    }

    /// 测试仅包含提及时拒绝消息
    ///
    /// 当消息仅包含 `@botname` 而无其他内容时，应返回 None。
    #[test]
    fn mention_only_rejects_empty_after_stripping() {
        let ch = make_mention_only_channel();
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "@mybot",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg =
            ch.parse_mattermost_post(&post, "bot123", "mybot", 1_500_000_000_000_i64, "chan1");
        assert!(msg.is_none());
    }

    /// 测试提及检测的大小写不敏感
    ///
    /// `@MyBot` 应与 `@mybot` 匹配，实现大小写不敏感的用户名匹配。
    #[test]
    fn mention_only_case_insensitive() {
        let ch = make_mention_only_channel();
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "@MyBot hello",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(&post, "bot123", "mybot", 1_500_000_000_000_i64, "chan1")
            .unwrap();
        assert_eq!(msg.content, "hello");
    }

    /// 测试通过元数据（metadata）检测提及
    ///
    /// 即使消息文本中没有 `@username`，只要 `metadata.mentions` 包含机器人 ID，
    /// 也应触发处理。此时内容原样保留，因为没有需要移除的文本提及。
    #[test]
    fn mention_only_detects_metadata_mentions() {
        let ch = make_mention_only_channel();
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "hey check this out",
            "create_at": 1_600_000_000_000_i64,
            "root_id": "",
            "metadata": {
                "mentions": ["bot123"]
            }
        });

        let msg = ch
            .parse_mattermost_post(&post, "bot123", "mybot", 1_500_000_000_000_i64, "chan1")
            .unwrap();
        assert_eq!(msg.content, "hey check this out");
    }

    /// 测试词边界防止部分匹配
    ///
    /// `@mybotextended` 不应匹配 `@mybot`，因为用户名后跟随了额外字符。
    /// 词边界检测确保不会发生意外的部分用户名匹配。
    #[test]
    fn mention_only_word_boundary_prevents_partial_match() {
        let ch = make_mention_only_channel();
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "@mybotextended hello",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg =
            ch.parse_mattermost_post(&post, "bot123", "mybot", 1_500_000_000_000_i64, "chan1");
        assert!(msg.is_none());
    }

    /// 测试提及在消息中间的情况
    ///
    /// 当 `@botname` 出现在消息中间时，移除提及后会保留其他文本，
    /// 但原提及位置会留下额外空白。
    #[test]
    fn mention_only_mention_in_middle_of_text() {
        let ch = make_mention_only_channel();
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "hey @mybot how are you?",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(&post, "bot123", "mybot", 1_500_000_000_000_i64, "chan1")
            .unwrap();
        assert_eq!(msg.content, "hey   how are you?");
    }

    /// 测试禁用仅提及模式时所有消息通过
    ///
    /// 当 `mention_only=false`（默认值）时，所有消息都应通过过滤，
    /// 无需检测是否包含提及。
    #[test]
    fn mention_only_disabled_passes_all_messages() {
        let ch = make_channel(vec!["*".into()], true);
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "no mention here",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(&post, "bot123", "mybot", 1_500_000_000_000_i64, "chan1")
            .unwrap();
        assert_eq!(msg.content, "no mention here");
    }

    /// 测试群组回复允许发送者绕过提及要求
    ///
    /// 当配置了 `group_reply_allowed_senders` 时，来自这些用户的
    /// 消息可以绕过仅提及模式的过滤，即使消息中未直接提及机器人。
    #[test]
    fn mention_only_sender_override_allows_without_mention() {
        let ch = make_mention_only_channel()
            .with_group_reply_allowed_senders(vec!["user1".into(), " user1 ".into()]);
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "hello everyone",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(&post, "bot123", "mybot", 1_500_000_000_000_i64, "chan1")
            .unwrap();
        assert_eq!(msg.content, "hello everyone");
    }

    // ── contains_bot_mention_mm 函数单元测试 ────────────────────────

    /// 测试提及在文本末尾的检测
    #[test]
    fn contains_mention_text_at_end() {
        let post = json!({});
        assert!(contains_bot_mention_mm("hello @mybot", "bot123", "mybot", &post));
    }

    /// 测试提及在文本开头的检测
    #[test]
    fn contains_mention_text_at_start() {
        let post = json!({});
        assert!(contains_bot_mention_mm("@mybot hello", "bot123", "mybot", &post));
    }

    /// 测试仅包含提及的检测
    #[test]
    fn contains_mention_text_alone() {
        let post = json!({});
        assert!(contains_bot_mention_mm("@mybot", "bot123", "mybot", &post));
    }

    /// 测试不同用户名不触发匹配
    #[test]
    fn no_mention_different_username() {
        let post = json!({});
        assert!(!contains_bot_mention_mm("@otherbot hello", "bot123", "mybot", &post));
    }

    /// 测试部分用户名不触发匹配
    ///
    /// "mybot" 是 "mybotx" 的前缀，但不应匹配。
    #[test]
    fn no_mention_partial_username() {
        let post = json!({});
        assert!(!contains_bot_mention_mm("@mybotx hello", "bot123", "mybot", &post));
    }

    /// 测试在部分前缀后检测有效提及
    ///
    /// 当文本中同时存在部分前缀和完整匹配时，应能检测到有效提及。
    #[test]
    fn mention_detects_later_valid_mention_after_partial_prefix() {
        let post = json!({});
        assert!(contains_bot_mention_mm(
            "@mybotx ignore this, but @mybot handle this",
            "bot123",
            "mybot",
            &post
        ));
    }

    /// 测试提及后跟随标点符号
    ///
    /// 逗号不是字母数字/下划线/连字符/点，因此构成有效的词边界。
    #[test]
    fn mention_followed_by_punctuation() {
        let post = json!({});
        assert!(contains_bot_mention_mm("@mybot, hello", "bot123", "mybot", &post));
    }

    /// 测试仅通过元数据检测提及
    ///
    /// 当文本中没有 @ 符号，但 metadata.mentions 包含机器人 ID 时，应返回 true。
    #[test]
    fn mention_via_metadata_only() {
        let post = json!({
            "metadata": { "mentions": ["bot123"] }
        });
        assert!(contains_bot_mention_mm("no at mention", "bot123", "mybot", &post));
    }

    /// 测试空用户名且无元数据时不触发匹配
    #[test]
    fn no_mention_empty_username_no_metadata() {
        let post = json!({});
        assert!(!contains_bot_mention_mm("hello world", "bot123", "", &post));
    }

    // ── normalize_mattermost_content 函数单元测试 ───────────────────

    /// 测试内容规范化：移除提及并修剪空白
    #[test]
    fn normalize_strips_and_trims() {
        let post = json!({});
        let result = normalize_mattermost_content("  @mybot  do stuff  ", "bot123", "mybot", &post);
        assert_eq!(result.as_deref(), Some("do stuff"));
    }

    /// 测试无提及时返回 None
    #[test]
    fn normalize_returns_none_for_no_mention() {
        let post = json!({});
        let result = normalize_mattermost_content("hello world", "bot123", "mybot", &post);
        assert!(result.is_none());
    }

    /// 测试仅包含提及时返回 None
    #[test]
    fn normalize_returns_none_when_only_mention() {
        let post = json!({});
        let result = normalize_mattermost_content("@mybot", "bot123", "mybot", &post);
        assert!(result.is_none());
    }

    /// 测试元数据提及时保留原文本
    ///
    /// 当通过 metadata 检测到提及时，文本原样保留。
    #[test]
    fn normalize_preserves_text_for_metadata_mention() {
        let post = json!({
            "metadata": { "mentions": ["bot123"] }
        });
        let result = normalize_mattermost_content("check this out", "bot123", "mybot", &post);
        assert_eq!(result.as_deref(), Some("check this out"));
    }

    /// 测试移除多次提及
    #[test]
    fn normalize_strips_multiple_mentions() {
        let post = json!({});
        let result =
            normalize_mattermost_content("@mybot hello @mybot world", "bot123", "mybot", &post);
        assert_eq!(result.as_deref(), Some("hello   world"));
    }

    /// 测试保留部分用户名提及
    ///
    /// `@mybotx` 不匹配 `@mybot`，应原样保留。
    #[test]
    fn normalize_keeps_partial_username_mentions() {
        let post = json!({});
        let result =
            normalize_mattermost_content("@mybot hello @mybotx world", "bot123", "mybot", &post);
        assert_eq!(result.as_deref(), Some("hello @mybotx world"));
    }

    /// 测试群组回复允许发送者列表去重
    ///
    /// 验证函数能正确去除空白、去重并过滤空字符串。
    #[test]
    fn normalize_group_reply_allowed_sender_ids_deduplicates() {
        let normalized = normalize_group_reply_allowed_sender_ids(vec![
            " user-1 ".into(),
            "user-1".into(),
            String::new(),
            "user-2".into(),
        ]);
        assert_eq!(normalized, vec!["user-1".to_string(), "user-2".to_string()]);
    }
}
