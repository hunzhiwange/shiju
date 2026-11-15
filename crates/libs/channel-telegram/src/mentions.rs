//! Telegram @提及解析模块。
//!
//! 本模块负责识别、匹配和移除消息文本中的 bot 用户名提及。
//! Telegram 用户名只允许 ASCII 字母、数字和下划线，因此这里按平台规则做边界判断，
//! 防止把邮箱、普通文本片段或更长用户名的一部分误识别为有效提及。

use super::TelegramChannel;

impl TelegramChannel {
    /// 判断字符是否属于 Telegram 用户名允许字符集。
    ///
    /// # 参数
    /// - `ch`: 待检查字符。
    ///
    /// # 返回值
    /// ASCII 字母、数字或下划线返回 `true`。
    pub(super) fn is_telegram_username_char(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || ch == '_'
    }

    /// 查找文本中匹配 bot 用户名的提及范围。
    ///
    /// # 参数
    /// - `text`: 待扫描的消息文本。
    /// - `bot_username`: bot 用户名，可带或不带 `@` 前缀。
    ///
    /// # 返回值
    /// 返回所有匹配提及在原字符串中的字节范围 `(start, end)`。
    ///
    /// # 说明
    /// 匹配大小写不敏感，并要求 `@` 前一字符不是用户名字符，
    /// 以避免把 `foo@bot` 或 `@bot_suffix` 这类片段误判为独立提及。
    pub(super) fn find_bot_mention_spans(text: &str, bot_username: &str) -> Vec<(usize, usize)> {
        let bot_username = bot_username.trim_start_matches('@');
        if bot_username.is_empty() {
            return Vec::new();
        }

        let mut spans = Vec::new();

        for (at_idx, ch) in text.char_indices() {
            if ch != '@' {
                continue;
            }

            if at_idx > 0 {
                let prev = text[..at_idx].chars().next_back().unwrap_or(' ');
                if Self::is_telegram_username_char(prev) {
                    // @ 前仍是用户名字符时不是独立 mention，跳过可避免误删用户输入。
                    continue;
                }
            }

            let username_start = at_idx + 1;
            let mut username_end = username_start;

            for (rel_idx, candidate_ch) in text[username_start..].char_indices() {
                if Self::is_telegram_username_char(candidate_ch) {
                    username_end = username_start + rel_idx + candidate_ch.len_utf8();
                } else {
                    break;
                }
            }

            if username_end == username_start {
                continue;
            }

            let mention_username = &text[username_start..username_end];
            if mention_username.eq_ignore_ascii_case(bot_username) {
                spans.push((at_idx, username_end));
            }
        }

        spans
    }

    /// 判断文本是否包含 bot 提及。
    ///
    /// # 参数
    /// - `text`: 待检查文本。
    /// - `bot_username`: bot 用户名，可带或不带 `@` 前缀。
    ///
    /// # 返回值
    /// 至少存在一个有效提及时返回 `true`。
    pub(super) fn contains_bot_mention(text: &str, bot_username: &str) -> bool {
        !Self::find_bot_mention_spans(text, bot_username).is_empty()
    }

    /// 移除文本中的 bot 提及并压缩空白。
    ///
    /// # 参数
    /// - `text`: 原始消息文本。
    /// - `bot_username`: bot 用户名，可带或不带 `@` 前缀。
    ///
    /// # 返回值
    /// 返回清理后的非空文本；清理后为空时返回 `None`。
    ///
    /// # 说明
    /// 即使没有提及，也会执行空白标准化，便于调用方在不同入口复用同一规范化行为。
    pub(super) fn normalize_incoming_content(text: &str, bot_username: &str) -> Option<String> {
        let spans = Self::find_bot_mention_spans(text, bot_username);
        if spans.is_empty() {
            let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
            return (!normalized.is_empty()).then_some(normalized);
        }

        let mut normalized = String::with_capacity(text.len());
        let mut cursor = 0;
        for (start, end) in spans {
            normalized.push_str(&text[cursor..start]);
            cursor = end;
        }
        normalized.push_str(&text[cursor..]);

        let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
        (!normalized.is_empty()).then_some(normalized)
    }
}
