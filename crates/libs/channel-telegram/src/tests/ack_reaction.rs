//! Telegram 确认表情功能测试模块
//!
//! 本模块提供对 Telegram 确认表情相关功能的单元测试，包括：
//! - 随机确认表情生成器的正确性验证
//! - 确认表情请求构建器的格式验证
//!
//! 这些测试确保 Telegram 频道能够使用预定义的表情池来回应用户消息，
//! 提供视觉反馈表示代理已接收并正在处理消息。

use super::*;

/// 测试随机确认表情是否始终来自预定义的表情池
///
/// 该测试通过多次（128次）调用 `random_telegram_ack_reaction()` 函数，
/// 验证返回的每个表情都存在于 `TELEGRAM_ACK_REACTIONS` 常量池中。
/// 这确保了随机表情生成器不会返回无效或意外的表情字符。
///
/// # 测试逻辑
/// - 循环 128 次进行采样，以获得足够的统计置信度
/// - 每次迭代获取一个随机表情并断言其在池中
#[test]
fn random_telegram_ack_reaction_is_from_pool() {
    for _ in 0..128 {
        let emoji = random_telegram_ack_reaction();
        assert!(TELEGRAM_ACK_REACTIONS.contains(&emoji));
    }
}

/// 测试确认表情请求的 JSON 结构是否符合 Telegram API 规范
///
/// 该测试验证 `build_telegram_ack_reaction_request()` 函数生成的
/// JSON 请求体包含正确的字段和值：
/// - `chat_id`: Telegram 聊天/频道标识符
/// - `message_id`: 要添加表情的消息 ID
/// - `reaction`: 表情类型和具体表情的数组
///
/// # 测试数据
/// - 聊天 ID: "-100200300"（模拟超级群组/频道 ID）
/// - 消息 ID: 42
/// - 表情: "⚡️"（闪电表情）
///
/// # 验证点
/// - chat_id 字段正确传递
/// - message_id 字段正确传递
/// - reaction 数组的结构正确（包含 type 和 emoji 字段）
/// - 表情类型为 "emoji"
/// - 表情值正确设置
#[test]
fn telegram_ack_reaction_request_shape() {
    let body = build_telegram_ack_reaction_request("-100200300", 42, "⚡️");
    assert_eq!(body["chat_id"], "-100200300");
    assert_eq!(body["message_id"], 42);
    assert_eq!(body["reaction"][0]["type"], "emoji");
    assert_eq!(body["reaction"][0]["emoji"], "⚡️");
}
