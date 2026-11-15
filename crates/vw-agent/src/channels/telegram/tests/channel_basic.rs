//! Telegram 频道基础功能测试模块
//!
//! 本模块包含 Telegram 频道实现的基础功能测试用例，涵盖以下方面：
//! - 频道名称验证
//! - "正在输入"状态管理
//! - API URL 构造（包括自定义基础 URL）
//! - 工作空间目录配置
//! - 文件下载大小限制验证
//! - 消息更新目标解析
//! - 消息 ID 格式化和确定性验证

use super::*;
use channel_types::Channel;
use std::time::Duration;

/// 测试 Telegram 频道名称是否正确返回 "telegram"
///
/// 验证 `TelegramChannel::name()` 方法返回预期的频道标识符。
#[test]
fn telegram_channel_name() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    assert_eq!(ch.name(), "telegram");
}

/// 测试"正在输入"句柄的初始状态是否为 None
///
/// 验证新创建的 TelegramChannel 实例中，typing_handle 字段的初始值为 None，
/// 确保没有残留的后台任务句柄。
#[test]
fn typing_handle_starts_as_none() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    let guard = ch.typing_handle.lock();
    assert!(guard.is_none());
}

/// 测试 stop_typing 方法是否正确清除句柄
///
/// 测试步骤：
/// 1. 创建一个 TelegramChannel 实例
/// 2. 手动设置一个模拟的"正在输入"任务句柄
/// 3. 调用 stop_typing 方法
/// 4. 验证句柄已被清除为 None
#[tokio::test]
async fn stop_typing_clears_handle() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);

    // 手动设置一个模拟的"正在输入"后台任务
    {
        let mut guard = ch.typing_handle.lock();
        *guard = Some(tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }));
    }

    // 调用 stop_typing 清除句柄
    ch.stop_typing("123").await.unwrap();

    // 验证句柄已被清除
    let guard = ch.typing_handle.lock();
    assert!(guard.is_none());
}

/// 测试 start_typing 方法是否正确替换之前的句柄
///
/// 测试步骤：
/// 1. 创建一个 TelegramChannel 实例
/// 2. 手动设置一个旧的"正在输入"任务句柄
/// 3. 调用 start_typing 方法启动新的"正在输入"状态
/// 4. 验证句柄已被替换为新值（is_some()）
#[tokio::test]
async fn start_typing_replaces_previous_handle() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);

    // 手动设置一个旧的"正在输入"后台任务
    {
        let mut guard = ch.typing_handle.lock();
        *guard = Some(tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }));
    }

    // 启动新的"正在输入"状态，应替换旧句柄
    let _ = ch.start_typing("123").await;

    // 验证句柄已被替换为新值
    let guard = ch.typing_handle.lock();
    assert!(guard.is_some());
}

/// 测试默认 API URL 的构造是否正确
///
/// 验证使用默认 Telegram API 基础 URL 时，api_url 方法能正确构造完整的 API 端点 URL。
/// 格式应为：https://api.telegram.org/bot{token}/{method}
#[test]
fn telegram_api_url() {
    let ch = TelegramChannel::new("123:ABC".into(), vec![], false);
    assert_eq!(ch.api_url("getMe"), "https://api.telegram.org/bot123:ABC/getMe");
}

/// 测试自定义基础 URL 是否正确应用
///
/// 验证通过 with_api_base 方法设置自定义 API 基础 URL 后，
/// api_url 方法能正确构造使用自定义基础 URL 的完整端点。
/// 这支持使用 Telegram API 兼容的第三方服务（如 Bale）。
#[test]
fn telegram_custom_base_url() {
    let ch = TelegramChannel::new("123:ABC".into(), vec![], false)
        .with_api_base("https://tapi.bale.ai".to_string());
    assert_eq!(ch.api_url("getMe"), "https://tapi.bale.ai/bot123:ABC/getMe");
    assert_eq!(ch.api_url("sendMessage"), "https://tapi.bale.ai/bot123:ABC/sendMessage");
}

/// 测试 sendDocument API 端点 URL 的构造
///
/// 验证文档发送 API 的 URL 构造是否正确。
#[test]
fn telegram_api_url_send_document() {
    let ch = TelegramChannel::new("123:ABC".into(), vec![], false);
    assert_eq!(ch.api_url("sendDocument"), "https://api.telegram.org/bot123:ABC/sendDocument");
}

/// 测试 sendPhoto API 端点 URL 的构造
///
/// 验证图片发送 API 的 URL 构造是否正确。
#[test]
fn telegram_api_url_send_photo() {
    let ch = TelegramChannel::new("123:ABC".into(), vec![], false);
    assert_eq!(ch.api_url("sendPhoto"), "https://api.telegram.org/bot123:ABC/sendPhoto");
}

/// 测试 sendVideo API 端点 URL 的构造
///
/// 验证视频发送 API 的 URL 构造是否正确。
#[test]
fn telegram_api_url_send_video() {
    let ch = TelegramChannel::new("123:ABC".into(), vec![], false);
    assert_eq!(ch.api_url("sendVideo"), "https://api.telegram.org/bot123:ABC/sendVideo");
}

/// 测试 sendAudio API 端点 URL 的构造
///
/// 验证音频文件发送 API 的 URL 构造是否正确。
#[test]
fn telegram_api_url_send_audio() {
    let ch = TelegramChannel::new("123:ABC".into(), vec![], false);
    assert_eq!(ch.api_url("sendAudio"), "https://api.telegram.org/bot123:ABC/sendAudio");
}

/// 测试 sendVoice API 端点 URL 的构造
///
/// 验证语音消息发送 API 的 URL 构造是否正确。
#[test]
fn telegram_api_url_send_voice() {
    let ch = TelegramChannel::new("123:ABC".into(), vec![], false);
    assert_eq!(ch.api_url("sendVoice"), "https://api.telegram.org/bot123:ABC/sendVoice");
}

/// 测试 with_workspace_dir 方法是否正确设置工作空间目录
///
/// 验证通过 with_workspace_dir 方法配置的自定义工作空间目录
/// 是否正确存储在实例的 workspace_dir 字段中。
#[test]
fn with_workspace_dir_sets_field() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false)
        .with_workspace_dir(std::path::PathBuf::from("/tmp/test_workspace"));
    assert_eq!(ch.workspace_dir.as_deref(), Some(std::path::Path::new("/tmp/test_workspace")));
}

/// 测试 Telegram 文件下载大小限制常量是否为 20MB
///
/// 验证 TELEGRAM_MAX_FILE_DOWNLOAD_BYTES 常量的值等于 20 * 1024 * 1024 字节，
/// 确保符合 Telegram API 的文件大小限制规范。
#[test]
fn telegram_max_file_download_bytes_is_20mb() {
    assert_eq!(TELEGRAM_MAX_FILE_DOWNLOAD_BYTES, 20 * 1024 * 1024);
}

/// 测试 extract_update_message_target 方法是否正确解析聊天和消息 ID
///
/// 测试步骤：
/// 1. 构造一个包含 update_id、message_id 和 chat.id 的 JSON 对象
/// 2. 调用 extract_update_message_target 方法解析
/// 3. 验证返回的元组包含正确的聊天 ID 和消息 ID
#[test]
fn telegram_extract_update_message_target_parses_ids() {
    let update = serde_json::json!({
        "update_id": 1,
        "message": {
            "message_id": 99,
            "chat": { "id": -100_123_456 }
        }
    });

    let target = TelegramChannel::extract_update_message_target(&update);
    assert_eq!(target, Some(("-100123456".to_string(), 99)));
}

/// 测试消息 ID 格式是否正确包含聊天 ID 和消息 ID
///
/// 验证消息 ID 的格式为 "telegram_{chat_id}_{message_id}"，
/// 确保唯一标识符的构造格式正确。
#[test]
fn telegram_message_id_format_includes_chat_and_message_id() {
    let chat_id = "123456";
    let message_id = 789;
    let expected_id = format!("telegram_{chat_id}_{message_id}");
    assert_eq!(expected_id, "telegram_123456_789");
}

/// 测试消息 ID 生成是否具有确定性
///
/// 验证相同的 chat_id 和 message_id 组合生成的消息 ID 完全一致，
/// 确保消息 ID 生成是确定性的，而非随机的。
#[test]
fn telegram_message_id_is_deterministic() {
    let chat_id = "123456";
    let message_id = 789;
    let id1 = format!("telegram_{chat_id}_{message_id}");
    let id2 = format!("telegram_{chat_id}_{message_id}");
    assert_eq!(id1, id2);
}

/// 测试不同消息 ID 是否生成不同的标识符
///
/// 验证相同的聊天中，不同的消息会产生不同的标识符，
/// 确保消息 ID 的唯一性。
#[test]
fn telegram_message_id_different_message_different_id() {
    let chat_id = "123456";
    let id1 = format!("telegram_{chat_id}_789");
    let id2 = format!("telegram_{chat_id}_790");
    assert_ne!(id1, id2);
}

/// 测试不同聊天 ID 是否生成不同的标识符
///
/// 验证相同的消息 ID 在不同的聊天中会产生不同的完整标识符，
/// 确保全局唯一性。
#[test]
fn telegram_message_id_different_chat_different_id() {
    let message_id = 789;
    let id1 = format!("telegram_123456_{message_id}");
    let id2 = format!("telegram_789012_{message_id}");
    assert_ne!(id1, id2);
}

/// 测试消息 ID 不包含 UUID 随机性特征
///
/// 验证消息 ID 不包含连字符（UUID 的典型特征），
/// 并且始终以 "telegram_" 前缀开头，确保格式可预测。
#[test]
fn telegram_message_id_no_uuid_randomness() {
    let chat_id = "123456";
    let message_id = 789;
    let id = format!("telegram_{chat_id}_{message_id}");
    assert!(!id.contains('-'));
    assert!(id.starts_with("telegram_"));
}

/// 测试消息 ID 是否正确处理零值消息 ID
///
/// 验证当消息 ID 为 0 时，标识符仍能正确生成，
/// 边界条件测试确保所有有效消息 ID 都能正确处理。
#[test]
fn telegram_message_id_handles_zero_message_id() {
    let chat_id = "123456";
    let message_id = 0;
    let id = format!("telegram_{chat_id}_{message_id}");
    assert_eq!(id, "telegram_123456_0");
}
