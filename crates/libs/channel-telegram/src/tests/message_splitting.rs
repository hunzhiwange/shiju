//! Telegram 消息分片测试模块。
//!
//! 本模块验证 `split_message_for_telegram` 在 Telegram 单条消息长度限制下的行为，
//! 覆盖短消息、边界长度、超长文本、换行/空格优先分割、长单词硬切分和
//! UTF-8 字符边界等场景，确保发送层可以安全地逐片发送。

use super::*;

/// 短消息应保持单片输出且内容不变。
#[test]
fn telegram_split_short_message() {
    let msg = "Hello, world!";
    let chunks = split_message_for_telegram(msg);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], msg);
}

/// 正好等于 Telegram 长度上限的消息不应被拆分。
#[test]
fn telegram_split_exact_limit() {
    let msg = "a".repeat(TELEGRAM_MAX_MESSAGE_LENGTH);
    let chunks = split_message_for_telegram(&msg);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].len(), TELEGRAM_MAX_MESSAGE_LENGTH);
}

/// 超过长度上限的消息应拆成多个不超过限制的片段。
#[test]
fn telegram_split_over_limit() {
    let msg = "a".repeat(TELEGRAM_MAX_MESSAGE_LENGTH + 100);
    let chunks = split_message_for_telegram(&msg);
    assert_eq!(chunks.len(), 2);
    assert!(chunks[0].len() <= TELEGRAM_MAX_MESSAGE_LENGTH);
    assert!(chunks[1].len() <= TELEGRAM_MAX_MESSAGE_LENGTH);
}

/// 分片应优先使用单词边界，保持文本可读性。
#[test]
fn telegram_split_at_word_boundary() {
    let msg = format!("{} more text here", "word ".repeat(TELEGRAM_MAX_MESSAGE_LENGTH / 5));
    let chunks = split_message_for_telegram(&msg);
    assert!(chunks.len() >= 2);
    for chunk in &chunks[..chunks.len() - 1] {
        assert!(chunk.len() <= TELEGRAM_MAX_MESSAGE_LENGTH);
    }
}

/// 分片应优先使用换行边界，避免打断多行文本结构。
#[test]
fn telegram_split_at_newline() {
    let text_block = "Line of text\n".repeat(TELEGRAM_MAX_MESSAGE_LENGTH / 13 + 1);
    let chunks = split_message_for_telegram(&text_block);
    assert!(chunks.len() >= 2);
    for chunk in chunks {
        assert!(chunk.len() <= TELEGRAM_MAX_MESSAGE_LENGTH);
    }
}

/// 分片后重新拼接应完整保留原始内容。
#[test]
fn telegram_split_preserves_content() {
    let msg = "test ".repeat(TELEGRAM_MAX_MESSAGE_LENGTH / 5 + 100);
    let chunks = split_message_for_telegram(&msg);
    let rejoined = chunks.join("");
    assert_eq!(rejoined, msg);
}

/// 空消息保持为空片段，便于调用方统一处理返回值。
#[test]
fn telegram_split_empty_message() {
    let chunks = split_message_for_telegram("");
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], "");
}

/// 多倍超过上限的消息应被持续拆分到每片都合法。
#[test]
fn telegram_split_very_long_message() {
    let msg = "x".repeat(TELEGRAM_MAX_MESSAGE_LENGTH * 3);
    let chunks = split_message_for_telegram(&msg);
    assert!(chunks.len() >= 3);
    for chunk in chunks {
        assert!(chunk.len() <= TELEGRAM_MAX_MESSAGE_LENGTH);
    }
}

/// 跨越边界的代码块也必须拆分为符合长度限制的片段。
#[test]
fn telegram_split_code_block_at_boundary() {
    let mut msg = String::new();
    msg.push_str("```python\n");
    msg.push_str(&"x".repeat(4085));
    msg.push_str("\n```\nMore text after code block");
    let parts = split_message_for_telegram(&msg);
    assert!(parts.len() >= 2, "code block spanning boundary should split");
    for part in &parts {
        assert!(
            part.len() <= TELEGRAM_MAX_MESSAGE_LENGTH,
            "each part must be <= {TELEGRAM_MAX_MESSAGE_LENGTH}, got {}",
            part.len()
        );
    }
}

/// 单个超长单词没有自然边界时应硬切分，并保持可重组。
#[test]
fn telegram_split_single_long_word() {
    let long_word = "a".repeat(5000);
    let parts = split_message_for_telegram(&long_word);
    assert!(parts.len() >= 2, "word exceeding limit must be split");
    for part in &parts {
        assert!(
            part.len() <= TELEGRAM_MAX_MESSAGE_LENGTH,
            "hard-split part must be <= {TELEGRAM_MAX_MESSAGE_LENGTH}, got {}",
            part.len()
        );
    }
    let reassembled: String = parts.join("");
    assert_eq!(reassembled, long_word);
}

/// 再次锁定精确上限场景，防止边界判断回归。
#[test]
fn telegram_split_exactly_at_limit_no_split() {
    let msg = "a".repeat(TELEGRAM_MAX_MESSAGE_LENGTH);
    let parts = split_message_for_telegram(&msg);
    assert_eq!(parts.len(), 1, "message exactly at limit should not split");
}

/// 只超过一个字符也必须拆分，避免 Telegram API 拒收。
#[test]
fn telegram_split_one_over_limit() {
    let msg = "a".repeat(TELEGRAM_MAX_MESSAGE_LENGTH + 1);
    let parts = split_message_for_telegram(&msg);
    assert!(parts.len() >= 2, "message 1 char over limit must split");
}

/// 大量短行应按批次合并，但每片仍不超过限制。
#[test]
fn telegram_split_many_short_lines() {
    let msg: String = (0..1_000).map(|i| format!("line {i}\n")).collect::<Vec<_>>().concat();
    let parts = split_message_for_telegram(&msg);
    for part in &parts {
        assert!(part.len() <= TELEGRAM_MAX_MESSAGE_LENGTH, "short-line batch must be <= limit");
    }
}

/// 纯空白输入不应产生多余片段。
#[test]
fn telegram_split_only_whitespace() {
    let msg = "   \n\n\t  ";
    let parts = split_message_for_telegram(msg);
    assert!(parts.len() <= 1);
}

/// Emoji 等多字节字符位于边界附近时不应被切坏。
#[test]
fn telegram_split_emoji_at_boundary() {
    let mut msg = "a".repeat(4094);
    msg.push_str("🎉🎊");
    let parts = split_message_for_telegram(&msg);
    for part in &parts {
        assert!(
            part.chars().count() <= TELEGRAM_MAX_MESSAGE_LENGTH,
            "emoji boundary split must respect limit"
        );
    }
}

/// 连续换行靠近边界时仍应保持片段长度合法。
#[test]
fn telegram_split_consecutive_newlines() {
    let mut msg = "a".repeat(4090);
    msg.push_str("\n\n\n\n\n\n");
    msg.push_str(&"b".repeat(100));
    let parts = split_message_for_telegram(&msg);
    for part in &parts {
        assert!(part.len() <= TELEGRAM_MAX_MESSAGE_LENGTH);
    }
}
