use super::MatrixChannel;
use matrix_sdk::ruma::{
    events::Mentions,
    events::room::message::{MessageType, OriginalSyncRoomMessageEvent, Relation},
};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use vw_config_types::channels::chat::DEFAULT_CHANNEL_INSTANCE_ID;

impl MatrixChannel {
    /// 清理错误日志，避免泄露敏感信息
    pub(crate) fn sanitize_error_for_log(error: &impl std::fmt::Display) -> String {
        let error_type = std::any::type_name_of_val(error);
        format!("{error_type} (details redacted)")
    }

    /// 检查错误消息是否为一次性密钥冲突
    pub(crate) fn is_otk_conflict_message(message: &str) -> bool {
        let lower = message.to_ascii_lowercase();
        lower.contains("one time key") && lower.contains("already exists")
    }

    /// 生成一次性密钥冲突恢复提示消息
    pub(crate) fn otk_conflict_recovery_message(&self) -> String {
        let mut message = String::from(
            "Matrix E2EE one-time key upload conflict detected (`one time key ... already exists`). \
            VibeWindow paused Matrix sync to avoid an infinite retry loop. \
            Resolve by deregistering the stale Matrix device for this bot account, resetting the local Matrix crypto store, then restarting VibeWindow.",
        );
        if let Some(store_dir) = self.matrix_store_dir() {
            message.push_str(&format!(" Local crypto store: {}", store_dir.display()));
        }
        message
    }

    /// 标准化可选字段
    fn normalize_optional_field(value: Option<String>) -> Option<String> {
        value.map(|entry| entry.trim().to_string()).filter(|entry| !entry.is_empty())
    }

    pub(crate) fn normalize_room_reference(value: &str) -> String {
        let trimmed = value.trim();
        let permalink = trimmed
            .strip_prefix("https://matrix.to/#/")
            .or_else(|| trimmed.strip_prefix("http://matrix.to/#/"));
        let reference = if let Some(reference) = permalink {
            let reference = reference.split_once('?').map(|(head, _)| head).unwrap_or(reference);
            Self::percent_decode(reference)
        } else {
            trimmed.to_string()
        };

        if Self::looks_like_bare_room_alias(&reference) {
            format!("#{reference}")
        } else {
            reference
        }
    }

    fn looks_like_bare_room_alias(value: &str) -> bool {
        let Some((localpart, server_name)) = value.split_once(':') else {
            return false;
        };
        if localpart.is_empty() || server_name.is_empty() {
            return false;
        }
        if matches!(value.as_bytes().first(), Some(b'!' | b'#' | b'@' | b'$')) {
            return false;
        }

        !value.contains('/') && !value.contains('?') && !value.chars().any(char::is_whitespace)
    }

    fn percent_decode(value: &str) -> String {
        fn hex_value(byte: u8) -> Option<u8> {
            match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                b'A'..=b'F' => Some(byte - b'A' + 10),
                _ => None,
            }
        }

        let bytes = value.as_bytes();
        let mut decoded = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'%'
                && index + 2 < bytes.len()
                && let (Some(high), Some(low)) =
                    (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
            {
                decoded.push((high << 4) | low);
                index += 3;
            } else {
                decoded.push(bytes[index]);
                index += 1;
            }
        }

        String::from_utf8_lossy(&decoded).into_owned()
    }

    /// 创建新的 Matrix 频道实例
    pub fn new(
        homeserver: String,
        access_token: String,
        room_id: String,
        allowed_users: Vec<String>,
    ) -> Self {
        Self::new_with_session_hint(homeserver, access_token, room_id, allowed_users, None, None)
    }

    /// 创建带有会话提示的 Matrix 频道实例
    pub fn new_with_session_hint(
        homeserver: String,
        access_token: String,
        room_id: String,
        allowed_users: Vec<String>,
        owner_hint: Option<String>,
        device_id_hint: Option<String>,
    ) -> Self {
        Self::new_with_session_hint_and_vibewindow_dir(
            homeserver,
            access_token,
            room_id,
            allowed_users,
            owner_hint,
            device_id_hint,
            None,
        )
    }

    /// 创建带有会话提示和数据目录的 Matrix 频道实例
    pub fn new_with_session_hint_and_vibewindow_dir(
        homeserver: String,
        access_token: String,
        room_id: String,
        allowed_users: Vec<String>,
        owner_hint: Option<String>,
        device_id_hint: Option<String>,
        vibewindow_dir: Option<PathBuf>,
    ) -> Self {
        Self::new_with_session_hint_instance_id_and_vibewindow_dir(
            homeserver,
            access_token,
            room_id,
            allowed_users,
            owner_hint,
            device_id_hint,
            None,
            vibewindow_dir,
        )
    }

    /// 创建带有会话提示、实例 ID 和数据目录的 Matrix 频道实例
    pub fn new_with_session_hint_instance_id_and_vibewindow_dir(
        homeserver: String,
        access_token: String,
        room_id: String,
        allowed_users: Vec<String>,
        owner_hint: Option<String>,
        device_id_hint: Option<String>,
        instance_id: Option<String>,
        vibewindow_dir: Option<PathBuf>,
    ) -> Self {
        let homeserver = homeserver.trim_end_matches('/').to_string();
        let access_token = access_token.trim().to_string();
        let room_id = Self::normalize_room_reference(&room_id);
        let allowed_users = allowed_users
            .into_iter()
            .map(|user| user.trim().to_string())
            .filter(|user| !user.is_empty())
            .collect();

        Self {
            homeserver,
            access_token,
            room_id,
            allowed_users,
            mention_only: false,
            session_owner_hint: Self::normalize_optional_field(owner_hint),
            session_device_id_hint: Self::normalize_optional_field(device_id_hint),
            instance_id: Self::normalize_optional_field(instance_id),
            vibewindow_dir,
            resolved_room_id_cache: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            sdk_client: std::sync::Arc::new(tokio::sync::OnceCell::new()),
            otk_conflict_detected: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            http_client: reqwest::Client::new(),
        }
    }

    /// 配置是否仅响应提及消息
    pub fn with_mention_only(mut self, mention_only: bool) -> Self {
        self.mention_only = mention_only;
        self
    }

    /// URL 编码路径段
    pub(crate) fn encode_path_segment(value: &str) -> String {
        fn should_encode(byte: u8) -> bool {
            !matches!(
                byte,
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~'
            )
        }

        let mut encoded = String::with_capacity(value.len());
        for byte in value.bytes() {
            if should_encode(byte) {
                use std::fmt::Write;
                let _ = write!(&mut encoded, "%{byte:02X}");
            } else {
                encoded.push(byte as char);
            }
        }

        encoded
    }

    /// 构建 Authorization 头部值
    pub(crate) fn auth_header_value(&self) -> String {
        format!("Bearer {}", self.access_token)
    }

    fn matrix_store_instance_dir(&self) -> Option<PathBuf> {
        self.vibewindow_dir.as_ref().map(|dir| {
            let matrix_dir = dir.join("state").join("matrix");
            match self.instance_id.as_deref() {
                Some(instance_id) if instance_id != DEFAULT_CHANNEL_INSTANCE_ID => {
                    matrix_dir.join(Self::encode_path_segment(instance_id))
                }
                _ => matrix_dir,
            }
        })
    }

    pub(crate) fn matrix_store_dir_for_user_id(&self, user_id: &str) -> Option<PathBuf> {
        let store_dir = self.matrix_store_instance_dir()?;
        let user_id = user_id.trim();
        if user_id.is_empty() {
            return Some(store_dir);
        }

        Some(store_dir.join(Self::encode_path_segment(user_id)))
    }

    /// 获取 Matrix 加密状态存储目录
    pub(crate) fn matrix_store_dir(&self) -> Option<PathBuf> {
        match self.session_owner_hint.as_deref() {
            Some(user_id) => self.matrix_store_dir_for_user_id(user_id),
            None => self.matrix_store_instance_dir(),
        }
    }

    /// 检查用户是否在允许列表中
    pub(crate) fn is_user_allowed(&self, sender: &str) -> bool {
        Self::is_sender_allowed(&self.allowed_users, sender)
    }

    /// 检查发送者是否在允许列表中
    pub(crate) fn is_sender_allowed(allowed_users: &[String], sender: &str) -> bool {
        if allowed_users.iter().any(|u| u == "*") {
            return true;
        }

        allowed_users.iter().any(|u| u.eq_ignore_ascii_case(sender))
    }

    /// 检查消息类型是否受支持
    pub(crate) fn is_supported_message_type(msgtype: &str) -> bool {
        matches!(msgtype, "m.text" | "m.notice" | "m.audio")
    }

    /// 检查消息体是否非空
    pub(crate) fn has_non_empty_body(body: &str) -> bool {
        !body.trim().is_empty()
    }

    /// 判断字符是否为 Matrix 标识符有效字符
    fn is_matrix_identifier_char(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')
    }

    /// 检查文本中是否包含 Matrix 用户 ID 提及
    fn contains_matrix_user_id_mention(text: &str, user_id: &str) -> bool {
        if text.is_empty() || user_id.is_empty() {
            return false;
        }

        let text_lower = text.to_ascii_lowercase();
        let user_id_lower = user_id.to_ascii_lowercase();
        let mut search_from = 0;

        while let Some(found) = text_lower[search_from..].find(&user_id_lower) {
            let start = search_from + found;
            let end = start + user_id_lower.len();
            let before = text[..start].chars().next_back();
            let after = text[end..].chars().next();
            let left_ok = before.is_none_or(|c| !Self::is_matrix_identifier_char(c));
            let right_ok = after.is_none_or(|c| !Self::is_matrix_identifier_char(c));

            if left_ok && right_ok {
                return true;
            }

            search_from = end;
        }

        false
    }

    /// 对字符串进行百分号编码
    fn percent_encode(input: &str) -> String {
        let mut encoded = String::with_capacity(input.len());
        for byte in input.bytes() {
            if matches!(
                byte,
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~'
            ) {
                encoded.push(char::from(byte));
            } else {
                use std::fmt::Write;
                let _ = write!(&mut encoded, "%{byte:02X}");
            }
        }
        encoded
    }

    /// 检查结构化提及中是否包含机器人用户
    fn has_structured_mention(mentions: Option<&Mentions>, bot_user_id: &str) -> bool {
        mentions.is_some_and(|m| {
            m.user_ids.iter().any(|user_id| user_id.as_str().eq_ignore_ascii_case(bot_user_id))
        })
    }

    /// 从消息类型中提取格式化消息体
    fn extract_formatted_body(msgtype: &MessageType) -> Option<&str> {
        match msgtype {
            MessageType::Text(content) => content.formatted.as_ref().map(|f| f.body.as_str()),
            MessageType::Notice(content) => content.formatted.as_ref().map(|f| f.body.as_str()),
            MessageType::Emote(content) => content.formatted.as_ref().map(|f| f.body.as_str()),
            _ => None,
        }
    }

    /// 检查事件是否提及了指定用户
    pub(crate) fn event_mentions_user(
        event: &OriginalSyncRoomMessageEvent,
        plain_body: &str,
        bot_user_id: &str,
    ) -> bool {
        if Self::has_structured_mention(event.content.mentions.as_ref(), bot_user_id) {
            return true;
        }

        if Self::contains_matrix_user_id_mention(plain_body, bot_user_id) {
            return true;
        }

        let Some(formatted_body) = Self::extract_formatted_body(&event.content.msgtype) else {
            return false;
        };

        if Self::contains_matrix_user_id_mention(formatted_body, bot_user_id) {
            return true;
        }

        let encoded_user_id = Self::percent_encode(bot_user_id).to_ascii_lowercase();
        formatted_body.to_ascii_lowercase().contains(&encoded_user_id)
    }

    /// 从事件中提取回复目标事件 ID
    pub(crate) fn reply_target_event_id(event: &OriginalSyncRoomMessageEvent) -> Option<String> {
        match event.content.relates_to.as_ref()? {
            Relation::Reply { in_reply_to } => Some(in_reply_to.event_id.to_string()),
            Relation::Thread(thread) => thread.in_reply_to.as_ref().map(|r| r.event_id.to_string()),
            Relation::Replacement(_) | Relation::_Custom(_) => None,
            _ => None,
        }
    }

    /// 判断是否应该处理消息
    pub(crate) fn should_process_message(
        mention_only: bool,
        is_direct_room: bool,
        is_mentioned: bool,
        is_reply_to_bot: bool,
    ) -> bool {
        if !mention_only {
            return true;
        }

        is_direct_room || is_mentioned || is_reply_to_bot
    }

    /// 缓存事件 ID 并检查是否已存在
    pub(crate) fn cache_event_id(
        event_id: &str,
        recent_order: &mut VecDeque<String>,
        recent_lookup: &mut HashSet<String>,
    ) -> bool {
        const MAX_RECENT_EVENT_IDS: usize = 2048;

        if recent_lookup.contains(event_id) {
            return true;
        }

        let event_id_owned = event_id.to_string();
        recent_lookup.insert(event_id_owned.clone());
        recent_order.push_back(event_id_owned);

        if recent_order.len() > MAX_RECENT_EVENT_IDS {
            if let Some(evicted) = recent_order.pop_front() {
                recent_lookup.remove(&evicted);
            }
        }

        false
    }

    /// 生成同步过滤器 JSON
    pub(crate) fn sync_filter_for_room(room_id: &str, timeline_limit: usize) -> String {
        let timeline_limit = timeline_limit.max(1);
        serde_json::json!({
            "room": {
                "rooms": [room_id],
                "timeline": {
                    "limit": timeline_limit
                }
            }
        })
        .to_string()
    }
}

#[cfg(test)]
#[path = "core_tests.rs"]
mod core_tests;
