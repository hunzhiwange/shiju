//! Matrix 通道模块的单元测试集
//!
//! 本模块包含对 `MatrixChannel` 及相关数据结构的全面测试，覆盖以下功能领域：
//!
//! - **通道构造与配置**：验证 homeserver URL 规范化、访问令牌修剪、房间 ID 处理等
//! - **会话管理**：测试会话提示（session hints）的规范化与存储目录派生
//! - **端到端加密**：验证一次性密钥（OTK）冲突检测与恢复提示生成
//! - **消息处理**：测试消息类型检测、@提及 解析、回复关系提取、Markdown 格式化
//! - **用户权限**：验证允许列表匹配、通配符支持、大小写不敏感性
//! - **同步协议**：测试 Matrix 同步响应反序列化与过滤
//! - **缓存机制**：验证事件 ID 去重缓存与 LRU 淘汰行为
//!
//! # 测试约定
//!
//! - 所有测试使用模拟数据，不依赖真实的 Matrix 服务器连接
//! - 异步测试使用 `#[tokio::test]` 属性
//! - 测试函数命名遵循 `<功能>_<预期行为>` 模式

use super::types::{SyncResponse, TimelineEvent, WhoAmIResponse};
use super::*;
use crate::app::agent::channels::traits::Channel;
use matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use std::path::PathBuf;

/// 测试模块：封装所有 Matrix 通道相关测试用例
///
/// 使用 `#[allow(dead_code)]` 是因为测试模块中的辅助函数可能在某些配置下
/// 被编译器误判为未使用，但它们在测试执行时确实被调用。
#[allow(dead_code)]
mod tests {
    use super::*;

    /// 创建一个标准的测试用 MatrixChannel 实例
    ///
    /// 该辅助函数提供一个预配置的通道实例，用于需要基础通道对象的测试。
    /// 使用固定的测试参数以确保测试的可重复性。
    ///
    /// # 返回值
    ///
    /// 返回配置如下的 `MatrixChannel` 实例：
    /// - homeserver: `https://matrix.org`
    /// - access_token: `syt_test_token`
    /// - room_id: `!room:matrix.org`
    /// - allowed_users: `["@user:matrix.org"]`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = make_channel();
    /// assert_eq!(channel.name(), "matrix");
    /// ```
    fn make_channel() -> MatrixChannel {
        MatrixChannel::new(
            "https://matrix.org".to_string(),
            "syt_test_token".to_string(),
            "!room:matrix.org".to_string(),
            vec!["@user:matrix.org".to_string()],
        )
    }

    /// 测试：验证通道构造时各字段正确设置
    ///
    /// 确保通过 `new()` 构造函数创建的 `MatrixChannel` 实例
    /// 其核心字段（homeserver、access_token、room_id、allowed_users）
    /// 被正确初始化为传入的值。
    #[test]
    fn creates_with_correct_fields() {
        let ch = make_channel();
        assert_eq!(ch.homeserver, "https://matrix.org");
        assert_eq!(ch.access_token, "syt_test_token");
        assert_eq!(ch.room_id, "!room:matrix.org");
        assert_eq!(ch.allowed_users.len(), 1);
    }

    /// 测试：homeserver URL 尾部斜杠被移除
    ///
    /// 当 homeserver URL 以单个斜杠结尾时，构造函数应自动移除，
    /// 以确保后续 API 请求路径拼接的正确性。
    #[test]
    fn strips_trailing_slash() {
        let ch = MatrixChannel::new(
            "https://matrix.org/".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec![],
        );
        assert_eq!(ch.homeserver, "https://matrix.org");
    }

    /// 测试：无尾部斜杠的 URL 保持不变
    ///
    /// 验证当 homeserver URL 已规范（无尾部斜杠）时，
    /// 构造函数不会对其进行不必要的修改。
    #[test]
    fn no_trailing_slash_unchanged() {
        let ch = MatrixChannel::new(
            "https://matrix.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec![],
        );
        assert_eq!(ch.homeserver, "https://matrix.org");
    }

    /// 测试：多个尾部斜杠全部被移除
    ///
    /// 当 homeserver URL 以多个斜杠结尾时，构造函数应全部移除，
    /// 确保最终的 URL 是规范的。
    #[test]
    fn multiple_trailing_slashes_strip_all() {
        let ch = MatrixChannel::new(
            "https://matrix.org//".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec![],
        );
        assert_eq!(ch.homeserver, "https://matrix.org");
    }

    /// 测试：访问令牌首尾空白被修剪
    ///
    /// 验证构造函数会自动修剪 access_token 字段的前导和尾部空白字符，
    /// 防止因配置文件格式问题导致的认证失败。
    #[test]
    fn trims_access_token() {
        let ch = MatrixChannel::new(
            "https://matrix.org".to_string(),
            "  syt_test_token  ".to_string(),
            "!r:m".to_string(),
            vec![],
        );
        assert_eq!(ch.access_token, "syt_test_token");
    }

    /// 测试：会话提示（session hints）被正确规范化
    ///
    /// 验证 `new_with_session_hint` 构造函数会修剪会话所有者提示
    /// 和设备 ID 提示的首尾空白，确保存储的是规范化的值。
    ///
    /// 会话提示用于在端到端加密场景中标识机器人的 Matrix 身份，
    /// 以便正确解密发往该身份的消息。
    #[test]
    fn session_hints_are_normalized() {
        let ch = MatrixChannel::new_with_session_hint(
            "https://matrix.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec![],
            Some("  @bot:matrix.org ".to_string()),
            Some("  DEVICE123  ".to_string()),
        );

        assert_eq!(ch.session_owner_hint.as_deref(), Some("@bot:matrix.org"));
        assert_eq!(ch.session_device_id_hint.as_deref(), Some("DEVICE123"));
    }

    /// 测试：空白会话提示被忽略
    ///
    /// 验证当会话提示仅包含空白或为空字符串时，
    /// 构造函数会将其设置为 `None`，而非存储无效值。
    /// 这避免了后续加密流程中使用无效的身份标识。
    #[test]
    fn empty_session_hints_are_ignored() {
        let ch = MatrixChannel::new_with_session_hint(
            "https://matrix.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec![],
            Some("   ".to_string()),
            Some(String::new()),
        );

        assert!(ch.session_owner_hint.is_none());
        assert!(ch.session_device_id_hint.is_none());
    }

    /// 测试：Matrix 存储目录从 VibeWindow 目录正确派生
    ///
    /// 验证当提供 `vibewindow_dir` 时，`matrix_store_dir()` 方法
    /// 返回正确的子目录路径 `<vibewindow_dir>/state/matrix`。
    /// 该目录用于存储 Matrix 客户端的本地状态，如加密密钥和会话数据。
    #[test]
    fn matrix_store_dir_is_derived_from_vibewindow_dir() {
        let ch = MatrixChannel::new_with_session_hint_and_vibewindow_dir(
            "https://matrix.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec![],
            None,
            None,
            Some(PathBuf::from("/tmp/vibewindow")),
        );

        assert_eq!(ch.matrix_store_dir(), Some(PathBuf::from("/tmp/vibewindow/state/matrix")));
    }

    /// 测试：未设置 VibeWindow 目录时存储目录为空
    ///
    /// 验证当未提供 `vibewindow_dir` 时，`matrix_store_dir()` 返回 `None`。
    /// 这表明通道将使用默认的内存存储或系统临时目录。
    #[test]
    fn matrix_store_dir_absent_without_vibewindow_dir() {
        let ch = MatrixChannel::new_with_session_hint(
            "https://matrix.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec![],
            None,
            None,
        );

        assert!(ch.matrix_store_dir().is_none());
    }

    /// 测试：一次性密钥（OTK）冲突消息检测正确匹配 Matrix 错误
    ///
    /// 验证 `is_otk_conflict_message` 方法能正确识别 Matrix 服务器
    /// 返回的一次性密钥冲突错误消息。该错误通常发生在
    /// 多个客户端实例使用相同设备 ID 登录时。
    ///
    /// # 检测逻辑
    ///
    /// 方法通过检查消息是否包含 "already exists" 和 "Old key:"
    /// 关键词来识别 OTK 冲突。
    #[test]
    fn otk_conflict_message_detection_matches_matrix_errors() {
        assert!(MatrixChannel::is_otk_conflict_message(
            "One time key signed_curve25519:AAAAAAAAAA4 already exists. Old key: ... new key: ..."
        ));
        assert!(!MatrixChannel::is_otk_conflict_message(
            "Matrix sync timeout while waiting for long poll"
        ));
    }

    /// 测试：OTK 冲突恢复消息包含存储路径（当可用时）
    ///
    /// 验证 `otk_conflict_recovery_message` 方法在生成恢复提示时，
    /// 如果 Matrix 存储目录可用，会在消息中包含该路径，
    /// 指导用户删除相应的状态文件以解决冲突。
    #[test]
    fn otk_conflict_recovery_message_includes_store_path_when_available() {
        let ch = MatrixChannel::new_with_session_hint_and_vibewindow_dir(
            "https://matrix.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec![],
            None,
            None,
            Some(PathBuf::from("/tmp/vibewindow")),
        );

        let message = ch.otk_conflict_recovery_message();
        let expected_store_dir = PathBuf::from("/tmp/vibewindow").join("state").join("matrix");
        assert!(message.contains("one-time key upload conflict"));
        assert!(message.contains(&expected_store_dir.display().to_string()));
    }

    /// 测试：路径段编码正确处理 Matrix 房间引用
    ///
    /// 验证 `encode_path_segment` 方法能正确对 Matrix 房间别名（#）
    /// 和房间 ID（!）进行 URL 编码，使其可以安全地用于 API 路径中。
    ///
    /// # 编码示例
    ///
    /// - `#ops:matrix.example.com` → `%23ops%3Amatrix.example.com`
    /// - `!room:matrix.example.com` → `%21room%3Amatrix.example.com`
    #[test]
    fn encode_path_segment_encodes_room_refs() {
        assert_eq!(
            MatrixChannel::encode_path_segment("#ops:matrix.example.com"),
            "%23ops%3Amatrix.example.com"
        );
        assert_eq!(
            MatrixChannel::encode_path_segment("!room:matrix.example.com"),
            "%21room%3Amatrix.example.com"
        );
    }

    /// 测试：支持的消息类型检测
    ///
    /// 验证 `is_supported_message_type` 方法能正确识别
    /// VibeWindow 支持处理的 Matrix 消息类型：
    ///
    /// - **支持**：`m.text`（纯文本）、`m.notice`（通知）、`m.audio`（音频）
    /// - **不支持**：`m.image`（图片）、`m.file`（文件）
    ///
    /// 不支持的消息类型将被通道忽略，不会触发代理响应。
    #[test]
    fn supported_message_type_detection() {
        assert!(MatrixChannel::is_supported_message_type("m.text"));
        assert!(MatrixChannel::is_supported_message_type("m.notice"));
        assert!(MatrixChannel::is_supported_message_type("m.audio"));
        assert!(!MatrixChannel::is_supported_message_type("m.image"));
        assert!(!MatrixChannel::is_supported_message_type("m.file"));
    }

    /// 测试：消息正文存在性检测
    ///
    /// 验证 `has_non_empty_body` 方法能正确判断消息正文是否有效：
    ///
    /// - 包含非空白字符的文本被视为有效
    /// - 空字符串或仅包含空白字符的文本被视为无效
    ///
    /// 该方法用于过滤空消息，避免触发无意义的代理响应。
    #[test]
    fn body_presence_detection() {
        assert!(MatrixChannel::has_non_empty_body("hello"));
        assert!(MatrixChannel::has_non_empty_body("  hello  "));
        assert!(!MatrixChannel::has_non_empty_body(""));
        assert!(!MatrixChannel::has_non_empty_body("   \n\t  "));
    }

    /// 解析 JSON 值为 Matrix 同步消息事件
    ///
    /// 该辅助函数将 `serde_json::Value` 反序列化为 `OriginalSyncRoomMessageEvent`，
    /// 用于构造测试所需的事件对象。
    ///
    /// # 参数
    ///
    /// * `value` - 包含事件数据的 JSON 值
    ///
    /// # 返回值
    ///
    /// 返回解析成功的事件对象，失败时触发 panic（测试中预期数据总是有效）。
    fn parse_sync_message_event(value: serde_json::Value) -> OriginalSyncRoomMessageEvent {
        serde_json::from_value(value).expect("valid m.room.message event")
    }

    /// 测试：mention_only 构建器正确设置标志
    ///
    /// 验证 `with_mention_only(true)` 方法能正确设置 `mention_only` 标志。
    /// 当该标志为 true 时，通道仅处理包含 @提及 机器人的消息。
    #[test]
    fn mention_only_builder_sets_flag() {
        let ch = make_channel().with_mention_only(true);
        assert!(ch.mention_only);
    }

    /// 测试：事件提及检测 - 纯文本用户 ID
    ///
    /// 验证 `event_mentions_user` 方法能检测消息正文中
    /// 直接出现的 Matrix 用户 ID（如 `@bot:matrix.org`）。
    ///
    /// 这是最基础的 @提及 检测方式，适用于大多数客户端。
    #[test]
    fn event_mentions_user_detects_plain_text_user_id() {
        let event = parse_sync_message_event(serde_json::json!({
            "type": "m.room.message",
            "event_id": "$event:matrix.org",
            "sender": "@user:matrix.org",
            "origin_server_ts": 1u64,
            "content": {
                "msgtype": "m.text",
                "body": "hello @bot:matrix.org"
            }
        }));

        assert!(MatrixChannel::event_mentions_user(
            &event,
            "hello @bot:matrix.org",
            "@bot:matrix.org"
        ));
    }

    /// 测试：事件提及检测 - HTML matrix.to 链接
    ///
    /// 验证 `event_mentions_user` 方法能检测格式化 HTML 消息中的
    /// matrix.to 链接形式的 @提及。某些客户端会将 @提及 渲染为
    /// `<a href="https://matrix.to/#/@user:domain">` 格式的链接。
    #[test]
    fn event_mentions_user_detects_html_matrix_to_link() {
        let event = parse_sync_message_event(serde_json::json!({
            "type": "m.room.message",
            "event_id": "$event:matrix.org",
            "sender": "@user:matrix.org",
            "origin_server_ts": 1u64,
            "content": {
                "msgtype": "m.text",
                "body": "hello bot",
                "format": "org.matrix.custom.html",
                "formatted_body": "<a href=\"https://matrix.to/#/%40bot%3Amatrix.org\">bot</a>"
            }
        }));

        assert!(MatrixChannel::event_mentions_user(&event, "hello bot", "@bot:matrix.org"));
    }

    /// 测试：事件提及检测 - 结构化 m.mentions 字段
    ///
    /// 验证 `event_mentions_user` 方法能检测 Matrix 规范中
    /// 结构化的 `m.mentions` 字段。这是 Matrix 最新的提及机制，
    /// 客户端在发送消息时会显式声明提及的用户列表。
    #[test]
    fn event_mentions_user_detects_structured_mentions() {
        let event = parse_sync_message_event(serde_json::json!({
            "type": "m.room.message",
            "event_id": "$event:matrix.org",
            "sender": "@user:matrix.org",
            "origin_server_ts": 1u64,
            "content": {
                "msgtype": "m.text",
                "body": "hello there",
                "m.mentions": {
                    "user_ids": ["@bot:matrix.org"]
                }
            }
        }));

        assert!(MatrixChannel::event_mentions_user(&event, "hello there", "@bot:matrix.org"));
    }

    /// 测试：回复目标事件 ID 提取 - 正确识别回复关系
    ///
    /// 验证 `reply_target_event_id` 方法能从事件的 `m.relates_to.m.in_reply_to`
    /// 字段中提取被回复消息的事件 ID。该功能用于实现回复链追踪，
    /// 允许代理理解消息的上下文关系。
    #[test]
    fn reply_target_event_id_extracts_reply_relation() {
        let event = parse_sync_message_event(serde_json::json!({
            "type": "m.room.message",
            "event_id": "$event:matrix.org",
            "sender": "@user:matrix.org",
            "origin_server_ts": 1u64,
            "content": {
                "msgtype": "m.text",
                "body": "reply",
                "m.relates_to": {
                    "m.in_reply_to": {
                        "event_id": "$botmsg:matrix.org"
                    }
                }
            }
        }));

        assert_eq!(
            MatrixChannel::reply_target_event_id(&event).as_deref(),
            Some("$botmsg:matrix.org")
        );
    }

    /// 测试：mention_only 门控逻辑符合预期
    ///
    /// 验证 `should_process_message` 方法在不同条件组合下
    /// 是否正确决定是否处理消息。
    ///
    /// # 决策逻辑
    ///
    /// - 如果 `mention_only` 为 `false`，总是处理消息
    /// - 如果 `mention_only` 为 `true`，仅当满足以下任一条件时处理：
    ///   - 消息直接 @提及 了机器人
    ///   - 消息回复了机器人的消息
    ///   - 发送者在允许列表中（受信任用户）
    #[test]
    fn mention_only_gate_behaves_as_expected() {
        // mention_only=false 时总是处理
        assert!(MatrixChannel::should_process_message(false, false, false, false));
        // mention_only=true 且有提及
        assert!(MatrixChannel::should_process_message(true, true, false, false));
        // mention_only=true 且是回复
        assert!(MatrixChannel::should_process_message(true, false, true, false));
        // mention_only=true 且发送者受信任
        assert!(MatrixChannel::should_process_message(true, false, false, true));
        // mention_only=true 但无任何触发条件，不处理
        assert!(!MatrixChannel::should_process_message(true, false, false, false));
    }

    /// 测试：发送内容使用 Markdown 格式化
    ///
    /// 验证通过 `text_markdown` 构造的消息内容会同时包含
    /// 纯文本正文（body）和 HTML 格式化正文（formatted_body）。
    /// 这确保支持富文本的客户端能正确渲染格式，
    /// 而不支持富文本的客户端仍能显示原始 Markdown。
    #[test]
    fn send_content_uses_markdown_formatting() {
        let content = RoomMessageEventContent::text_markdown("**hello**");
        let value = serde_json::to_value(content).unwrap();

        assert_eq!(value["msgtype"], "m.text");
        assert_eq!(value["body"], "**hello**");
        assert_eq!(value["format"], "org.matrix.custom.html");
        assert!(
            value["formatted_body"].as_str().unwrap_or_default().contains("<strong>hello</strong>")
        );
    }

    /// 测试：房间同步过滤正确定向目标房间
    ///
    /// 验证 `sync_filter_for_room` 生成的 JSON 过滤器
    /// 能正确限制同步请求仅返回指定房间的事件。
    /// 这对于只监听单个房间的高效同步至关重要。
    ///
    /// # 过滤器结构
    ///
    /// - `room.rooms`：限制为指定房间 ID
    /// - `room.timeline.limit`：限制初始时间线事件数量
    #[test]
    fn sync_filter_for_room_targets_requested_room() {
        let filter = MatrixChannel::sync_filter_for_room("!room:matrix.org", 0);
        let value: serde_json::Value = serde_json::from_str(&filter).unwrap();

        assert_eq!(value["room"]["rooms"][0], "!room:matrix.org");
        assert_eq!(value["room"]["timeline"]["limit"], 1);
    }

    /// 测试：事件 ID 缓存正确去重并淘汰旧条目
    ///
    /// 验证 `cache_event_id` 方法的事件去重和 LRU 淘汰机制：
    ///
    /// 1. **去重**：首次遇到的事件 ID 返回 `false`（未缓存过），
    ///    再次遇到相同 ID 返回 `true`（已缓存，表示重复）
    /// 2. **淘汰**：当缓存超过容量限制（约 2048 条）时，
    ///    最旧的条目被淘汰，使得之前缓存的事件 ID 可能再次被视为新事件
    ///
    /// 该机制用于防止重复处理同一消息事件。
    #[test]
    fn event_id_cache_deduplicates_and_evicts_old_entries() {
        let mut recent_order = std::collections::VecDeque::new();
        let mut recent_lookup = std::collections::HashSet::new();

        // 首次遇到事件，应返回 false（未重复）
        assert!(!MatrixChannel::cache_event_id(
            "$first:event",
            &mut recent_order,
            &mut recent_lookup
        ));
        // 再次遇到相同事件，应返回 true（已重复）
        assert!(MatrixChannel::cache_event_id(
            "$first:event",
            &mut recent_order,
            &mut recent_lookup
        ));

        // 填充超过缓存容量，触发淘汰
        for i in 0..2050 {
            let event_id = format!("$event-{i}:matrix");
            MatrixChannel::cache_event_id(&event_id, &mut recent_order, &mut recent_lookup);
        }

        // 之前缓存的事件已被淘汰，再次出现时视为新事件
        assert!(!MatrixChannel::cache_event_id(
            "$first:event",
            &mut recent_order,
            &mut recent_lookup
        ));
    }

    /// 测试：房间 ID 和允许用户列表被正确修剪
    ///
    /// 验证构造函数会自动修剪 room_id 和 allowed_users 列表中
    /// 每个用户 ID 的首尾空白，并过滤掉空白的用户条目。
    /// 这确保配置文件中的格式问题不会影响权限验证。
    #[test]
    fn trims_room_id_and_allowed_users() {
        let ch = MatrixChannel::new(
            "https://matrix.org".to_string(),
            "tok".to_string(),
            "  !room:matrix.org  ".to_string(),
            vec![
                "  @user:matrix.org  ".to_string(),
                "   ".to_string(),
                "@other:matrix.org".to_string(),
            ],
        );

        assert_eq!(ch.room_id, "!room:matrix.org");
        assert_eq!(ch.allowed_users.len(), 2);
        assert!(ch.allowed_users.contains(&"@user:matrix.org".to_string()));
        assert!(ch.allowed_users.contains(&"@other:matrix.org".to_string()));
    }

    /// 测试：通配符 "*" 允许任何用户
    ///
    /// 验证当 allowed_users 列表包含通配符 "*" 时，
    /// `is_user_allowed` 方法对任何用户 ID 都返回 true。
    /// 这用于开放通道，允许所有用户与代理交互。
    #[test]
    fn wildcard_allows_anyone() {
        let ch = MatrixChannel::new(
            "https://m.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec!["*".to_string()],
        );
        assert!(ch.is_user_allowed("@anyone:matrix.org"));
        assert!(ch.is_user_allowed("@hacker:evil.org"));
    }

    /// 测试：特定用户被允许访问
    ///
    /// 验证当用户 ID 存在于 allowed_users 列表中时，
    /// `is_user_allowed` 方法返回 true。
    #[test]
    fn specific_user_allowed() {
        let ch = make_channel();
        assert!(ch.is_user_allowed("@user:matrix.org"));
    }

    /// 测试：未知用户被拒绝访问
    ///
    /// 验证当用户 ID 不在 allowed_users 列表中时，
    /// `is_user_allowed` 方法返回 false，拒绝该用户与代理交互。
    #[test]
    fn unknown_user_denied() {
        let ch = make_channel();
        assert!(!ch.is_user_allowed("@stranger:matrix.org"));
        assert!(!ch.is_user_allowed("@evil:hacker.org"));
    }

    /// 测试：用户 ID 匹配大小写不敏感
    ///
    /// 验证 `is_user_allowed` 方法在比较用户 ID 时
    /// 不区分大小写，符合 Matrix 用户 ID 规范。
    /// 这确保配置中的大小写变体不会导致权限问题。
    #[test]
    fn user_case_insensitive() {
        let ch = MatrixChannel::new(
            "https://m.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec!["@User:Matrix.org".to_string()],
        );
        assert!(ch.is_user_allowed("@user:matrix.org"));
        assert!(ch.is_user_allowed("@USER:MATRIX.ORG"));
    }

    /// 测试：空允许列表拒绝所有用户
    ///
    /// 验证当 allowed_users 列表为空时，
    /// `is_user_allowed` 方法对所有用户都返回 false。
    /// 这是默认的安全策略，除非显式配置否则不允许任何人访问。
    #[test]
    fn empty_allowlist_denies_all() {
        let ch = MatrixChannel::new(
            "https://m.org".to_string(),
            "tok".to_string(),
            "!r:m".to_string(),
            vec![],
        );
        assert!(!ch.is_user_allowed("@anyone:matrix.org"));
    }

    /// 测试：name() 方法返回 "matrix"
    ///
    /// 验证 `MatrixChannel` 实现的 `Channel` trait 的 `name()` 方法
    /// 正确返回通道类型标识符 "matrix"。
    #[test]
    fn name_returns_matrix() {
        let ch = make_channel();
        assert_eq!(ch.name(), "matrix");
    }

    /// 测试：同步响应正确反序列化空响应
    ///
    /// 验证 `SyncResponse` 结构体能正确解析仅包含 `next_batch` token
    /// 和空房间列表的 JSON 响应。这是初始同步或无新事件时的常见场景。
    #[test]
    fn sync_response_deserializes_empty() {
        let json = r#"{"next_batch":"s123","rooms":{"join":{}}}"#;
        let resp: SyncResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.next_batch, "s123");
        assert!(resp.rooms.join.is_empty());
    }

    /// 测试：同步响应正确反序列化包含事件的响应
    ///
    /// 验证 `SyncResponse` 结构体能正确解析包含完整消息事件
    /// 的 JSON 响应，包括嵌套的房间、时间线和事件结构。
    ///
    /// # 验证字段
    ///
    /// - `next_batch`：下次同步的起始 token
    /// - `rooms.join`：已加入房间的事件
    /// - `timeline.events`：时间线事件列表
    /// - 事件内容：`sender`、`event_id`、`content.body`、`content.msgtype`
    #[test]
    fn sync_response_deserializes_with_events() {
        let json = r#"{
                "next_batch": "s456",
                "rooms": {
                    "join": {
                        "!room:matrix.org": {
                            "timeline": {
                                "events": [
                                    {
                                        "type": "m.room.message",
                                        "event_id": "$event:matrix.org",
                                        "sender": "@user:matrix.org",
                                        "content": {
                                            "msgtype": "m.text",
                                            "body": "Hello!"
                                        }
                                    }
                                ]
                            }
                        }
                    }
                }
            }"#;
        let resp: SyncResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.next_batch, "s456");
        let room = resp.rooms.join.get("!room:matrix.org").unwrap();
        assert_eq!(room.timeline.events.len(), 1);
        assert_eq!(room.timeline.events[0].sender, "@user:matrix.org");
        assert_eq!(room.timeline.events[0].event_id.as_deref(), Some("$event:matrix.org"));
        assert_eq!(room.timeline.events[0].content.body.as_deref(), Some("Hello!"));
        assert_eq!(room.timeline.events[0].content.msgtype.as_deref(), Some("m.text"));
    }

    /// 测试：同步响应忽略非文本消息事件
    ///
    /// 验证 `SyncResponse` 能正确解析非消息类型事件
    /// （如 `m.room.member` 成员变更事件），即使这些事件
    /// 没有 `body` 或 `msgtype` 字段。这确保解析器不会
    /// 因未知事件类型而失败。
    #[test]
    fn sync_response_ignores_non_text_events() {
        let json = r#"{
                "next_batch": "s789",
                "rooms": {
                    "join": {
                        "!room:m": {
                            "timeline": {
                                "events": [
                                    {
                                        "type": "m.room.member",
                                        "sender": "@user:m",
                                        "content": {}
                                    }
                                ]
                            }
                        }
                    }
                }
            }"#;
        let resp: SyncResponse = serde_json::from_str(json).unwrap();
        let room = resp.rooms.join.get("!room:m").unwrap();
        assert_eq!(room.timeline.events[0].event_type, "m.room.member");
        assert!(room.timeline.events[0].content.body.is_none());
    }

    /// 测试：WhoAmI 响应正确反序列化
    ///
    /// 验证 `WhoAmIResponse` 结构体能正确解析 Matrix 的
    /// `/whoami` API 响应，提取当前认证用户的 Matrix ID。
    /// 该端点用于验证访问令牌的有效性和归属。
    #[test]
    fn whoami_response_deserializes() {
        let json = r#"{"user_id":"@bot:matrix.org"}"#;
        let resp: WhoAmIResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.user_id, "@bot:matrix.org");
    }

    /// 测试：事件内容字段使用默认值
    ///
    /// 验证 `TimelineEvent` 和 `EventContent` 结构体在
    /// JSON 中缺少可选字段时能正确使用默认值。
    /// 这确保解析器对不完整的事件数据具有容错性。
    #[test]
    fn event_content_defaults() {
        let json = r#"{"type":"m.room.message","sender":"@u:m","content":{}}"#;
        let event: TimelineEvent = serde_json::from_str(json).unwrap();
        assert!(event.content.body.is_none());
        assert!(event.content.msgtype.is_none());
    }

    /// 测试：事件内容支持 m.notice 消息类型
    ///
    /// 验证 `EventContent` 能正确解析 `m.notice` 类型的消息。
    /// `m.notice` 是 Matrix 中用于机器人消息的特殊类型，
    /// 与 `m.text` 的区别在于客户端通常不会触发通知提醒。
    #[test]
    fn event_content_supports_notice_msgtype() {
        let json = r#"{
                "type":"m.room.message",
                "sender":"@u:m",
                "event_id":"$notice:m",
                "content":{"msgtype":"m.notice","body":"Heads up"}
            }"#;
        let event: TimelineEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.content.msgtype.as_deref(), Some("m.notice"));
        assert_eq!(event.content.body.as_deref(), Some("Heads up"));
        assert_eq!(event.event_id.as_deref(), Some("$notice:m"));
    }

    /// 测试：无效房间引用快速失败
    ///
    /// 验证 `resolve_room_id` 方法在房间标识符既不是以 "!" 开头的房间 ID
    /// 也不是以 "#" 开头的房间别名时，会立即返回明确的错误信息。
    /// 这提供了早期验证，避免后续 API 调用失败。
    ///
    /// # 异步说明
    ///
    /// 该测试使用 `#[tokio::test]` 因为 `resolve_room_id` 是异步方法，
    /// 即使该测试场景不需要实际的网络请求。
    #[tokio::test]
    async fn invalid_room_reference_fails_fast() {
        let ch = MatrixChannel::new(
            "https://matrix.org".to_string(),
            "tok".to_string(),
            "room_without_prefix".to_string(),
            vec![],
        );

        let err = ch.resolve_room_id().await.unwrap_err();
        assert!(err.to_string().contains("must start with '!' (room ID) or '#' (room alias)"));
    }

    /// 测试：目标房间 ID 保留规范房间 ID 无需查询
    ///
    /// 验证 `target_room_id` 方法在房间标识符已经是规范的房间 ID
    /// （以 "!" 开头）时，直接返回该 ID 而不进行网络查询。
    /// 这优化了性能，避免不必要的 API 调用。
    #[tokio::test]
    async fn target_room_id_keeps_canonical_room_id_without_lookup() {
        let ch = MatrixChannel::new(
            "https://matrix.org".to_string(),
            "tok".to_string(),
            "!canonical:matrix.org".to_string(),
            vec![],
        );

        let room_id = ch.target_room_id().await.unwrap();
        assert_eq!(room_id, "!canonical:matrix.org");
    }

    /// 测试：目标房间 ID 使用缓存的别名解析结果
    ///
    /// 验证 `target_room_id` 方法在处理房间别名（以 "#" 开头）时，
    /// 会优先使用缓存的解析结果，避免重复的网络查询。
    /// 缓存通过 `resolved_room_id_cache` 字段存储。
    ///
    /// # 缓存机制
    ///
    /// 1. 首次调用时通过 API 将别名解析为房间 ID 并缓存
    /// 2. 后续调用直接返回缓存的房间 ID
    /// 3. 缓存在通道生命周期内持续有效
    #[tokio::test]
    async fn target_room_id_uses_cached_alias_resolution() {
        let ch = MatrixChannel::new(
            "https://matrix.org".to_string(),
            "tok".to_string(),
            "#ops:matrix.org".to_string(),
            vec![],
        );

        // 预填充缓存
        *ch.resolved_room_id_cache.write().await = Some("!cached:matrix.org".to_string());
        let room_id = ch.target_room_id().await.unwrap();
        assert_eq!(room_id, "!cached:matrix.org");
    }

    /// 测试：同步响应缺少 rooms 字段时使用默认值
    ///
    /// 验证 `SyncResponse` 结构体在 JSON 响应中缺少 `rooms` 字段时，
    /// 能正确使用默认的空房间映射。这确保解析器对最小化响应具有容错性。
    #[test]
    fn sync_response_missing_rooms_defaults() {
        let json = r#"{"next_batch":"s0"}"#;
        let resp: SyncResponse = serde_json::from_str(json).unwrap();
        assert!(resp.rooms.join.is_empty());
    }
}
