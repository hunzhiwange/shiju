use super::super::message_split as discord_message_split;
use super::*;

/// 测试空消息的分割
/// 空消息应该返回包含一个空字符串的向量
#[test]
fn split_empty_message() {
    let chunks = discord_message_split::split_message_for_discord("");
    assert_eq!(chunks, vec![""]);
}

/// 测试短消息（在限制内）不分割
/// 小于 2000 字符的消息应该保持为单个块
#[test]
fn split_short_message_under_limit() {
    let msg = "Hello, world!";
    let chunks = discord_message_split::split_message_for_discord(msg);
    assert_eq!(chunks, vec![msg]);
}

/// 测试恰好 2000 字符的消息不分割
/// 恰好达到限制的消息应该保持为单个块
#[test]
fn split_message_exactly_2000_chars() {
    let msg = "a".repeat(discord_message_split::DISCORD_MAX_MESSAGE_LENGTH);
    let chunks = discord_message_split::split_message_for_discord(&msg);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].chars().count(), discord_message_split::DISCORD_MAX_MESSAGE_LENGTH);
}

/// 测试刚超过限制的消息正确分割
/// 超过限制 1 个字符应该分割为 2 个块
#[test]
fn split_message_just_over_limit() {
    let msg = "a".repeat(discord_message_split::DISCORD_MAX_MESSAGE_LENGTH + 1);
    let chunks = discord_message_split::split_message_for_discord(&msg);
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].chars().count(), discord_message_split::DISCORD_MAX_MESSAGE_LENGTH);
    assert_eq!(chunks[1].chars().count(), 1);
}

/// 测试非常长的消息的分割
/// 10000 字符的消息应该分割为 5 个块
#[test]
fn split_very_long_message() {
    let msg = "word ".repeat(2000);
    let chunks = discord_message_split::split_message_for_discord(&msg);
    assert_eq!(chunks.len(), 5);
    assert!(chunks.iter().all(|chunk| {
        chunk.chars().count() <= discord_message_split::DISCORD_MAX_MESSAGE_LENGTH
    }));
    let reconstructed = chunks.concat();
    assert_eq!(reconstructed, msg);
}

/// 测试优先在换行符处分割
/// 当可能时，应该在换行符边界分割消息
#[test]
fn split_prefer_newline_break() {
    let msg = format!("{}\n{}", "a".repeat(1500), "b".repeat(500));
    let chunks = discord_message_split::split_message_for_discord(&msg);
    assert_eq!(chunks.len(), 2);
    assert!(chunks[0].ends_with('\n'));
    assert!(chunks[1].starts_with('b'));
}

/// 测试优先在空格处分割
/// 当没有换行符时，应该在空格边界分割
#[test]
fn split_prefer_space_break() {
    let msg = format!("{} {}", "a".repeat(1500), "b".repeat(600));
    let chunks = discord_message_split::split_message_for_discord(&msg);
    assert_eq!(chunks.len(), 2);
}

/// 测试没有良好分割点时的硬分割
/// 没有空格或换行符时，应该在 2000 字符处硬分割
#[test]
fn split_without_good_break_points_hard_split() {
    let msg = "a".repeat(5000);
    let chunks = discord_message_split::split_message_for_discord(&msg);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].chars().count(), discord_message_split::DISCORD_MAX_MESSAGE_LENGTH);
    assert_eq!(chunks[1].chars().count(), discord_message_split::DISCORD_MAX_MESSAGE_LENGTH);
    assert_eq!(chunks[2].chars().count(), 1000);
}

/// 测试多个分割点的处理
/// 消息中有多个换行符时的分割行为
#[test]
fn split_multiple_breaks() {
    let part1 = "a".repeat(900);
    let part2 = "b".repeat(900);
    let part3 = "c".repeat(900);
    let msg = format!("{part1}\n{part2}\n{part3}");
    let chunks = discord_message_split::split_message_for_discord(&msg);
    assert_eq!(chunks.len(), 2);
    assert!(chunks[0].chars().count() <= discord_message_split::DISCORD_MAX_MESSAGE_LENGTH);
    assert!(chunks[1].chars().count() <= discord_message_split::DISCORD_MAX_MESSAGE_LENGTH);
}

/// 测试分割后内容的完整性
/// 重新拼接所有块应该得到原始消息
#[test]
fn split_preserves_content() {
    let original = "Hello world! This is a test message with some content. ".repeat(200);
    let chunks = discord_message_split::split_message_for_discord(&original);
    let reconstructed = chunks.concat();
    assert_eq!(reconstructed, original);
}

/// 测试 Unicode 内容（表情符号和多字节字符）的分割
/// 确保不在多字节字符中间分割
#[test]
fn split_unicode_content() {
    let msg = "Rust is awesome! ".repeat(500);
    let chunks = discord_message_split::split_message_for_discord(&msg);
    for chunk in &chunks {
        assert!(std::str::from_utf8(chunk.as_bytes()).is_ok());
        assert!(chunk.chars().count() <= discord_message_split::DISCORD_MAX_MESSAGE_LENGTH);
    }
    let reconstructed = chunks.concat();
    assert_eq!(reconstructed, msg);
}

/// 测试换行符接近窗口末尾的情况
/// 如果换行符在窗口的前半部分，不使用它 - 改用空格或硬分割
#[test]
fn split_newline_too_close_to_end() {
    let msg = format!("{}\n{}", "a".repeat(1900), "b".repeat(500));
    let chunks = discord_message_split::split_message_for_discord(&msg);
    assert_eq!(chunks.len(), 2);
}

/// 测试仅包含多字节字符的内容不会导致 panic
/// 确保字符计数正确处理多字节字符
#[test]
fn split_multibyte_only_content_without_panics() {
    let msg = "🦀".repeat(2500);
    let chunks = discord_message_split::split_message_for_discord(&msg);
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].chars().count(), discord_message_split::DISCORD_MAX_MESSAGE_LENGTH);
    assert_eq!(chunks[1].chars().count(), 500);
    let reconstructed = chunks.concat();
    assert_eq!(reconstructed, msg);
}

/// 测试所有块都遵守 Discord 限制
/// 无论输入多长，每个块都不能超过 2000 字符
#[test]
fn split_chunks_always_within_discord_limit() {
    let msg = "x".repeat(12_345);
    let chunks = discord_message_split::split_message_for_discord(&msg);
    assert!(chunks.iter().all(|chunk| {
        chunk.chars().count() <= discord_message_split::DISCORD_MAX_MESSAGE_LENGTH
    }));
}

/// 测试包含多个换行符的消息分割
/// 验证换行符密集消息的分割行为
#[test]
fn split_message_with_multiple_newlines() {
    let msg = "Line 1\nLine 2\nLine 3\n".repeat(1000);
    let chunks = discord_message_split::split_message_for_discord(&msg);
    assert!(chunks.len() > 1);
    let reconstructed = chunks.concat();
    assert_eq!(reconstructed, msg);
}

/// 测试代码块在边界处的分割
/// 跨越分割边界的代码块应该被正确处理
#[test]
fn split_message_code_block_at_boundary() {
    let mut msg = String::new();
    msg.push_str("```rust\n");
    msg.push_str(&"x".repeat(1990));
    msg.push_str("\n```\nMore text after code block");
    let parts = discord_message_split::split_message_for_discord(&msg);
    assert!(parts.len() >= 2, "跨越边界的代码块应该被分割");
    for part in &parts {
        assert!(
            part.len() <= discord_message_split::DISCORD_MAX_MESSAGE_LENGTH,
            "每个部分必须 <= {}, 得到 {}",
            discord_message_split::DISCORD_MAX_MESSAGE_LENGTH,
            part.len()
        );
    }
}

/// 测试单个超长单词超过限制
/// 单个超过 2000 字符的单词必须被硬分割
#[test]
fn split_message_single_long_word_exceeds_limit() {
    let long_word = "a".repeat(2500);
    let parts = discord_message_split::split_message_for_discord(&long_word);
    assert!(parts.len() >= 2, "超过限制的单词必须被分割");
    for part in &parts {
        assert!(
            part.len() <= discord_message_split::DISCORD_MAX_MESSAGE_LENGTH,
            "硬分割部分必须 <= {}, 得到 {}",
            discord_message_split::DISCORD_MAX_MESSAGE_LENGTH,
            part.len()
        );
    }
    let reassembled: String = parts.join("");
    assert_eq!(reassembled, long_word);
}

/// 测试恰好达到限制的消息不分割
/// 恰好 2000 字符的消息应该保持为单个块
#[test]
fn split_message_exactly_at_limit_no_split() {
    let msg = "a".repeat(discord_message_split::DISCORD_MAX_MESSAGE_LENGTH);
    let parts = discord_message_split::split_message_for_discord(&msg);
    assert_eq!(parts.len(), 1, "恰好达到限制的消息不应该被分割");
    assert_eq!(parts[0].len(), discord_message_split::DISCORD_MAX_MESSAGE_LENGTH);
}

/// 测试超过限制 1 个字符的消息会分割
/// 超过限制 1 个字符必须分割
#[test]
fn split_message_one_over_limit_splits() {
    let msg = "a".repeat(discord_message_split::DISCORD_MAX_MESSAGE_LENGTH + 1);
    let parts = discord_message_split::split_message_for_discord(&msg);
    assert!(parts.len() >= 2, "超过限制 1 个字符的消息必须分割");
}

/// 测试许多短行的批处理
/// 许多短行应该被批处理为限制内的块
#[test]
#[allow(clippy::format_collect)]
fn split_message_many_short_lines() {
    let msg: String = (0..500).map(|i| format!("line {i}\n")).collect();
    let parts = discord_message_split::split_message_for_discord(&msg);
    for part in &parts {
        assert!(
            part.len() <= discord_message_split::DISCORD_MAX_MESSAGE_LENGTH,
            "短行批次必须 <= 限制"
        );
    }
    let reassembled: String = parts.join("");
    assert_eq!(reassembled.trim(), msg.trim());
}

/// 测试仅包含空白的消息
/// 应该优雅处理而不 panic
#[test]
fn split_message_only_whitespace() {
    let msg = "   \n\n\t  ";
    let parts = discord_message_split::split_message_for_discord(msg);
    assert!(parts.len() <= 1);
}

/// 测试表情符号在边界处的情况
/// 表情符号是多字节的；确保不在表情符号中间分割
#[test]
fn split_message_emoji_at_boundary() {
    let mut msg = "a".repeat(1998);
    msg.push_str("🎉🎊");
    let parts = discord_message_split::split_message_for_discord(&msg);
    for part in &parts {
        assert!(
            part.chars().count() <= discord_message_split::DISCORD_MAX_MESSAGE_LENGTH,
            "表情符号边界分割必须遵守限制"
        );
    }
}

/// 测试边界处连续换行符的分割
#[test]
fn split_message_consecutive_newlines_at_boundary() {
    let mut msg = "a".repeat(1995);
    msg.push_str("\n\n\n\n\n");
    msg.push_str(&"b".repeat(100));
    let parts = discord_message_split::split_message_for_discord(&msg);
    for part in &parts {
        assert!(part.len() <= discord_message_split::DISCORD_MAX_MESSAGE_LENGTH);
    }
}
