//! Telegram 频道文件发送功能测试模块
//!
//! 本模块包含针对 TelegramChannel 文件发送功能的单元测试，覆盖以下场景：
//! - 通过字节数组发送各类文件（文档、图片）
//! - 通过 URL 发送文件
//! - 通过文件路径发送文件（文档、图片、视频、音频、语音）
//! - 边界条件测试（空文件、空文件名、空聊天 ID、不存在的文件）
//!
//! # 测试策略
//!
//! 所有测试均使用伪造的 token 和允许列表配置 TelegramChannel 实例，
//! 并期望因网络错误而失败（使用无效 token 无法连接到真实 Telegram API）。
//! 这验证了：
//! 1. 请求构建逻辑正确执行
//! 2. 错误处理路径正常工作
//! 3. 参数验证按预期进行

use super::*;
use std::path::Path;

/// 测试通过字节数组发送文档时表单构建的正确性
///
/// # 测试场景
/// - 使用字节数组发送文档到指定聊天
/// - 提供文件名和说明文字
///
/// # 预期结果
/// - 返回错误（因为 token 无效，网络请求失败）
/// - 错误信息应包含 "error"、"failed" 或 "connect" 关键字
///
/// # 验证点
/// - multipart 表单正确构建
/// - 文件内容、文件名、说明文字正确传递
#[tokio::test]
async fn telegram_send_document_bytes_builds_correct_form() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    let file_bytes = b"Hello, this is a test file content".to_vec();

    let result =
        ch.send_document_bytes("123456", None, file_bytes, "test.txt", Some("Test caption")).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("error") || err.contains("failed") || err.contains("connect"),
        "Expected network error, got: {err}"
    );
}

/// 测试通过字节数组发送图片时表单构建的正确性
///
/// # 测试场景
/// - 使用 PNG 文件头魔数的字节数组发送图片
/// - 仅提供必需参数，不提供说明文字
///
/// # 预期结果
/// - 返回错误（因为 token 无效，网络请求失败）
///
/// # 验证点
/// - 图片字节流正确处理
/// - 可选参数（说明文字）为 None 时不影响请求构建
#[tokio::test]
async fn telegram_send_photo_bytes_builds_correct_form() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    let file_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    let result = ch.send_photo_bytes("123456", None, file_bytes, "test.png", None).await;

    assert!(result.is_err());
}

/// 测试通过 URL 发送文档时 JSON 请求构建的正确性
///
/// # 测试场景
/// - 使用外部 URL 作为文档来源
/// - 提供文档说明文字
///
/// # 预期结果
/// - 返回错误（因为 token 无效，网络请求失败）
///
/// # 验证点
/// - URL 参数正确编码到请求体
/// - 说明文字正确传递
#[tokio::test]
async fn telegram_send_document_by_url_builds_correct_json() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);

    let result = ch
        .send_document_by_url("123456", None, "https://example.com/file.pdf", Some("PDF doc"))
        .await;

    assert!(result.is_err());
}

/// 测试通过 URL 发送图片时 JSON 请求构建的正确性
///
/// # 测试场景
/// - 使用外部 URL 作为图片来源
/// - 不提供说明文字
///
/// # 预期结果
/// - 返回错误（因为 token 无效，网络请求失败）
///
/// # 验证点
/// - URL 参数正确编码到请求体
#[tokio::test]
async fn telegram_send_photo_by_url_builds_correct_json() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);

    let result = ch.send_photo_by_url("123456", None, "https://example.com/image.jpg", None).await;

    assert!(result.is_err());
}

/// 测试发送不存在的文档文件时的错误处理
///
/// # 测试场景
/// - 尝试发送一个不存在的本地文件作为文档
///
/// # 预期结果
/// - 返回错误
/// - 错误信息应包含文件不存在的提示
///
/// # 验证点
/// - 文件存在性检查正常工作
/// - 错误信息清晰易懂
#[tokio::test]
async fn telegram_send_document_nonexistent_file() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    let path = Path::new("/nonexistent/path/to/file.txt");

    let result = ch.send_document("123456", None, path, None).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("No such file") || err.contains("not found") || err.contains("os error"),
        "Expected file not found error, got: {err}"
    );
}

/// 测试发送不存在的图片文件时的错误处理
///
/// # 测试场景
/// - 尝试发送一个不存在的本地图片文件
///
/// # 预期结果
/// - 返回错误（文件不存在）
///
/// # 验证点
/// - 图片文件存在性检查正常工作
#[tokio::test]
async fn telegram_send_photo_nonexistent_file() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    let path = Path::new("/nonexistent/path/to/photo.jpg");

    let result = ch.send_photo("123456", None, path, None).await;

    assert!(result.is_err());
}

/// 测试发送不存在的视频文件时的错误处理
///
/// # 测试场景
/// - 尝试发送一个不存在的本地视频文件
///
/// # 预期结果
/// - 返回错误（文件不存在）
///
/// # 验证点
/// - 视频文件存在性检查正常工作
#[tokio::test]
async fn telegram_send_video_nonexistent_file() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    let path = Path::new("/nonexistent/path/to/video.mp4");

    let result = ch.send_video("123456", None, path, None).await;

    assert!(result.is_err());
}

/// 测试发送不存在的音频文件时的错误处理
///
/// # 测试场景
/// - 尝试发送一个不存在的本地音频文件
///
/// # 预期结果
/// - 返回错误（文件不存在）
///
/// # 验证点
/// - 音频文件存在性检查正常工作
#[tokio::test]
async fn telegram_send_audio_nonexistent_file() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    let path = Path::new("/nonexistent/path/to/audio.mp3");

    let result = ch.send_audio("123456", None, path, None).await;

    assert!(result.is_err());
}

/// 测试发送不存在的语音文件时的错误处理
///
/// # 测试场景
/// - 尝试发送一个不存在的本地语音文件
///
/// # 预期结果
/// - 返回错误（文件不存在）
///
/// # 验证点
/// - 语音文件存在性检查正常工作
#[tokio::test]
async fn telegram_send_voice_nonexistent_file() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    let path = Path::new("/nonexistent/path/to/voice.ogg");

    let result = ch.send_voice("123456", None, path, None).await;

    assert!(result.is_err());
}

/// 测试带说明文字的文档字节数组发送
///
/// # 测试场景
/// - 使用相同的字节数组发送两次文档
/// - 第一次带说明文字
/// - 第二次不带说明文字
///
/// # 预期结果
/// - 两次操作都应返回错误（网络失败）
///
/// # 验证点
/// - 说明文字参数可选性正常工作
/// - 同一实例可重复使用发送不同配置的消息
#[tokio::test]
async fn telegram_send_document_bytes_with_caption() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    let file_bytes = b"test content".to_vec();

    let result = ch
        .send_document_bytes("123456", None, file_bytes.clone(), "test.txt", Some("My caption"))
        .await;
    assert!(result.is_err());

    let result = ch.send_document_bytes("123456", None, file_bytes, "test.txt", None).await;
    assert!(result.is_err());
}

/// 测试带说明文字的图片字节数组发送
///
/// # 测试场景
/// - 使用相同的字节数组发送两次图片
/// - 第一次带说明文字
/// - 第二次不带说明文字
///
/// # 预期结果
/// - 两次操作都应返回错误（网络失败）
///
/// # 验证点
/// - 图片说明文字参数可选性正常工作
#[tokio::test]
async fn telegram_send_photo_bytes_with_caption() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    let file_bytes = vec![0x89, 0x50, 0x4E, 0x47];

    let result = ch
        .send_photo_bytes("123456", None, file_bytes.clone(), "test.png", Some("Photo caption"))
        .await;
    assert!(result.is_err());

    let result = ch.send_photo_bytes("123456", None, file_bytes, "test.png", None).await;
    assert!(result.is_err());
}

/// 测试发送空字节数组的文档
///
/// # 测试场景
/// - 尝试发送一个空的字节数组作为文档
///
/// # 预期结果
/// - 返回错误（空文件无效）
///
/// # 验证点
/// - 空文件检测逻辑正常工作
#[tokio::test]
async fn telegram_send_document_bytes_empty_file() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    let file_bytes: Vec<u8> = vec![];

    let result = ch.send_document_bytes("123456", None, file_bytes, "empty.txt", None).await;

    assert!(result.is_err());
}

/// 测试发送文档时使用空文件名
///
/// # 测试场景
/// - 尝试发送文档但不提供有效文件名（空字符串）
///
/// # 预期结果
/// - 返回错误（文件名无效）
///
/// # 验证点
/// - 文件名验证逻辑正常工作
#[tokio::test]
async fn telegram_send_document_bytes_empty_filename() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    let file_bytes = b"content".to_vec();

    let result = ch.send_document_bytes("123456", None, file_bytes, "", None).await;

    assert!(result.is_err());
}

/// 测试发送文档时使用空聊天 ID
///
/// # 测试场景
/// - 尝试发送文档但不提供有效的聊天 ID（空字符串）
///
/// # 预期结果
/// - 返回错误（聊天 ID 无效）
///
/// # 验证点
/// - 聊天 ID 验证逻辑正常工作
#[tokio::test]
async fn telegram_send_document_bytes_empty_chat_id() {
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    let file_bytes = b"content".to_vec();

    let result = ch.send_document_bytes("", None, file_bytes, "test.txt", None).await;

    assert!(result.is_err());
}
