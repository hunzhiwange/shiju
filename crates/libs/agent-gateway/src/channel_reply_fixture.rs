//! Shared legacy channel reply fixture; not a production dispatch path.
use gateway_types::AppState;
use channel_types::{Channel, ChannelMessage, SendMessage};
use memory_types::memory::MemoryCategory;
use tool_types::Tool;
use crate::chat::{log_channel_message, sanitize_gateway_response};
use std::{future::Future, pin::Pin};
pub type GatewayChatFuture<'a> = Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'a>>;
pub type GatewayChatRunner<E, S, H> = for<'a> fn(&'a AppState<E, S, H>, &'a str, &'a str) -> GatewayChatFuture<'a>;
pub async fn process_channel_messages_with_chat<E, S, H, C, K>(
    state: &AppState<E, S, H>,
    channel_name: &str,
    channel: &C,
    messages: &[ChannelMessage],
    memory_key: K,
    llm_error_label: &str,
    include_thread: bool,
    run_chat: GatewayChatRunner<E, S, H>,
) where
    C: Channel + ?Sized,
    K: Fn(&ChannelMessage) -> String,
{
    for msg in messages {
        log_channel_message(channel_name, &msg.sender, &msg.content);

        let key = memory_key(msg);
        if state.auto_save {
            let _ = state.mem.store(&key, &msg.content, MemoryCategory::Conversation, None).await;
        }

        match run_chat(state, &msg.content, &key).await {
            Ok(response) => {
                send_successful_channel_reply(
                    channel_name,
                    channel,
                    msg,
                    &response,
                    state.tools_registry_exec.as_ref(),
                    include_thread,
                )
                .await;
            }
            Err(e) => {
                tracing::error!("LLM error for {llm_error_label}: {e:#}");
                let mut reply = SendMessage::new(
                    "Sorry, I couldn't process your message right now.",
                    &msg.reply_target,
                );
                if include_thread {
                    reply = reply.in_thread(msg.thread_ts.clone());
                }
                let _ = channel.send(&reply).await;
            }
        }
    }
}

pub async fn send_successful_channel_reply<C: Channel + ?Sized>(
    channel_name: &str,
    channel: &C,
    msg: &ChannelMessage,
    response: &str,
    tools: &[Box<dyn Tool>],
    include_thread: bool,
) {
    let safe_response = sanitize_gateway_response(response, tools);
    let mut reply = SendMessage::new(safe_response, &msg.reply_target);
    if include_thread {
        reply = reply.in_thread(msg.thread_ts.clone());
    }
    if let Err(e) = channel.send(&reply).await {
        tracing::error!("Failed to send {channel_name} reply: {e}");
    }
}

use parking_lot::Mutex;
#[derive(Default)]
pub struct RecordingChannel {
    pub sent: Mutex<Vec<SendMessage>>,
    pub fail_send: bool,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for RecordingChannel {
    fn name(&self) -> &str {
        "recording"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.sent.lock().push(message.clone());
        if self.fail_send {
            anyhow::bail!("send failed");
        }
        Ok(())
    }

    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        Ok(())
    }
}

// 构造统一渠道消息，便于比较成功回复和错误降级是否保留线程上下文。
pub fn channel_message() -> ChannelMessage {
    ChannelMessage {
        instance_id: "default".to_string(),
        id: "message-1".to_string(),
        sender: "sender-1".to_string(),
        reply_target: "target-1".to_string(),
        content: "hello".to_string(),
        channel: "recording".to_string(),
        timestamp: 123,
        thread_ts: Some("thread-1".to_string()),
    }
}
