//! 维护工具循环向前端/调用方发送的流式输出。
//!
//! 这里的函数只处理展示层进度与最终文本分片，不参与模型调用或工具执行逻辑。

use anyhow::Result;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;


use super::constants::STREAM_CHUNK_MIN_CHARS;
use super::errors::ToolLoopCancelled;



/// 发送“思考中”草稿进度。
///
/// 参数 `on_delta` 为空时静默跳过；`iteration` 用于区分首轮与后续工具循环轮次。
pub async fn update_thinking_progress(
    wire: &ProgressWire,on_delta: Option<&Sender<String>>, iteration: usize) {
    let Some(tx) = on_delta else {
        return;
    };

    let phase = if iteration == 0 {
        "💡 思考中\n".to_string()
    } else {
        format!("💡 思考中 (第{}轮)\n", iteration + 1)
    };
    let _ = tx.send(format!("{}{phase}", wire.progress)).await;
}

/// 发送模型返回工具调用后的进度提示。
///
/// 参数 `tool_call_count` 为 0 时不发送内容；`llm_secs` 展示本轮 LLM 耗时。
pub async fn update_tool_call_progress(
    wire: &ProgressWire,
    on_delta: Option<&Sender<String>>,
    tool_call_count: usize,
    llm_secs: u64,
) {
    let Some(tx) = on_delta else {
        return;
    };

    if tool_call_count == 0 {
        return;
    }

    let _ = tx
        .send(format!(
            "{}\u{1f4ac} Got {tool_call_count} tool call(s) ({llm_secs}s)\n", wire.progress
        ))
        .await;
}

/// 发送因疑似工具调用解析失败而重试的进度提示。
///
/// 没有 delta 通道时直接返回，发送失败也不影响主流程。
pub async fn send_retry_progress(
    wire: &ProgressWire,on_delta: Option<&Sender<String>>) {
    let Some(tx) = on_delta else {
        return;
    };

    let _ = tx
        .send(format!(
            "{}\u{21bb} Retrying: response implied action without a verifiable tool call\n", wire.progress
        ))
        .await;
}

/// 将最终回答按自然空白边界分片发送。
///
/// 参数 `display_text` 是要展示的文本；`on_delta` 缺失时直接完成；取消令牌触发时
/// 返回 `ToolLoopCancelled`，发送端关闭则视为调用方不再需要输出。
pub async fn stream_final_response(
    wire: &ProgressWire,
    display_text: &str,
    on_delta: Option<&Sender<String>>,
    cancellation_token: Option<&CancellationToken>,
) -> Result<()> {
    let Some(tx) = on_delta else {
        return Ok(());
    };

    let _ = tx.send(wire.clear.to_string()).await;

    let mut chunk = String::new();
    for word in display_text.split_inclusive(char::is_whitespace) {
        if cancellation_token.is_some_and(CancellationToken::is_cancelled) {
            return Err(ToolLoopCancelled.into());
        }
        chunk.push_str(word);
        // 按最小字符数批量发送，减少 UI 更新频率，同时保留接近流式的观感。
        if chunk.len() >= STREAM_CHUNK_MIN_CHARS
            && tx.send(std::mem::take(&mut chunk)).await.is_err()
        {
            return Ok(());
        }
    }

    if !chunk.is_empty() {
        let _ = tx.send(chunk).await;
    }

    Ok(())
}

/// 已有通道草稿协议的常量引用。
pub struct ProgressWire { pub clear: &'static str, pub progress: &'static str }
