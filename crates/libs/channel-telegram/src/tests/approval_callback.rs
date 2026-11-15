//! Telegram 通道审批回调功能测试模块
//!
//! 本模块包含对 Telegram 通道中审批回调（approval callback）相关功能的单元测试。
//! 主要测试内容包括：
//! - 审批回调命令的解析和映射（批准/拒绝）
//! - 输入数据的规范化处理（修剪空白、拒绝空 ID）
//! - 完整的回调查询解析和运行时命令消息构建
//!
//! 这些测试确保用户通过 Telegram 内联按钮提交的审批操作能够正确转换为
//! 代理运行时可处理的命令格式。

use super::*;

/// 测试审批回调命令的正确映射（批准和拒绝）
///
/// # 测试内容
///
/// 1. 测试格式为 `zcapr:yes:<id>` 的回调数据应映射为 `/approve-allow <id>` 命令
/// 2. 测试格式为 `zcapr:no:<id>` 的回调数据应映射为 `/approve-deny <id>` 命令
/// 3. 测试无效前缀的回调数据应返回 `None`
///
/// # 示例
///
/// ```
/// "zcapr:yes:apr-1234" -> "/approve-allow apr-1234"
/// "zcapr:no:apr-5678"  -> "/approve-deny apr-5678"
/// "noop:data"          -> None
/// ```
#[test]
fn parse_approval_callback_command_maps_approve_and_deny() {
    // 测试批准操作：yes 关键字应映射为 approve-allow 命令
    assert_eq!(
        TelegramChannel::parse_approval_callback_command("zcapr:yes:apr-1234"),
        Some("/approve-allow apr-1234".to_string())
    );
    // 测试拒绝操作：no 关键字应映射为 approve-deny 命令
    assert_eq!(
        TelegramChannel::parse_approval_callback_command("zcapr:no:apr-5678"),
        Some("/approve-deny apr-5678".to_string())
    );
    // 测试无效前缀：非 zcapr 开头的数据应返回 None
    assert_eq!(TelegramChannel::parse_approval_callback_command("noop:data"), None);
}

/// 测试审批回调命令解析的输入规范化处理
///
/// # 测试内容
///
/// 1. 测试 ID 字段中的前后空白字符（包括空格和制表符）应被正确修剪
/// 2. 测试修剪后为空的 ID 应返回 `None`
/// 3. 测试缺少 ID 字段的情况应返回 `None`
///
/// # 边界情况
///
/// - 包含前后空格的 ID（如 `"   apr-1234   "`）
/// - 包含制表符的 ID（如 `"\tapr-5678  "`）
/// - 仅有空白字符的 ID（如 `"   "`）
/// - 完全缺失的 ID（如 `""`）
#[test]
fn parse_approval_callback_command_trims_and_rejects_empty_ids() {
    // 测试 ID 包含前后空格：应修剪为 "apr-1234"
    assert_eq!(
        TelegramChannel::parse_approval_callback_command("zcapr:yes:   apr-1234   "),
        Some("/approve-allow apr-1234".to_string())
    );
    // 测试 ID 包含制表符和空格：应修剪为 "apr-5678"
    assert_eq!(
        TelegramChannel::parse_approval_callback_command("zcapr:no:\tapr-5678  "),
        Some("/approve-deny apr-5678".to_string())
    );
    // 测试 ID 仅为空白字符：应返回 None
    assert_eq!(TelegramChannel::parse_approval_callback_command("zcapr:yes:   "), None);
    // 测试 ID 为空字符串：应返回 None
    assert_eq!(TelegramChannel::parse_approval_callback_command("zcapr:no:"), None);
}

/// 测试完整的审批回调查询解析和运行时命令消息构建
///
/// # 测试内容
///
/// 本测试验证从 Telegram 回调查询 JSON 对象到运行时命令消息的完整转换流程：
///
/// 1. 解析 Telegram API 格式的回调查询 JSON 数据
/// 2. 提取发送者信息（username）
/// 3. 构建回复目标（chat_id:message_thread_id 格式）
/// 4. 转换回调数据为运行时命令（`/approve-allow` 或 `/approve-deny`）
/// 5. 生成唯一的消息标识符（格式：`telegram_cb_<chat_id>_<message_id>_<timestamp>`）
///
/// # 测试数据结构
///
/// - `update_id`: Telegram 更新 ID
/// - `callback_query.id`: 回调查询标识
/// - `callback_query.data`: 编码的审批数据（格式：`zcapr:<yes|no>:<approval_id>`）
/// - `callback_query.from`: 发送者信息（id, username）
/// - `callback_query.message`: 关联消息信息（message_id, chat, message_thread_id）
#[tokio::test]
async fn try_parse_approval_callback_query_builds_runtime_command_message() {
    // 创建 TelegramChannel 实例，使用测试 token 和允许所有用户的配置
    let ch = TelegramChannel::new("token".into(), vec!["*".into()], false);

    // 构造模拟的 Telegram 回调查询更新对象
    let update = serde_json::json!({
        "update_id": 7,
        "callback_query": {
            "id": "cb-1",                                    // 回调查询 ID
            "data": "zcapr:yes:apr-deadbeef",               // 编码的审批数据：批准操作
            "from": {
                "id": 555,                                  // 发送者 Telegram 用户 ID
                "username": "alice"                         // 发送者用户名
            },
            "message": {
                "message_id": 44,                           // 关联消息的 ID
                "chat": { "id": -100_200_300 },            // 群组/频道 ID（负数表示群组）
                "message_thread_id": 789                    // 话题/线程 ID
            }
        }
    });

    // 尝试解析回调查询，预期应成功返回消息对象
    let msg = ch.try_parse_approval_callback_query(&update).expect("callback query should parse");

    // 验证发送者用户名被正确提取
    assert_eq!(msg.sender, "alice");
    // 验证回复目标格式正确（chat_id:message_thread_id）
    assert_eq!(msg.reply_target, "-100200300:789");
    // 验证审批命令被正确转换
    assert_eq!(msg.content, "/approve-allow apr-deadbeef");
    // 验证消息 ID 格式正确（包含前缀、chat_id 和 message_id）
    assert!(msg.id.starts_with("telegram_cb_-100200300_44_"));
}
